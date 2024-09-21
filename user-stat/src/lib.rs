#![feature(impl_trait_in_assoc_type)]

mod abi;
mod config;
pub mod pb;

use std::pin::Pin;
use std::{ops::Deref, sync::Arc};

pub use abi::{ClickHouseRepo, PostgresRepo, Repo, UserRow};
pub use config::{AppConfig, DBType};
use futures::Stream;
use pb::{
    user_stats_server::{UserStats, UserStatsServer},
    User,
};
use tonic::{async_trait, Response, Status};
use tracing::instrument;

type ServiceResult<T> = Result<Response<T>, Status>;

#[derive(Clone)]
pub struct UserStatsService<R> {
    inner: Arc<UserStatsServiceInner<R>>,
}

#[allow(unused)]
pub struct UserStatsServiceInner<R> {
    repo: R,
    /// App base config
    config: AppConfig,
}

impl<R: Repo> UserStatsService<R> {
    pub fn query(
        &self,
        request: tonic::Request<pb::QueryRequest>,
    ) -> impl std::future::Future<
        Output = ServiceResult<
            impl Stream<Item = Result<User, Status>> + Send + 'static + use<'_, R>, // https://blog.rust-lang.org/2024/09/05/impl-trait-capture-rules.html
        >,
    > {
        let stream = self.repo.query(request.into_inner());

        async move {
            Ok(Response::new(
                stream.await.map_err(|e| Status::internal(e.to_string()))?,
            ))
        }
    }
}

#[async_trait]
impl<R: Repo> UserStats for UserStatsService<R> {
    type QueryStream = Pin<Box<dyn Stream<Item = Result<User, Status>> + Send>>;
    type RawQueryStream = Pin<Box<dyn Stream<Item = Result<User, Status>> + Send>>;

    #[instrument(name = "query-handler", skip_all)]
    async fn query(
        &self,
        request: tonic::Request<pb::QueryRequest>,
    ) -> ServiceResult<Self::QueryStream> {
        let stream = self
            .repo
            .query(request.into_inner())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Box::pin(stream)))
    }

    #[instrument(name = "raw-query-handler", skip_all)]
    async fn raw_query(
        &self,
        request: tonic::Request<pb::RawQueryRequest>,
    ) -> ServiceResult<Self::RawQueryStream> {
        let stream = self
            .repo
            .raw_query(request.into_inner().query.clone())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Box::pin(stream)))
    }
}

impl<R> UserStatsService<R> {
    pub async fn new(repo: R, config: AppConfig) -> Self {
        // let client = Client::default()
        //     .with_url(&config.server.db_url)
        //     .with_database(&config.server.db_name);

        let inner = UserStatsServiceInner { repo, config };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn into_server(self) -> UserStatsServer<Self> {
        UserStatsServer::new(self)
    }
}

#[cfg(feature = "test-util")]
pub mod tests {
    use super::*;
    use crate::UserStatsService;

    use std::{env, path::Path, sync::Arc};

    use sqlx::{Executor, PgPool};
    use sqlx_db_tester::TestPg;

    impl<R> UserStatsService<R> {
        pub fn new_for_test(repo: R, config: AppConfig) -> Self {
            let inner = UserStatsServiceInner { repo, config };
            Self {
                inner: Arc::new(inner),
            }
        }
    }

    /// Get a test postgres pool
    pub async fn get_test_pool(url: Option<&str>) -> (TestPg, PgPool) {
        let url = match url {
            Some(url) => url.to_string(),
            None => "postgres://postgres:postgres@localhost:5432".to_string(),
        };
        let p = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("migrations");
        let tdb = TestPg::new(url, p);
        let pool = tdb.get_pool().await;

        // run prepared sql to insert test dat
        let sql = include_str!("../assets/postgres.sql").split(';');
        let mut ts = pool.begin().await.expect("begin transaction failed");
        for s in sql {
            if s.trim().is_empty() {
                continue;
            }
            ts.execute(s).await.expect("execute sql failed");
        }
        ts.commit().await.expect("commit transaction failed");

        (tdb, pool)
    }
}

impl<R> Deref for UserStatsService<R> {
    type Target = UserStatsServiceInner<R>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
