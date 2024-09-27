//! This example shows how to use the load balancer with a client.
//! The client will send requests to the load balancer(localhost:8000), which will
//! round robin between two upstream servers.

use anyhow::Result;
use crm_notification::pb::{
    notification_client::NotificationClient, EmailMessage, InAppMessage, SendRequest, SmsMessage,
};
use futures::StreamExt;
use load_balancer::{EncodingKey, User, SK};
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig},
    Request,
};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::Layer, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter, Layer as _,
};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new()
        .with_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()));
    tracing_subscriber::registry().with(layer).init();

    let ek = EncodingKey::load(SK)?;
    let token = ek.sign(User {
        name: "John Doe".to_string(),
        email: "john.doe@example.com".to_string(),
    })?;

    let pem = include_str!("../assets/cert/ca.crt");
    let tls = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(pem))
        .domain_name("kv.test.com");

    // localhost:8000 is pingora's load balancer server
    let channel = Channel::from_static("https://localhost:8000")
        .tls_config(tls)?
        .connect()
        .await?;

    // attach the token to the request
    let mut client = NotificationClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut()
            .insert("authorization", token.parse().unwrap());
        Ok(req)
    });

    let stream = tokio_stream::iter(vec![
        SendRequest {
            msg: Some(EmailMessage::default().into()),
        },
        SendRequest {
            msg: Some(SmsMessage::default().into()),
        },
        SendRequest {
            msg: Some(InAppMessage::default().into()),
        },
    ]);
    let request = Request::new(stream);
    let response = client.send(request).await?.into_inner();
    let ret: Vec<_> = response
        .filter_map(|res| async { res.ok() })
        .collect()
        .await;

    info!("ret: {:?}", ret);

    Ok(())
}
