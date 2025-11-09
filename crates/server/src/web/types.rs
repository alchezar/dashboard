use crate::model::types::ApiUser;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;

/// API response with JWT inside.
///
pub type TokenResponse = Response<TokenPayload>;

/// API response with user info inside.
///
pub type UserResponse = Response<ApiUser>;

/// Generic API response.
///
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenPayload {
    pub token: String,
}

impl From<String> for TokenPayload {
    fn from(token: String) -> Self {
        Self { token }
    }
}

/// Payload for creating a new server.
///
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewServerPayload {
    pub product_id: Uuid,
    pub host_name: String,
    pub cpu_cores: Option<i32>,
    pub ram_gb: Option<i32>,
    pub os: String,
    pub datacenter: String,
    pub ip_config: Option<String>,
}

/// Payload for performing an action on a server.
///
#[derive(Debug, Deserialize, ToSchema)]
pub struct ServerActionPayload {
    pub action: ServerAction,
}

/// Represents the possible actions that can be performed on a server.
///
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ServerAction {
    Start,
    Stop,
    // Shutting VM down, and starting it again.
    Reboot,
    // This is similar to pressing the power button on a physical machine.
    Shutdown,
}

/// Represents all required configurable options.
///
#[derive(Debug, Display)]
pub enum RequiredConfigOption {
    CPU,
    RAM,
}

/// Represents all required custom fields.
///
#[derive(Debug, Serialize)]
pub enum RequiredCustomField {
    #[serde(rename = "OS Template")]
    OsTemplate,
    #[serde(rename = "Datacenter Location")]
    Datacenter,
}

// Small overhead to avoid magick values in match hands.
//
impl Display for RequiredCustomField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_string(self).unwrap();
        f.write_str(value.trim_matches('"'))
    }
}
