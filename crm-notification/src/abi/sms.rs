use super::{to_ts, Sender};
use crate::{
    pb::{send_request::Msg, SendRequest, SendResponse, SmsMessage},
    NotificationService,
};
use tonic::Status;
use tracing::{debug, instrument, warn};

impl Sender for SmsMessage {
    #[instrument(name = "send-sms", skip_all)]
    async fn send(self, svc: NotificationService) -> Result<SendResponse, Status> {
        let message_id = self.message_id.clone();
        svc.sender.send(Msg::Sms(self)).await.map_err(|e| {
            warn!("Failed to send message: {:?}", e);
            Status::internal("Failed to send message")
        })?;
        debug!("Sent SMS notification: {}", message_id);
        Ok(SendResponse {
            message_id,
            timestamp: Some(to_ts()),
        })
    }
}

impl From<SmsMessage> for Msg {
    fn from(sms: SmsMessage) -> Self {
        Msg::Sms(sms)
    }
}

impl From<SmsMessage> for SendRequest {
    fn from(sms: SmsMessage) -> Self {
        let msg: Msg = sms.into();
        SendRequest { msg: Some(msg) }
    }
}

#[cfg(feature = "test_utils")]
impl SmsMessage {
    pub fn fake() -> Self {
        use fake::faker::phone_number::en::PhoneNumber;
        use fake::Fake;
        use uuid::Uuid;
        SmsMessage {
            message_id: Uuid::new_v4().to_string(),
            sender: PhoneNumber().fake(),
            recipients: vec![PhoneNumber().fake()],
            body: "Hello, world!".to_string(),
        }
    }
}
