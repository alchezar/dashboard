use crate::model::types::ApiUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type TokenResponse = Response<TokenPayload>;
pub type UserResponse = Response<ApiUser>;

/// Generic API response.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T> {
    pub result: T,
}

impl<T> Response<T> {
    /// Creates a new instance of the API response.
    ///
    pub fn new(result: T) -> Self {
        Self { result }
    }
}

/// Payload for successful registration or login, containing `JWT`.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPayload {
    pub token: String,
}

impl From<String> for TokenPayload {
    fn from(token: String) -> Self {
        Self { token }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewServerPayload {
    pub product_id: Uuid,
    pub host_name: String,
    pub cpu_cores: Option<i32>,
    pub ram_gb: Option<i32>,
    pub os: String,
    pub datacenter: String,
    pub ip_config: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ServerActionPayload {
    pub action: ServerAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerAction {
    Start,
    Stop,
    // Shutting VM down, and starting it again.
    Reboot,
    // This is similar to pressing the power button on a physical machine.
    Shutdown,
}
