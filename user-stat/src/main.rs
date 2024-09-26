use anyhow::Result;
use clickhouse::Client;
use crm_core::{accept_trace, log_error, make_span, telemetry, ConfigExt};
use tonic::transport::Server;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;
use user_stat::{AppConfig, ClickHouseRepo, DBType, PostgresRepo, UserStatsService};

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load().expect("Failed to load config");

    telemetry::init(config.telemetry.clone()).inspect_err(log_error)?;

    let addr = config.server.port;
    let addr = format!("127.0.0.1:{}", addr).parse().unwrap();
    info!("User-Stat service listening on {}", addr);

    let mut server = Server::builder().layer(
        ServiceBuilder::new()
            .layer(tower::timeout::TimeoutLayer::new(
                std::time::Duration::from_secs(30),
            ))
            .layer(
                TraceLayer::new_for_grpc()
                    .make_span_with(make_span)
                    .on_request(accept_trace),
            ),
    );

    // initialize service with different db type by db_type in configuration file
    // and add it into server
    let router = match config.server.db_type {
        DBType::Clickhouse => {
            info!("Using Clickhouse as database");
            let repo = ClickHouseRepo::new(
                Client::default()
                    .with_url(&config.server.db_url)
                    .with_database(&config.server.db_name),
            );
            let svc = UserStatsService::new(repo, config).await.into_server();
            server.add_service(svc)
        }
        DBType::Postgres => {
            info!("Using Postgres as database");
            let repo = PostgresRepo::new(&format!(
                "{}/{}",
                config.server.db_url, config.server.db_name
            ))
            .await?;
            let svc = UserStatsService::new(repo, config).await.into_server();
            server.add_service(svc)
        }
    };

    router.serve(addr).await?;

    Ok(())
}
