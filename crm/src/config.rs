use crm_core::telemetry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub telemetry: telemetry::Config,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub pk: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub sender_email: String,
    pub metadata: String,
    pub user_stats: String,
    pub notification: String,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}
