use axum::http::{HeaderName, HeaderValue, Method};
use dashboard_common::prelude::{Error, Result};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, ToSocketAddrs};
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
    application: Application,
    database: Database,
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

        let config_dir =
            std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).join("../../configuration");
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

    /// Returns the full database connection URL as a string.
    ///
    pub fn get_database_url(&self) -> String {
        self.database.get_url()
    }

    /// Returns the socket address for the application server to bind to.
    ///
    pub fn get_address(&self) -> Result<SocketAddr> {
        self.application.get_address()
    }
}

// -----------------------------------------------------------------------------

/// Configuration, specific to the application server.
///
#[derive(Debug, Deserialize)]
pub struct Application {
    host: String,
    port: u16,
}

impl Application {
    /// Constructs a `SocketAddr` from the configured host and port.
    ///
    pub fn get_address(&self) -> Result<SocketAddr> {
        let (host, port) = (self.host.as_str(), self.port);
        (host, port)
            .to_socket_addrs()?
            .next()
            .ok_or(Error::NotFound(format!("IP Address for {}:{}", host, port)))
    }
}

// -----------------------------------------------------------------------------

/// All settings required to connect to the database.
///
#[derive(Debug, Deserialize)]
pub struct Database {
    host: String,
    port: u16,
    username: String,
    password: SecretString,
    database_name: String,
}

impl Database {
    /// Constructs the full database connection URL as a string.
    ///
    pub fn get_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
    }
}

// -----------------------------------------------------------------------------

/// All settings required to work with JWT.
///
#[derive(Debug, Deserialize)]
pub struct TokenEnv {
    pub secret: SecretString,
    pub duration_sec: u64,
}

/// All settings required to work with Proxmox.
///
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
