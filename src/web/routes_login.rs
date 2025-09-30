use crate::prelude::{Controller, Result};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};
use tower_cookies::cookie::CookieJar;

pub fn routes() -> Router {
    Router::new()
        .route("/login", post(login))
        .route("user/me", get(get_user))
}

#[derive(Debug, Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

async fn login(
    jar: CookieJar,
    State(controller): State<Controller>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<Value>> {
    // todo!("Implement db/auth logic!");
    // Err(Error::Database(sqlx::Error::BeginFailed))

    let body = Json(json!({
        "result": {
            "success": true,
        }
    }));

    Ok(body)
}

async fn get_user(State(controller): State<Controller>) {}
