use axum::response::{IntoResponse, Response};
use derive_more::Display;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Authentication error: {0}")]
    Auth(AuthError),
    #[error("Environment error: {0}")]
    Environment(#[from] dotenv::Error),
    #[error("Envy error: {0}")]
    Envy(#[from] envy::Error),
    #[error("Environment variable error: {0}")]
    EnvironmentVariable(#[from] std::env::VarError),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    InputOutput(#[from] std::io::Error),
    #[error("Hash error: {0}")]
    Hash(#[from] argon2::Error),
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
