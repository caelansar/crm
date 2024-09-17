use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use crm_core::ConfigExt;
use crm_notification::{
    pb::{
        notification_client::NotificationClient, EmailMessage, InAppMessage, SendRequest,
        SmsMessage,
    },
    AppConfig, NotificationService,
};
use futures::StreamExt;
use tokio::time::sleep;
use tonic::{transport::Server, Request};

#[tokio::test]
async fn send_notification_should_work() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = start_server().await?;
    let mut client = NotificationClient::connect(format!("http://{}", addr)).await?;
    let stream = tokio_stream::iter(vec![
        SendRequest {
            msg: Some(EmailMessage::fake().into()),
        },
        SendRequest {
            msg: Some(SmsMessage::fake().into()),
        },
        SendRequest {
            msg: Some(InAppMessage::fake().into()),
        },
    ]);
    let request = Request::new(stream);
    let response = client.send(request).await?.into_inner();
    let ret: Vec<_> = response
        .filter_map(|res| async { res.ok() })
        .collect()
        .await;

    assert_eq!(ret.len(), 3);

    Ok(())
}

async fn start_server() -> Result<SocketAddr> {
    let config = AppConfig::load()?;
    let addr = format!("127.0.0.1:{}", config.server.port).parse()?;

    let svc = NotificationService::new(config).into_server();
    tokio::spawn(async move {
        Server::builder()
            .add_service(svc)
            .serve(addr)
            .await
            .unwrap();
    });
    sleep(Duration::from_micros(1)).await;

    Ok(addr)
}
