use std::{mem, panic};

use anyhow::Result;
use crm::{AppConfig, CrmService};
use crm_core::{log_error, shutdown_signal, telemetry, ConfigExt};
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = AppConfig::load().expect("Failed to load config");

    // Initialize tracing & logging.
    telemetry::init(config.telemetry.clone()).inspect_err(log_error)?;

    // Replace the default panic hook with one that uses structured logging at ERROR level.
    panic::set_hook(Box::new(|panic| error!(%panic, "process panicked")));

    let tls = mem::take(&mut config.server.tls);

    let addr = config.server.port;
    let addr = format!("127.0.0.1:{}", addr).parse().unwrap();
    info!("CRM service listening on {}", addr);
    let svc = CrmService::try_new(config).await?.into_server()?;

    // if tls is enabled, use tls
    if let Some(tls) = tls {
        let identity = Identity::from_pem(tls.cert, tls.key);
        Server::builder()
            .tls_config(ServerTlsConfig::new().identity(identity))?
            .add_service(svc)
            .serve(addr)
            .await?;
    } else {
        info!("TLS is not enabled");
        Server::builder()
            .add_service(svc)
            .serve_with_shutdown(addr, shutdown_signal())
            .await?;
    }
    Ok(())
}
