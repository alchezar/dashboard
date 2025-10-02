//! Public routes

use crate::error::{AuthError, Error};
use crate::model::queries;
use crate::model::types::{LoginPayload, NewUser};
use crate::prelude::Result;
use crate::web::auth;
use crate::web::auth::create_token;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use sqlx::PgPool;

pub fn routes() -> Router<PgPool> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[tracing::instrument(level = "trace",
	skip(pool, new_user),
	fields(email = %new_user.email))]
async fn register(
    State(pool): State<PgPool>,
    Json(new_user): Json<NewUser>,
) -> Result<Json<Value>> {
    let user = queries::add_new_user(&pool, new_user).await?;
    let token = create_token(user.id)?;
    tracing::info!("-- Token: {:?}", token);

    Ok(Json(json!({
        "result": {
            "token": token,
        }
    })))
}

#[tracing::instrument(level = "trace", skip(pool))]
async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<Value>> {
    let user = queries::get_user_by_email(&pool, &payload.email)
        .await
        .map_err(|_| Error::Auth(AuthError::WrongEmail))?;

    auth::verify_password(&user.password, &payload.password)?;
    let token = create_token(user.id)?;

    Ok(Json(json!({
        "result": {
            "token": token,
        }
    })))
}
