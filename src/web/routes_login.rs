//! Public routes

use crate::error::{AuthError, Error};
use crate::prelude::{Controller, Result};
use crate::web::auth::create_token;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn routes() -> Router<Controller> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[derive(Debug, Deserialize)]
struct UserAuthPayload {
    email: String,
    password: String,
}

async fn register(
    State(controller): State<Controller>,
    Json(payload): Json<UserAuthPayload>,
) -> Result<Json<Value>> {
    let user = controller
        .add_new_user(payload.email, payload.password)
        .await?;
    let token = create_token(user.id)?;

    let body = Json(json!({
        "result": {
            "token": token,
        }
    }));
    Ok(body)
}

async fn login(
    State(controller): State<Controller>,
    Json(payload): Json<UserAuthPayload>,
) -> Result<Json<Value>> {
    let user = controller.get_user_by_email(&payload.email).await?;
    let valid = argon2::verify_encoded(&user.password, payload.password.as_bytes())?;

    if !valid {
        return Err(Error::Auth(AuthError::WrongEmail));
    }

    let token = create_token(user.id)?;

    let body = Json(json!({
        "result": {
            "token": token,
        }
    }));
    Ok(body)
}

pub fn verify_password(encoded: &str, password: &str) -> Result<()> {
    let valid = argon2::verify_encoded(encoded, password.as_bytes())?;

    if valid {
        Ok(())
    } else {
        Err(Error::Auth(AuthError::WrongPassword))
    }
}
