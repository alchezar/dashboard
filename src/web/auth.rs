use crate::config::CONFIG;
use crate::error::{AuthError, Error, Result};
use argon2::{ThreadMode, Variant, Version};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub user_id: i32,
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = rand::rng().random::<[u8; 16]>();
    let config = argon2::Config {
        ad: &[],
        hash_length: 32,
        lanes: 4,
        thread_mode: ThreadMode::Sequential,
        mem_cost: (u16::MAX + 1) as u32,
        secret: CONFIG.password_secret.as_bytes(),
        time_cost: 10,
        variant: Variant::Argon2id,
        version: Version::Version13,
    };
    argon2::hash_encoded(password.as_bytes(), &salt, &config).map_err(Error::Hash)
}

pub fn create_token(user_id: i32) -> Result<String> {
    let now = Utc::now();
    let expires_in = Duration::seconds(CONFIG.token_duration_sec as i64);
    let exp = (now + expires_in).timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = Claims { exp, iat, user_id };

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

pub fn verify_password(hash: &str, password: &str) -> Result<()> {
    match argon2::verify_encoded(hash, password.as_bytes())? {
        true => Ok(()),
        false => Err(Error::Auth(AuthError::WrongPassword)),
    }
}
