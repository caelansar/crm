#![feature(impl_trait_in_assoc_type)]

mod abi;
mod config;
pub mod pb;

use std::pin::Pin;
use std::{ops::Deref, sync::Arc};

pub use abi::{ClickHouseRepo, Repo, UserRow};
pub use config::AppConfig;
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

    impl<R> UserStatsService<R> {
        pub async fn new_for_test(repo: R, config: AppConfig) -> Self {
            let inner = UserStatsServiceInner { repo, config };
            Self {
                inner: Arc::new(inner),
            }
        }
    }
}

impl<R> Deref for UserStatsService<R> {
    type Target = UserStatsServiceInner<R>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
