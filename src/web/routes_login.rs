//! Public routes

use crate::model::queries;
use crate::model::types::{LoginPayload, NewUser};
use crate::prelude::{AppState, AuthError, Error, Result};
use crate::web::auth::{password, token};
use crate::web::types::TokenResponse;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

#[tracing::instrument(level = "trace",
	skip(app_state, new_user),
	fields(email = %new_user.email))]
async fn register(
    State(app_state): State<AppState>,
    Json(new_user): Json<NewUser>,
) -> Result<Json<TokenResponse>> {
    let user = queries::add_new_user(&app_state.pool, new_user).await?;
    let token = token::create(user.id)?;
    tracing::info!(target: ">> server", "Token: {:?}", token);

    Ok(Json(TokenResponse::new(token.into())))
}

#[tracing::instrument(level = "trace", skip(app_state))]
async fn login(
    State(app_state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<TokenResponse>> {
    let user = queries::get_user_by_email(&app_state.pool, &payload.email)
        .await
        .map_err(|_| Error::Auth(AuthError::WrongEmail))?;
    password::verify(&user.password, &payload.password)?;
    let token = token::create(user.id)?;
    tracing::info!(target: ">> server", "Token: {:?}", token);

    Ok(Json(TokenResponse::new(token.into())))
}
