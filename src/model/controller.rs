// use crate::error::{AuthError, Error};
use crate::model::types::User;
use crate::prelude::Result;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
pub struct Controller {
    pool: PgPool,
}

impl Controller {}

impl Controller {
    pub(crate) async fn add_new_user(&self, email: String, password: String) -> Result<User> {
        todo!()
    }

    pub async fn new() -> Result<Self> {
        dotenv::dotenv()?;

        let database_url = std::env::var("DATABASE_URL")?;
        let pool = PgPoolOptions::new().connect(&database_url).await?;

        Ok(Self { pool })
    }

    pub(crate) async fn get_user_by_id(&self, user_id: i32) -> Result<User> {
        Ok(sqlx::query_as!(
            User,
            r#"
			SELECT * FROM users WHERE id = $1
			"#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub(crate) async fn get_user_by_email(&self, email: &str) -> Result<User> {
        Ok(sqlx::query_as!(
            User,
            r#"
			SELECT * FROM users WHERE email = $1
			"#,
            email
        )
        .fetch_one(&self.pool)
        .await?)
    }
}
