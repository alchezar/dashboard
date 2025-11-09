use axum::response::{IntoResponse, Response};
use derive_more::Display;
use std::num::ParseIntError;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

/// Defines the application's custom error types.
///
#[derive(Debug, Error)]
pub enum Error {
    #[error("Error: {0}")]
    Any(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Not ready: {0}")]
    NotReady(String),
    #[error("Not supported: {0}")]
    NotSupported(String),
    #[error("Timeout after: {0} milliseconds")]
    Timeout(f32),
    #[error("Authentication error: {0}")]
    Auth(AuthError),
    #[error("Proxmox API error: {0} failed: status {1}, body: {2}")]
    Proxmox(ProxmoxError, reqwest::StatusCode, String),
    #[error("Header convert error: {0}")]
    Header(#[from] axum::http::header::InvalidHeaderValue),

    #[error("Environment error: {0}")]
    Environment(#[from] dotenv::Error),
    #[error("Environment variable error: {0}")]
    EnvironmentVariable(#[from] std::env::VarError),
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Failed to set logger: {0}")]
    Logger(#[from] tracing::log::SetLoggerError),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    InputOutput(#[from] std::io::Error),
    #[error("Hash error: {0}")]
    Hash(#[from] argon2::Error),
    #[error("Telemetry error: {0}")]
    Telemetry(#[from] tracing::dispatcher::SetGlobalDefaultError),
    #[error("Int parse error: {0}")]
    ParseInt(#[from] ParseIntError),
    #[error("Address parse error: {0}")]
    ParseAddr(#[from] std::net::AddrParseError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        use axum::http::StatusCode;

        match self {
            Error::Auth(AuthError::Token) => (
                StatusCode::UNAUTHORIZED,
                "Authorization token is missing or invalid!".to_owned(),
            ),
            Error::Auth(AuthError::Login) | Error::Hash(_) => (
                StatusCode::UNAUTHORIZED,
                "Incorrect email or password!".to_owned(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error!".to_owned(),
            ),
        }
        .into_response()
    }
}

/// Represents authentication-related errors.
///
#[derive(Debug, Display)]
pub enum AuthError {
    Token,
    Login,
}

/// Represents errors related to Proxmox API operations.
///
#[derive(Debug, Display)]
pub enum ProxmoxError {
    Start,
    Shutdown,
    Stop,
    Reboot,
    Create,
    Delete,
    Status,
}
