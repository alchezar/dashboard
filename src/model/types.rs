use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Default, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: uuid::Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub post_code: String,
    pub country: String,
    pub phone: String,
    pub password: String,
}