//! Protected routes

use crate::model::queries;
use crate::model::types::ApiServer;
use crate::services::{action, deletion, setup};
use crate::state::AppState;
use crate::web::auth::Claims;
use crate::web::middleware as mw;
use crate::web::types::*;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json};
use axum::{Router, middleware};
use dashboard_common::prelude::Result;
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
#[utoipa::path(
    get,
    path = "/user/me",
    security(("bearer_auth" = [])),
    tags = ["Server"],
    responses(
        (status = 200, body = UserResponse, description = "User found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "User not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
#[tracing::instrument(level = "trace", target = "handler",
	skip(app_state, claims),
	fields(id = %claims.user_id))]
async fn get_user(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserResponse>> {
    let user = queries::get_user_by_id(&app_state.pool, claims.user_id).await?;
    tracing::info!(target: "handler", email = user.email, "Found user");

    Ok(Json(Response::new(user)))
}

/// Returns the list of all servers that belong to currently authenticated user.
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
/// On success, returns a Json response with the list of user's servers.
/// [`ApiUser`].
///
#[utoipa::path(
    get,
    path = "/servers",
    tags = ["Server"],
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Response<Vec<ApiServer>>, description = "Servers found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "User not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
#[tracing::instrument(level = "trace", target = "handler",
	skip(app_state, claims),
	fields(id = %claims.user_id))]
async fn list_servers(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Response<Vec<ApiServer>>>> {
    let servers = queries::get_servers_for_user(&app_state.pool, claims.user_id).await?;
    tracing::info!(target: "handler", count = servers.len(), "Found servers");

    Ok(Json(Response::new(servers)))
}

/// Accepts a request to create a new server and starts the process in the
/// background.
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
/// * `Json(payload)`: specifications for the new server.
///
/// # Returns
///
/// This handler always returns an `HTTP 202 Accepted`
///
#[utoipa::path(
    post,
    path = "/servers",
    tags = ["Server"],
    security(("bearer_auth" = [])),
    responses(
        (status = 202, description = "Server creation accepted"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
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
/// This endpoint is protected, and the user is identified via the `user_id`
/// claim from the JWT provided in the `Authorization` bearer token.
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
#[utoipa::path(
    get,
    path = "/servers/{id}",
    tags = ["Server"],
    security(("bearer_auth" = [])),
    params(("id", Path, description = "Unique server ID")),
    responses(
        (status = 200, body = Response<ApiServer>, description = "Server found"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 404, body = String, description = "Server not found"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
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
/// This endpoint is protected, and the user is identified via the `user_id`
/// claim from the JWT provided in the `Authorization` bearer token.
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
#[utoipa::path(
    delete,
    path = "/servers/{id}",
    tags = ["Server"],
    security(("bearer_auth" = [])),
    params(("id", Path, description = "Unique server ID")),
    responses(
        (status = 202, description = "Server action accepted"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
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
/// This endpoint is protected, and the user is identified via the `user_id`
/// claim from the JWT provided in the `Authorization` bearer token.
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
#[utoipa::path(
    post,
    path = "/servers/{id}/actions",
    tags = ["Server"],
    security(("bearer_auth" = [])),
    params(("id", Path, description = "Unique server ID")),
    request_body = ServerActionPayload,
    responses(
        (status = 202, description = "Server deleted"),
        (status = 401, body = String, description = "Unauthorized"),
        (status = 500, body = String, description = "Internal server error")
    )
)]
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
