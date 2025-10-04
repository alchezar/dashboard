use crate::model::types::ApiUser;
use serde::{Deserialize, Serialize};

pub type TokenResponse = Response<TokenPayload>;
pub type UserResponse = Response<ApiUser>;

/// Generic API response.
#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T> {
    pub result: T,
}

impl<T> Response<T> {
    pub fn new(result: T) -> Self {
        Self { result }
    }
}

/// Payload for successful registration or login, containing `JWT`.
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPayload {
    pub token: String,
}

impl From<String> for TokenPayload {
    fn from(token: String) -> Self {
        Self { token }
    }
}
