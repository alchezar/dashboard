use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Represents a user row in the database, including the password hash.
///
#[derive(Debug, Clone, FromRow)]
pub struct DbUser {
    pub id: Uuid,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiUser {
    pub id: Uuid,
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
#[derive(Debug, Clone, Deserialize)]
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

/// Payload for authentication an existing user.
///
#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

// -----------------------------------------------------------------------------

/// Represents a row from the `services` table.
///
#[allow(unused)]
#[derive(Debug, Clone, FromRow)]
pub struct Service {
    id: Uuid,
    status: ServiceStatus,
    user_id: Uuid,
    server_id: Uuid,
    product_id: Uuid,
}

/// Represents the status from the `services` table.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "lowercase")]
pub enum ServiceStatus {
    Pending,
    Active,
    Failed,
}

impl From<&str> for ServiceStatus {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "pending" => ServiceStatus::Pending,
            "active" => ServiceStatus::Active,
            _ => ServiceStatus::Failed,
        }
    }
}

/// Represents a row from the `servers` table.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    id: Uuid,
    network_id: Uuid,
    vm_id: Option<i32>,
    node_name: Option<String>,
    ip_address: String,
}

/// Combined struct for the public API response.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiServer {
    pub service_id: Uuid,
    pub server_id: Uuid,
    pub vm_id: Option<i32>,
    pub node_name: Option<String>,
    pub ip_address: String,
    pub status: ServiceStatus,
}
