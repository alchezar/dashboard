use crate::config::CONFIG;
use crate::error::{AuthError, Error, Result};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub user_id: uuid::Uuid,
}

pub fn create_token(user_id: uuid::Uuid) -> Result<String> {
    let now = chrono::Utc::now();
    let expires_in = chrono::Duration::seconds(CONFIG.token_duration_sec as i64);
    let exp = (now + expires_in).timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = Claims {
        exp,
        iat,
        user_id,
    };

    let header = Header::default();
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(CONFIG.token_secret.as_bytes()),
    )
    .map_err(|_| Error::Auth(AuthError::TokenCreation))?;

    Ok(token)
}

pub fn validate_token(token: &str) -> Result<Claims> {
    let decoding_key = DecodingKey::from_secret(CONFIG.token_secret.as_bytes());
    let validation = Validation::default();

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|_| Error::Auth(AuthError::TokenInvalid))
}