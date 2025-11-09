use axum::http::{HeaderName, HeaderValue, Method};
use dashboard_common::prelude::Result;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnectOptions;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Represents the application's configuration.
///
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub application: SocketAddr,
    pub database: Database,
    pub token: TokenEnv,
    pub proxmox: ProxmoxEnv,
    pub cors: Cors,
}

impl Config {
    /// Loads the configuration from environment variables.
    ///
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv()?;
        tracing::info!(target: "config", ".env loaded.");

        let config_dir = std::path::PathBuf::from(std::env::var("APP_CONFIG_PATH")?);
        let env_filename = Environment::from(&*std::env::var("APP_ENVIRONMENT")?).as_filename();

        let config = config::Config::builder()
            .add_source(config::File::from(config_dir.join("base.yaml")))
            .add_source(config::File::from(config_dir.join(env_filename)))
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .build()?
            .try_deserialize::<Config>()?;

        tracing::info!( target: "config", ?config, "Configuration loaded.");

        Ok(config)
    }

    /// Returns the database connection options.
    ///
    pub fn get_database_connect_options(&self) -> PgConnectOptions {
        self.database.get_connect_options()
    }

    /// Returns the socket address for the application server to bind to.
    ///
    pub fn get_address(&self) -> SocketAddr {
        self.application
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            application: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            database: Database::default(),
            token: TokenEnv::default(),
            proxmox: ProxmoxEnv::default(),
            cors: Cors::default(),
        }
    }
}

// -----------------------------------------------------------------------------

/// All settings required to connect to the database.
///
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Database {
    host: String,
    port: u16,
    username: String,
    password: SecretString,
    database_name: String,
}

impl Database {
    /// Constructs a `PgConnectOptions` instance for connecting to the database.
    ///
    pub fn get_connect_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(self.password.expose_secret())
            .database(&self.database_name)
    }
}

// -----------------------------------------------------------------------------

/// All settings required to work with JWT.
///
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TokenEnv {
    pub secret: SecretString,
    pub duration_sec: u64,
}

/// All settings required to work with Proxmox.
///
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProxmoxEnv {
    pub url: String,
    pub auth_header: SecretString,
}

// -----------------------------------------------------------------------------

/// Represents the different environments the application can run in.
///
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    /// Returns the filename for the environment-specific configuration file.
    ///
    pub fn as_filename(&self) -> String {
        serde_json::to_string(self)
            .expect("Environment should always serialize successfully")
            .trim_matches('"')
            .to_owned()
            + ".yaml"
    }
}

impl From<&str> for Environment {
    fn from(value: &str) -> Self {
        match serde_json::from_str::<Self>(value) {
            Ok(environment) => environment,
            Err(error) => {
                tracing::warn!(target: "config", value, ?error, "Incorrect environment format. Use either `local` or `production`.");
                Self::Local
            }
        }
    }
}

// -----------------------------------------------------------------------------

/// Configuration for Cross-Origin Resource Sharing (CORS).
///
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Cors {
    origin: String,
    methods: String,
    headers: String,
}

impl Cors {
    /// Parses the configured origin string into an `axum::http::HeaderValue`.
    /// If the origin string cannot be parsed, it defaults to an "empty"
    /// `HeaderValue`.
    ///
    /// # Returns
    ///
    /// `HeaderValue` representing the allowed origin.
    ///
    pub fn allow_origin(&self) -> HeaderValue {
        self.origin.parse().unwrap_or(HeaderValue::from_static(""))
    }

    /// Parses the comma-separated methods string into a vector of
    /// `axum::http::Method`. Each method string is trimmed and parsed. Invalid
    /// method strings are ignored.
    ///
    /// # Returns
    ///
    /// `Vec<Method>` containing the allowed HTTP methods.
    ///
    pub fn allow_methods(&self) -> Vec<Method> {
        self.methods
            .split(',')
            .filter_map(|method| method.trim().parse().ok())
            .collect()
    }

    /// Parses the comma-separated headers string into a vector of
    /// `axum::http::HeaderName`. Each header string is trimmed and parsed.
    /// Invalid header strings are ignored.
    ///
    /// # Returns
    ///
    /// `Vec<HeaderName>` containing the allowed HTTP headers.
    ///
    pub fn allow_headers(&self) -> Vec<HeaderName> {
        self.headers
            .split(',')
            .filter_map(|header| header.trim().parse().ok())
            .collect()
    }
}
