#![feature(impl_trait_in_assoc_type)]

mod abi;
mod config;
pub mod pb;

use std::{mem, ops::Deref, sync::Arc};

use abi::UserRow;
use clickhouse::{test, Client};
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
pub struct UserStatsService {
    inner: Arc<UserStatsServiceInner>,
}

#[allow(unused)]
pub struct UserStatsServiceInner {
    /// Clickhouse client
    client: Client,
    /// App base config
    config: AppConfig,
}

#[async_trait]
impl UserStats for UserStatsService {
    type QueryStream = impl Stream<Item = Result<User, Status>> + Send + 'static;
    type RawQueryStream = impl Stream<Item = Result<User, Status>> + Send + 'static;

    #[instrument(name = "query-handler", skip_all)]
    async fn query(
        &self,
        request: tonic::Request<pb::QueryRequest>,
    ) -> ServiceResult<Self::QueryStream> {
        self.query(request.into_inner()).await
    }

    async fn raw_query(
        &self,
        request: tonic::Request<pb::RawQueryRequest>,
    ) -> ServiceResult<Self::RawQueryStream> {
        self.raw_query(request.into_inner().query.clone()).await
    }
}

impl UserStatsService {
    pub async fn new(config: AppConfig) -> Self {
        let client = Client::default()
            .with_url(&config.server.db_url)
            .with_database(&config.server.db_name);
        let inner = UserStatsServiceInner { client, config };
        Self {
            inner: Arc::new(inner),
        }
    }

    #[cfg(feature = "test-util")]
    pub async fn new_for_test(config: AppConfig) -> Self {
        let mock = test::Mock::new();

        let client = Client::default().with_url(mock.url());

        let list = vec![
            UserRow {
                name: "test1".to_string(),
                email: "test1@example.com".to_string(),
            },
            UserRow {
                name: "test2".to_string(),
                email: "test2@example.com".to_string(),
            },
        ];

        mock.add(test::handlers::provide(list));

        // Forget the mock instance to avoid it being dropped
        mem::forget(mock);

        let inner = UserStatsServiceInner { client, config };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn into_server(self) -> UserStatsServer<Self> {
        UserStatsServer::new(self)
    }
}

impl Deref for UserStatsService {
    type Target = UserStatsServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
