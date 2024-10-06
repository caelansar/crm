mod abi;
mod config;

pub mod pb;

pub use config::AppConfig;

use anyhow::{Context, Result};
use crm_core::SendTrace;
use crm_metadata::pb::metadata_client::MetadataClient;
use crm_notification::pb::notification_client::NotificationClient;
use mobc::Manager;
use mobc::Pool;
use pb::{
    crm_server::{Crm, CrmServer},
    RecallRequest, RecallResponse, RemindRequest, RemindResponse, WelcomeRequest, WelcomeResponse,
};
use std::time::Duration;
use tonic::transport::Endpoint;
use tonic::{
    async_trait, service::interceptor::InterceptedService, transport::Channel, Request, Response,
    Status,
};
use tracing::instrument;
use user_stat::pb::user_stats_client::UserStatsClient;

/// A service for managing CRM operations.
///
/// This service interacts with various gRPC clients to perform operations such as sending
/// welcome emails, recalling users, and reminding users. It uses a connection pool for each
/// gRPC client to manage connections efficiently.
pub struct CrmService {
    config: AppConfig,
    user_stats_pool:
        Pool<GrpcClientManager<UserStatsClient<InterceptedService<Channel, SendTrace>>>>,
    notification_pool:
        Pool<GrpcClientManager<NotificationClient<InterceptedService<Channel, SendTrace>>>>,
    metadata_pool: Pool<GrpcClientManager<MetadataClient<InterceptedService<Channel, SendTrace>>>>,
}

#[async_trait]
impl Crm for CrmService {
    #[instrument(name = "welcome_handler", skip_all)]
    async fn welcome(
        &self,
        request: Request<WelcomeRequest>,
    ) -> Result<Response<WelcomeResponse>, Status> {
        self.welcome(request.into_inner()).await
    }

    #[instrument(name = "recall_handler", skip_all)]
    async fn recall(
        &self,
        _request: Request<RecallRequest>,
    ) -> Result<Response<RecallResponse>, Status> {
        todo!()
    }

    #[instrument(name = "remind_handler", skip_all)]
    async fn remind(
        &self,
        _request: Request<RemindRequest>,
    ) -> Result<Response<RemindResponse>, Status> {
        todo!()
    }
}

impl CrmService {
    pub async fn try_new(config: AppConfig) -> Result<Self> {
        let user_stats_pool = create_client_pool(&config.server.user_stats).await?;
        let notification_pool = create_client_pool(&config.server.notification).await?;
        let metadata_pool = create_client_pool(&config.server.metadata).await?;

        Ok(Self {
            config,
            user_stats_pool,
            notification_pool,
            metadata_pool,
        })
    }

    pub fn into_server(self) -> Result<CrmServer<CrmService>> {
        Ok(CrmServer::new(self))
    }
}

async fn create_client_pool<T>(addr: &str) -> Result<Pool<GrpcClientManager<T>>>
where
    T: FromInterceptedService<InterceptedService<Channel, SendTrace>> + Send + 'static,
    T: Sync,
{
    let manager = GrpcClientManager {
        addr: addr.to_string(),
        _phantom: std::marker::PhantomData,
    };

    let pool = Pool::builder()
        .max_open(15)
        .max_idle(5)
        .get_timeout(Some(Duration::from_secs(5)))
        .build(manager);

    Ok(pool)
}

/// A custom mobc manager for gRPC clients
struct GrpcClientManager<T> {
    addr: String,
    _phantom: std::marker::PhantomData<T>,
}

unsafe impl<T> Sync for GrpcClientManager<T> where T: Sync {}

#[async_trait]
impl<T> Manager for GrpcClientManager<T>
where
    T: FromInterceptedService<InterceptedService<Channel, SendTrace>> + Send + 'static,
    T: Sync,
{
    type Connection = T;
    type Error = anyhow::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let channel = Endpoint::from_shared(self.addr.clone())?
            .connect()
            .await
            .context(format!(
                "Failed to connect to gRPC server with address: {}",
                self.addr
            ))?;

        Ok(T::from_intercepted_service(InterceptedService::new(
            channel, SendTrace,
        )))
    }

    async fn check(&self, conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        // Implement a health check if possible, or just return the connection
        Ok(conn)
    }
}

trait FromInterceptedService<T> {
    fn from_intercepted_service(value: T) -> Self;
}

impl FromInterceptedService<InterceptedService<Channel, SendTrace>>
    for UserStatsClient<InterceptedService<Channel, SendTrace>>
{
    fn from_intercepted_service(value: InterceptedService<Channel, SendTrace>) -> Self {
        UserStatsClient::new(value)
    }
}

impl FromInterceptedService<InterceptedService<Channel, SendTrace>>
    for NotificationClient<InterceptedService<Channel, SendTrace>>
{
    fn from_intercepted_service(value: InterceptedService<Channel, SendTrace>) -> Self {
        NotificationClient::new(value)
    }
}

impl FromInterceptedService<InterceptedService<Channel, SendTrace>>
    for MetadataClient<InterceptedService<Channel, SendTrace>>
{
    fn from_intercepted_service(value: InterceptedService<Channel, SendTrace>) -> Self {
        MetadataClient::new(value)
    }
}
