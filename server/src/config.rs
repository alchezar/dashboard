use dashboard_common::error::{Error, Result};
use serde::Deserialize;
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
}

impl Config {
    /// Loads the configuration from environment variables.
    ///
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv()?;
        tracing::info!(target: "config", ".env loaded.");

        let config_dif = std::env::current_dir()?.join("../configuration");
        let env_filename = (*std::env::var("APP_ENVIRONMENT")?)
            .try_into()
            .unwrap_or(Environment::Local)
            .as_filename();

        let config = config::Config::builder()
            .add_source(config::File::from(config_dif.join("base.yaml")))
            .add_source(config::File::from(config_dif.join(env_filename)))
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .build()?
            .try_deserialize::<Config>()?;

        tracing::info!(
            target: "config",
            application = ?config.application,
            database = ?config.database,
            "Configuration loaded.");

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
    password: String,
    database_name: String,
}

impl Database {
    /// Constructs the full database connection URL as a string.
    ///
    pub fn get_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
}

// -----------------------------------------------------------------------------

/// All settings required to work with JWT.
///
#[derive(Debug, Deserialize)]
pub struct TokenEnv {
    pub secret: String,
    pub duration_sec: u64,
}

/// All settings required to work with Proxmox.
///
#[derive(Debug, Clone, Deserialize)]
pub struct ProxmoxEnv {
    pub url: String,
    pub auth_header: String,
}

// -----------------------------------------------------------------------------

/// Represents the different environments the application can run in.
///
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    /// Returns the filename for the environment-specific configuration file.
    ///
    pub fn as_filename(&self) -> &'static str {
        match self {
            Environment::Local => "local.yaml",
            Environment::Production => "prod.yaml",
        }
    }
}

impl TryFrom<&str> for Environment {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "prod" => Ok(Self::Production),
            "production" => Ok(Self::Production),
            other => Err(Error::NotSupported(other.to_owned())),
        }
    }
}
