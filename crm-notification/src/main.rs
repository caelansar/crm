use std::mem;

use anyhow::Result;
use crm_core::{accept_trace, log_error, make_span, telemetry, ConfigExt};
use crm_notification::{AppConfig, NotificationService};
use tonic::transport::Server;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = AppConfig::load().expect("Failed to load config");

    let telemetry_config = mem::take(&mut config.telemetry);

    telemetry::init(telemetry_config).inspect_err(log_error)?;

    let addr = config.server.port;
    let addr = format!("127.0.0.1:{}", addr).parse().unwrap();
    info!("Notification service listening on {}", addr);

    let svc = NotificationService::new(config).into_server();

    Server::builder()
        .layer(
            ServiceBuilder::new()
                .layer(tower::timeout::TimeoutLayer::new(
                    std::time::Duration::from_secs(30),
                ))
                .layer(
                    TraceLayer::new_for_grpc()
                        .make_span_with(make_span)
                        .on_request(accept_trace),
                ),
        )
        .add_service(svc)
        .serve(addr)
        .await?;
    Ok(())
}
