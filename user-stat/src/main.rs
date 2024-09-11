use anyhow::Result;
use tonic::transport::Server;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
use user_stat::{AppConfig, UserStatsService};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = AppConfig::load().expect("Failed to load config");
    let addr = config.server.port;
    let addr = format!("127.0.0.1:{}", addr).parse().unwrap();
    info!("User-Stat service listening on {}", addr);
    let svc = UserStatsService::new(config).await.into_server();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let server = Server::builder().add_service(svc);

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install ctrl+c handler");
        tx.send(()).expect("Failed to send shutdown signal");
    });

    server
        .serve_with_shutdown(addr, async {
            rx.await.ok();
            info!("Shutting down gracefully");
        })
        .await?;

    Ok(())
}
