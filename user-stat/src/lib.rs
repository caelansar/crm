#![feature(impl_trait_in_assoc_type)]

mod abi;
mod config;
mod pb;

use std::{ops::Deref, sync::Arc};

use clickhouse::Client;
pub use config::AppConfig;
use futures::Stream;
use pb::{
    user_stats_server::{UserStats, UserStatsServer},
    User,
};
use tonic::{async_trait, Response, Status};

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

    async fn query(
        &self,
        _request: tonic::Request<pb::QueryRequest>,
    ) -> ServiceResult<Self::QueryStream> {
        self.inner.raw_query("".to_string()).await
    }

    async fn raw_query(
        &self,
        request: tonic::Request<pb::RawQueryRequest>,
    ) -> ServiceResult<Self::RawQueryStream> {
        self.inner
            .raw_query(request.into_inner().query.clone())
            .await
    }
}

impl UserStatsService {
    pub async fn new(config: AppConfig) -> Self {
        let client = Client::default().with_url(&config.server.db_url);
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
