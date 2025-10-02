use crate::config::CONFIG;
use crate::model::types::{ApiUser, DbUser, NewUser};
use crate::prelude::Result;
use crate::web::auth::hash_password;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

#[tracing::instrument(level = "trace")]
pub async fn connect_to_db() -> Result<PgPool> {
    let database_url = &CONFIG.database_url;
    let pool = PgPoolOptions::new().connect(&database_url).await?;

    Ok(pool)
}

pub(crate) async fn add_new_user(pool: &PgPool, new_user: NewUser) -> Result<ApiUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
INSERT INTO users (
    first_name,
    last_name,
    email,
    address,
    city,
    state,
    post_code,
    country,
    phone_number,
    password)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
RETURNING *
		"#,
        new_user.first_name,
        new_user.last_name,
        new_user.email,
        new_user.address,
        new_user.city,
        new_user.state,
        new_user.post_code,
        new_user.country,
        new_user.phone_number,
        hash_password(&new_user.plain_password)?
    )
    .fetch_one(pool)
    .await?
    .into())
}

pub(crate) async fn get_user_by_id(pool: &PgPool, user_id: i32) -> Result<ApiUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
SELECT * FROM users WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?
    .into())
}

pub(crate) async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<DbUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
SELECT * FROM users WHERE email = $1
        "#,
        email
    )
    .fetch_one(pool)
    .await?)
}
