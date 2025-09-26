use thiserror::Error;

#[derive(Debug, Error)]
pub enum DashboardError {
	#[error("Environment error: {0}")]
	Environment(#[from] dotenv::Error),
	#[error("Environment variable error: {0}")]
	EnvironmentVariable(#[from] std::env::VarError),
	#[error("Database error: {0}")]
	Database(#[from] sqlx::Error),
	#[error("IO error: {0}")]
	InputOutput(#[from] std::io::Error),
}
