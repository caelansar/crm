mod app;
mod email;
mod sms;

use chrono::Utc;
use futures::{Stream, StreamExt};
use prost_types::Timestamp;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::{info, warn};

use crate::{
    pb::{notification_server::NotificationServer, send_request::Msg, SendRequest, SendResponse},
    AppConfig, NotificationService, NotificationServiceInner, ServiceResult,
};

const CHANNEL_SIZE: usize = 1024;

/// A trait for sending notifications
pub trait Sender {
    async fn send(self, svc: NotificationService) -> Result<SendResponse, Status>;
}

impl NotificationService {
    pub fn new(config: AppConfig) -> Self {
        // In production, the sender should be a real implementation
        // In general, it had better be a message broker like Kafka, NATS, or RabbitMQ
        // so that we can scale the notification service horizontally
        let sender = dummy_send();
        let inner = NotificationServiceInner { config, sender };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn into_server(self) -> NotificationServer<Self> {
        NotificationServer::new(self)
    }

    /// Send a stream of notifications to the client
    /// Message type of request is one of the following:
    /// - EmailMessage
    /// - SmsMessage
    /// - InAppMessage
    pub async fn send(
        &self,
        mut stream: impl Stream<Item = Result<SendRequest, Status>> + Send + 'static + Unpin,
    ) -> ServiceResult<impl Stream<Item = Result<SendResponse, Status>> + Send + 'static> {
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
        let notify = self.clone();
        tokio::spawn(async move {
            while let Some(Ok(req)) = stream.next().await {
                let notify_clone = notify.clone();
                let res = match req.msg {
                    Some(Msg::Email(email)) => email.send(notify_clone).await,
                    Some(Msg::Sms(sms)) => sms.send(notify_clone).await,
                    Some(Msg::InApp(in_app)) => in_app.send(notify_clone).await,
                    None => {
                        warn!("Invalid request");
                        Err(Status::invalid_argument("Invalid request"))
                    }
                };
                tx.send(res).await.unwrap();
            }
        });

        let stream = ReceiverStream::new(rx);

        Ok(Response::new(stream))
    }
}

fn dummy_send() -> mpsc::Sender<Msg> {
    let (tx, mut rx) = mpsc::channel(CHANNEL_SIZE * 100);

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!("Received message: {:?}", msg);
            sleep(Duration::from_millis(300)).await;
        }
    });
    tx
}

fn to_ts() -> Timestamp {
    let now = Utc::now();
    Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    }
}
