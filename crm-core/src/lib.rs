mod config;
mod error;
mod otel;
pub mod telemetry;

pub use config::ConfigExt;
pub use error::log_error;
pub use otel::{accept_trace, make_span, SendTrace};
use tokio::signal;
use tracing::info;

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {info!("shutdown gracefully")},
        _ = terminate => {},
    }
}
