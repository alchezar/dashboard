//! Protected routes

use crate::model::queries;
use crate::prelude::{ApiServer, AppState, Result};
use crate::services::{action, deletion, setup};
use crate::web::auth::Claims;
use crate::web::middleware as mw;
use crate::web::types::{NewServerPayload, Response, ServerActionPayload, UserResponse};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json};
use axum::{Router, middleware};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/user/me", get(get_user))
        .route("/servers", get(list_servers).post(create_server))
        .route("/servers/{id}", get(get_server).delete(delete_server))
        .route("/servers/{id}/actions", post(server_action))
        .route_layer(middleware::from_fn(mw::require_auth))
}

/// Returns the profile of the currently authenticated user.
///
/// This endpoint is protected, and the user is identified via the `user_id`
/// claim from the JWT provided in the `Authorization` bearer token.
///
/// # Arguments
///
/// * `State(app_state)`: The shared application state, containing the database
///   pool.
/// * `Extension(claims)`: The claims extracted from the JWT, which include the
///   user's ID.
///
/// # Returns
///
/// On success, returns a Json response with the user's public profile
/// [`ApiUser`].
///
/// # Errors
///
/// Returns an [`Error`] if a user with the ID from the JWT claims cannot be
/// found.
///
/// # Examples
///
/// ```sh
/// curl --location 'http://127.0.0.1:8080/user/me' --header 'Authorization: Bearer <TOKEN>'
/// ```
///
#[tracing::instrument(level = "trace", target = "handler",
	skip(app_state, claims),
	fields(id = %claims.user_id))]
async fn get_user(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserResponse>> {
    let user = queries::get_user_by_id(&app_state.pool, claims.user_id).await?;
    tracing::info!(target: "handler", "Found user: email={}", user.email);

    Ok(Json(Response::new(user)))
}

/// Returns the list of all servers that belong to currently authenticated user.
///
/// # Arguments
///
/// * `State(app_state)`: The shared application state, containing the database
///   pool.
/// * `Extension(claims)`: The claims extracted from the JWT, which include the
///   user's ID.
///
/// # Returns
///
/// On success, returns a Json response with the list of user's servers.
/// [`ApiUser`].
///
/// # Errors
///
/// Returns an [`Error`] if a user with the ID from the JWT claims cannot be
/// found.
///
#[tracing::instrument(level = "trace", target = "handler",
	skip(app_state, claims),
	fields(id = %claims.user_id))]
async fn list_servers(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Response<Vec<ApiServer>>>> {
    let servers = queries::get_servers_for_user(&app_state.pool, claims.user_id).await?;
    tracing::info!(target: "handler", "Found {} servers", servers.len());

    Ok(Json(Response::new(servers)))
}

/// Accepts a request to create a new server and starts the process in the
/// background.
///
/// # Arguments
///
/// * `State(app_state)`: The shared application state, containing the database
///   pool.
/// * `Extension(claims)`: The claims extracted from the JWT, which include the
///   user's ID.
/// * `Json(payload)`: specifications for the new server.
///
/// # Returns
///
/// This handler always returns an `HTTP 202 Accepted`
///
async fn create_server(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<NewServerPayload>,
) -> Result<StatusCode> {
    tokio::spawn(setup::run(app_state.clone(), claims.user_id, payload));

    Ok(StatusCode::ACCEPTED)
}

/// Retrieves and returns the details of a specific server.
///
/// # Arguments
///
/// * `State(app_state)`: Shared application state.
/// * `Extension(claims)`: Claims extracted from the JWT.
/// * `Path(server_id)`: Unique ID of the server to retrieve.
///
/// # Returns
///
/// On success, returns a Json response with the user's server.
/// [`ApiUser`].
///
async fn get_server(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Response<ApiServer>>> {
    let server = queries::get_server_by_id(&app_state.pool, claims.user_id, server_id).await?;
    tracing::info!(target: "handler", server_id = ?server.server_id, "Server found");

    Ok(Json(Response::new(server)))
}

/// Deletes a specific server and all associated data from the database
///
/// # Arguments
///
/// * `State(app_state)`: Shared application state.
/// * `Extension(claims)`: Claims extracted from the JWT.
/// * `Path(server_id)`: Unique ID of the server to delete.
///
/// # Returns
///
/// This handler always returns an `HTTP 202 Accepted`
///
async fn delete_server(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode> {
    tokio::spawn(deletion::run(app_state.clone(), claims.user_id, server_id));

    Ok(StatusCode::ACCEPTED)
}

/// Makes specific action on the server.
///
/// # Arguments
///
/// * `State(app_state)`: Shared application state.
/// * `Extension(claims)`: Claims extracted from the JWT.
/// * `Path(server_id)`: Unique ID of the server to delete.
/// * `Json(payload)`: specific action for the server.
///
/// # Returns
///
/// This handler always returns an `HTTP 202 Accepted`
///
async fn server_action(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(server_id): Path<Uuid>,
    Json(payload): Json<ServerActionPayload>,
) -> Result<StatusCode> {
    tokio::spawn(action::run(
        app_state.clone(),
        claims.user_id,
        server_id,
        payload.action,
    ));

    Ok(StatusCode::ACCEPTED)
}
