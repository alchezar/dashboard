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
/// # Examples
///
/// ```sh
/// curl --location 'http://127.0.0.1:8080/register' --header 'Content-Type: application/json' --data '{ "first_name": "John", "last_name": "Doe", "email": "john.doe@example.com", "password": "secure_password_123", "address": "123 Main St", "city": "Anytown", "state": "Any-state", "post_code": "12345", "country": "USA", "phone_number": "555-1234" }'
/// ```
///
#[tracing::instrument(level = "trace", target = "-- routes",
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
/// # Examples
///
/// ```sh
/// curl --location 'http://127.0.0.1:8080/login' --header 'Content-Type: application/json' --data '{ "email": "john.doe@example.com", "password": "secure_password_123" }'
/// ```
///
#[tracing::instrument(level = "trace", target = "-- routes",
	skip(app_state, payload),
	fields(email = %payload.email))]
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
