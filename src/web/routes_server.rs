//! Protected routes
#![allow(unused)]

use crate::model::queries;
use crate::prelude::AppState;
use crate::prelude::Result;
use crate::web::auth::Claims;
use crate::web::mw_auth;
use crate::web::types::{Response, UserResponse};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Extension, Json};
use axum::{Router, middleware};
use serde_json::{Value, json};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/user/me", get(get_user))
        .route("/servers", get(get_servers).post(new_server))
        .route("/servers/{id}", get(get_server).delete(delete_server))
        .route("/servers/{id}/actions", post(server_action))
        .route_layer(middleware::from_fn(mw_auth::mw_require_auth))
}

async fn get_user(
    State(app_state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserResponse>> {
    let user = queries::get_user_by_id(&app_state.pool, claims.user_id).await?;

    Ok(Json(Response::new(user)))
}

async fn get_servers(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
async fn new_server(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
async fn get_server(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
async fn delete_server(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
async fn server_action(State(app_state): State<AppState>, Extension(claims): Extension<Claims>) {}
