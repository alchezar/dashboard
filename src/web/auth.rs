use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub user_id: Uuid,
}

pub mod password {
    use crate::error::{AuthError, Error};
    use rand::Rng;

    pub fn verify(hash: &str, password: &str) -> crate::error::Result<()> {
        match argon2::verify_encoded(hash, password.as_bytes())? {
            true => Ok(()),
            false => Err(Error::Auth(AuthError::WrongPassword)),
        }
    }

    pub fn hash(password: &str) -> crate::error::Result<String> {
        let salt = rand::rng().random::<[u8; 16]>();
        argon2::hash_encoded(password.as_bytes(), &salt, &argon2::Config::default())
            .map_err(Error::Hash)
    }
}

pub mod token {
    use crate::config::CONFIG;
    use crate::error::{AuthError, Error};
    use crate::web::auth::Claims;
    use chrono::{Duration, Utc};
    use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
    use uuid::Uuid;

    pub fn create(user_id: Uuid) -> crate::error::Result<String> {
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

    pub fn validate(token: &str) -> crate::error::Result<Claims> {
        let decoding_key = DecodingKey::from_secret(CONFIG.token_secret.as_bytes());
        let validation = Validation::default();

        decode::<Claims>(token, &decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| Error::Auth(AuthError::TokenInvalid))
    }
}
