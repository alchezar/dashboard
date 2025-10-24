use chrono::{DateTime, Utc};
use common::error::Result;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::net::Ipv4Addr;
use utoipa::ToSchema;
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Deserialize, ToSchema)]
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
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

// -----------------------------------------------------------------------------

/// Represents the status from the `services` table.
///
#[derive(Debug, Clone, Display, Serialize, Deserialize)]
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

impl From<String> for ServiceStatus {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Represents the status from the `servers` table.
///
#[derive(Debug, Clone, Copy, PartialEq, Display, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ServerStatus {
    // Stable statuses.
    Running,
    Stopped,
    Failed,
    // Lifecycle.
    SettingUp,
    Deleting,
    // Progress statuses.
    Starting,
    Stopping,
    Rebooting,
    ShuttingDown,
}

impl From<&str> for ServerStatus {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "running" => ServerStatus::Running,
            "stopped" => ServerStatus::Stopped,
            "setting_up" => ServerStatus::SettingUp,
            "deleting" => ServerStatus::Deleting,
            "starting" => ServerStatus::Starting,
            "stopping" => ServerStatus::Stopping,
            "rebooting" => ServerStatus::Rebooting,
            "shutting_down" => ServerStatus::ShuttingDown,
            _ => ServerStatus::Failed,
        }
    }
}

impl From<String> for ServerStatus {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// Represents a row from the `servers` table.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: Uuid,
    pub vm_id: Option<i32>,
    pub node_name: Option<String>,
    pub host_name: String,
}

/// Combined struct for the public API response.
///
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiServer {
    pub service_id: Uuid,
    pub server_id: Uuid,
    pub vm_id: Option<i32>,
    pub node_name: Option<String>,
    pub ip_address: String,
    pub status: ServerStatus,
}

/// Configuration for an IP address.
///
#[derive(Debug)]
pub struct IpConfig {
    pub ip_address: String,
    pub gateway: String,
    pub subnet_mask: String,
}

impl IpConfig {
    /// Formats the IP configuration into a string suitable for Proxmox.
    ///
    /// # Returns
    ///
    /// A formatted string, e.g., "ip=192.168.1.100/24,gw=192.168.1.1".
    ///
    pub fn form(self) -> Result<String> {
        // Convert IPv4 mask into CIDR, for example:
        // 255.255.255.0 -> 11111111.11111111.11111111.00000000 -> /24
        let mask = self.subnet_mask.parse::<Ipv4Addr>()?;
        let mask_u32 = u32::from(mask);
        let cidr_prefix = mask_u32.leading_ones();

        Ok(format!(
            "ip={}/{},gw={}",
            self.ip_address, cidr_prefix, self.gateway
        ))
    }
}
