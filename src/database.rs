use crate::error::DashboardError;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::env;

pub async fn connect() -> Result<PgPool, DashboardError> {
    dotenv::dotenv()?;

    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new().connect(&database_url).await?;

    Ok(pool)
}
