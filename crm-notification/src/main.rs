use anyhow::Result;
use crm_notification::{AppConfig, NotificationService};
use tonic::transport::Server;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer as _,
};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new()
        .with_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()));
    tracing_subscriber::registry().with(layer).init();

    let config = AppConfig::load().expect("Failed to load config");
    let addr = config.server.port;
    let addr = format!("127.0.0.1:{}", addr).parse().unwrap();
    info!("Notification service listening on {}", addr);

    let svc = NotificationService::new(config).into_server();
    Server::builder().add_service(svc).serve(addr).await?;
    Ok(())
}
