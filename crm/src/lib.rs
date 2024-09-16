mod abi;
mod config;

pub mod pb;

pub use config::AppConfig;

use anyhow::{Context, Result};
use crm_core::SendTrace;
use crm_metadata::pb::metadata_client::MetadataClient;
use crm_notification::pb::notification_client::NotificationClient;
use pb::{
    crm_server::{Crm, CrmServer},
    RecallRequest, RecallResponse, RemindRequest, RemindResponse, WelcomeRequest, WelcomeResponse,
};
use tonic::{
    async_trait, service::interceptor::InterceptedService, transport::Channel, Request, Response,
    Status,
};
use tracing::instrument;
use user_stat::pb::user_stats_client::UserStatsClient;

pub struct CrmService {
    config: AppConfig,
    user_stats: UserStatsClient<InterceptedService<Channel, SendTrace>>,
    notification: NotificationClient<InterceptedService<Channel, SendTrace>>,
    metadata: MetadataClient<InterceptedService<Channel, SendTrace>>,
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
        let channel = Channel::from_shared(config.server.user_stats.clone())?
            .connect()
            .await
            .context("user-stat service")?;
        let user_stats = UserStatsClient::with_interceptor(channel, SendTrace);

        let channel = Channel::from_shared(config.server.notification.clone())?
            .connect()
            .await
            .context("notification service")?;
        let notification = NotificationClient::with_interceptor(channel, SendTrace);

        let channel = Channel::from_shared(config.server.metadata.clone())?
            .connect()
            .await
            .context("metadata service")?;
        let metadata = MetadataClient::with_interceptor(channel, SendTrace);

        Ok(Self {
            config,
            user_stats,
            notification,
            metadata,
        })
    }

    pub fn into_server(self) -> Result<CrmServer<CrmService>> {
        Ok(CrmServer::new(self))
    }
}
