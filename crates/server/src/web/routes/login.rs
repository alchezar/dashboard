//! Public routes

use crate::model::queries;
use crate::model::types::{LoginPayload, NewUser};
use crate::state::AppState;
use crate::web::auth::{password, token};
use crate::web::types::TokenResponse;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use dashboard_common::prelude::{AuthError, Error, Result};
use secrecy::ExposeSecret;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

/// Creates a new user account.
///
/// On successful registration, it returns a `TokenResponse` containing a JWT
/// for the newly created user.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state, containing the database
///   pool.
/// * `Json(new_user)` - Payload for creating a new user, contains the plaintext
///   password.
///
/// # Errors
///
/// Returns an `Error` if the database query fails  or if JWT creation fails.
///
#[utoipa::path(
    post,
    path = "/register",
    request_body = NewUser,
    tags = ["Login"],
    responses(
        (status = 200, body = TokenResponse, description = "User registration completed"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
#[tracing::instrument(level = "trace", target = "handler",
	skip(app_state, new_user),
	fields(email = %new_user.email))]
async fn register(
    State(app_state): State<AppState>,
    Json(new_user): Json<NewUser>,
) -> Result<Json<TokenResponse>> {
    let user = queries::add_new_user(&app_state.pool, new_user).await?;
    let token = token::create(user.id)?;
    tracing::info!(target: "handler", user_id = %user.id, "Token generated successfully");

    Ok(Json(TokenResponse::new(token.into())))
}

/// Authenticates a user and provides a JWT.
///
/// Takes a user's email and password, verifies them against the database,
/// and returns a `TokenResponse` with a new JWT on success.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state, containing the database
///   pool.
/// * `Json(payload)` - Payload for authentication an existing user.
///
/// # Returns
///
/// On success, returns a Json response with a new JWT.
///
/// # Errors
///
/// Returns an `Error` if the user is not found by email, if the password
/// verification fails, or if JWT creation fails.
///
#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginPayload,
    tags = ["Login"],
    responses(
        (status = 200, body = TokenResponse, description = "User login completed"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
#[tracing::instrument(level = "trace", target = "handler",
	skip(app_state, payload),
	fields(email = %payload.email))]
async fn login(
    State(app_state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<TokenResponse>> {
    let user = queries::get_user_by_email(&app_state.pool, &payload.email)
        .await
        .map_err(|_| Error::Auth(AuthError::Login))?;

    let (hash, pass) = (
        user.password.expose_secret(),
        payload.password.expose_secret(),
    );
    if hash.starts_with("$2y$10$") {
        // Rehash old WHMCS passwords immediately, without confirmation email.
        bcrypt::verify(pass, hash).map_err(|_| Error::Auth(AuthError::Login))?;
        let new_hash = password::hash(pass)?;
        queries::update_password(&app_state.pool, &user.id, &new_hash).await?;
        tracing::info!(target: "handler", user_id = %user.id, "Old password hash updated");
    } else {
        password::verify(hash, pass)?;
    }

    let token = token::create(user.id)?;
    tracing::info!(target: "handler", user_id = %user.id, "Token generated successfully");

    Ok(Json(TokenResponse::new(token.into())))
}
