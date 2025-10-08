use crate::prelude::Result;
use serde::Deserialize;
use std::sync::LazyLock;

/// Global lazily-initialized application [`Config`].
///
pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::from_env().unwrap_or_else(|error| panic!("== Failed to load config: {:?}!", error))
});

/// Represents the application's configuration.
///
#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub password_secret: String,
    pub token_secret: String,
    pub token_duration_sec: u64,
    pub proxmox_url: String,
    pub proxmox_auth_header: String,
}

impl Config {
    /// Loads the configuration from environment variables.
    ///
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv()?;
        tracing::info!(target: ">> config", ".env loaded.");

        let config = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()?;
        tracing::info!(target: ">> config", "Configuration loaded from environment variables.");

        Ok(config)
    }
}
