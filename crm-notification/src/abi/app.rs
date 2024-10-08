use super::{to_ts, Sender};
use crate::{
    pb::{send_request::Msg, InAppMessage, SendRequest, SendResponse},
    NotificationService,
};
use tonic::Status;
use tracing::{debug, instrument, warn};

impl Sender for InAppMessage {
    #[instrument(name = "send-in-app", skip_all)]
    async fn send(self, svc: NotificationService) -> Result<SendResponse, Status> {
        let message_id = self.message_id.clone();
        svc.sender.send(Msg::InApp(self)).await.map_err(|e| {
            warn!("Failed to send message: {:?}", e);
            Status::internal("Failed to send message")
        })?;
        debug!("Sent in-app notification: {}", message_id);
        Ok(SendResponse {
            message_id,
            timestamp: Some(to_ts()),
        })
    }
}

impl From<InAppMessage> for Msg {
    fn from(in_app: InAppMessage) -> Self {
        Msg::InApp(in_app)
    }
}

impl From<InAppMessage> for SendRequest {
    fn from(in_app: InAppMessage) -> Self {
        let msg: Msg = in_app.into();
        SendRequest { msg: Some(msg) }
    }
}

#[cfg(feature = "test_utils")]
impl InAppMessage {
    pub fn fake() -> Self {
        use uuid::Uuid;
        InAppMessage {
            message_id: Uuid::new_v4().to_string(),
            device_id: Uuid::new_v4().to_string(),
            title: "Hello".to_string(),
            body: "Hello, world!".to_string(),
        }
    }
}
