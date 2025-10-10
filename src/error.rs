use axum::response::{IntoResponse, Response};
use derive_more::Display;
use std::num::ParseIntError;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error: {0}")]
    Any(String),
	#[error("Not found: {0}")]
	NotFound(String),
	#[error("Not supported: {0}")]
	NotSupported(String),
	#[error("Timeout after: {0} milliseconds")]
	Timeout(f32),
    #[error("Authentication error: {0}")]
    Auth(AuthError),
    #[error("Proxmox API error: {0} failed: status {1}, body: {2}")]
    Proxmox(ProxmoxError, reqwest::StatusCode, String),

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
    #[error("Parse error: {0}")]
    Parse(#[from] ParseIntError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        use axum::http::StatusCode;

        let status = match self {
            Error::Auth(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let error = self.to_string();

        (status, error).into_response()
    }
}

#[derive(Debug, Display)]
pub enum AuthError {
    TokenCreation,
    TokenInvalid,
    TokenNotFound,
    WrongEmail,
    WrongPassword,
}

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
