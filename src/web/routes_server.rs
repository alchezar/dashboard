//! Protected routes
#![allow(unused)]

use crate::model::queries;
use crate::model::types::{ApiServer, ApiUser};
use crate::prelude::{AppState, ProxmoxError};
use crate::prelude::{Error, Result};
use crate::proxmox::types::{TaskRef, TaskStatus, UniqueProcessId, VmConfig, VmRef};
use crate::services::server_setup;
use crate::web::auth::Claims;
use crate::web::mw_auth;
use crate::web::types::{NewServerPayload, Response, UserResponse};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json};
use axum::{Router, middleware};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/user/me", get(get_user))
        .route("/servers", get(list_servers).post(create_server))
        .route("/servers/{id}", get(get_server).delete(delete_server))
        .route("/servers/{id}/actions", post(server_action))
        .route_layer(middleware::from_fn(mw_auth::mw_require_auth))
}

/// Returns the profile of the currently authenticated user.
///
/// This endpoint is protected, and the user is identified via the `user_id`
/// claim from the JWT provided in the `Authorization` bearer token.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state, containing the database
///   pool.
/// * `Extension(claims)` - The claims extracted from the JWT, which include the
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
#[tracing::instrument(level = "trace", target = "-- handler",
	skip(app_state, claims),
	fields(id = %claims.user_id))]
async fn get_user(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserResponse>> {
    let user = queries::get_user_by_id(&app_state.pool, claims.user_id).await?;
    tracing::info!(target: ">> handler", "Found user: email={}", user.email);

    Ok(Json(Response::new(user)))
}

/// Returns the list of all servers that belong to currently authenticated user.
///
/// # Arguments
///
/// * `State(app_state)` - The shared application state, containing the database
///   pool.
/// * `Extension(claims)` - The claims extracted from the JWT, which include the
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
#[tracing::instrument(level = "trace", target = "-- handler",
	skip(app_state, claims),
	fields(id = %claims.user_id))]
async fn list_servers(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Response<Vec<ApiServer>>>> {
    let servers = queries::get_servers_for_user(&app_state.pool, claims.user_id).await?;
    tracing::info!(target: ">> handler", "Found {} servers", servers.len());

    Ok(Json(Response::new(servers)))
}

async fn create_server(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<NewServerPayload>,
) -> impl IntoResponse {
    tokio::spawn(server_setup::setup_new_server(
        app_state.clone(),
        claims.user_id,
        payload,
    ));

    StatusCode::ACCEPTED.into_response()
}

async fn get_server(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
async fn delete_server(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
async fn server_action(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
