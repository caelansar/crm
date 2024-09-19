use anyhow::Result;
use clickhouse::Client;
use crm_core::{accept_trace, log_error, telemetry, ConfigExt};
use tonic::transport::Server;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{field, info, info_span, Span};
use user_stat::{AppConfig, ClickHouseRepo, UserStatsService};

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load().expect("Failed to load config");

    telemetry::init(config.telemetry.clone()).inspect_err(log_error)?;

    let addr = config.server.port;
    let addr = format!("127.0.0.1:{}", addr).parse().unwrap();
    info!("User-Stat service listening on {}", addr);

    let client = Client::default()
        .with_url(&config.server.db_url)
        .with_database(&config.server.db_name);

    let repo = ClickHouseRepo::new(client);

    let svc = UserStatsService::new(repo, config).await.into_server();
    let server = Server::builder()
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
        .add_service(svc);

    server.serve(addr).await?;

    Ok(())
}

fn make_span<B>(request: &http::Request<B>) -> Span {
    let headers = request.headers();
    info_span!("incoming request", ?headers, trace_id = field::Empty)
}
