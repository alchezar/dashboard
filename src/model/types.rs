use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a user row in the database, including the password hash.
///
#[derive(Debug, Clone, FromRow)]
pub struct DbUser {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub post_code: String,
    pub country: String,
    pub phone_number: String,
    pub password: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Represents a user that is safe to expose to the public API.
///
#[derive(Debug, Clone, Serialize)]
pub struct ApiUser {
    pub id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub post_code: String,
    pub country: String,
    pub phone_number: String,
}

/// Payload for creating a new user, contains the plaintext password
///
#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub post_code: String,
    pub country: String,
    pub phone_number: String,
    #[serde(rename = "password")]
    pub plain_password: String,
}

impl From<DbUser> for ApiUser {
    fn from(user: DbUser) -> Self {
        Self {
            id: user.id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            address: user.address,
            city: user.city,
            state: user.state,
            post_code: user.post_code,
            country: user.country,
            phone_number: user.phone_number,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}
