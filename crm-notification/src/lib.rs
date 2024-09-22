#![feature(impl_trait_in_assoc_type)]

pub mod pb;

mod abi;
mod config;

use std::{ops::Deref, sync::Arc};

pub use config::AppConfig;
use futures::Stream;
use pb::{notification_server::Notification, send_request::Msg, SendRequest, SendResponse};
use tokio::sync::mpsc;
use tonic::{async_trait, Request, Response, Status, Streaming};
use tracing::instrument;

#[derive(Clone)]
pub struct NotificationService {
    inner: Arc<NotificationServiceInner>,
}

#[allow(unused)]
pub struct NotificationServiceInner {
    config: AppConfig,
    sender: mpsc::Sender<Msg>,
}

type ServiceResult<T> = Result<Response<T>, Status>;

#[async_trait]
impl Notification for NotificationService {
    type SendStream = impl Stream<Item = Result<SendResponse, Status>> + Send + 'static;

    #[instrument(name = "send-handler", skip(self, request))]
    async fn send(
        &self,
        request: Request<Streaming<SendRequest>>,
    ) -> Result<Response<Self::SendStream>, Status> {
        let stream = request.into_inner();
        self.send(stream).await
    }
}

impl Deref for NotificationService {
    type Target = NotificationServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
