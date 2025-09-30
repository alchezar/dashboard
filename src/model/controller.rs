use crate::prelude::Result;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub struct Controller {
    pool: PgPool,
}

impl Controller {
    pub async fn new() -> Result<Self> {
        dotenv::dotenv()?;

        let database_url = std::env::var("DATABASE_URL")?;
        let pool = PgPoolOptions::new().connect(&database_url).await?;

        Ok(Self { pool })
    }
}
