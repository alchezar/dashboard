//! Protected routes

use crate::prelude::{Controller, Result};
use crate::web::auth::Claims;
use crate::web::mw_auth;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Extension, Json}; // Add Extension
use axum::{Router, middleware};
use serde_json::{Value, json};

pub fn routes() -> Router<Controller> {
    Router::new()
        .route("/user/me", get(get_user))
        .route("/servers", get(get_servers).post(new_server))
        .route("/servers/{id}", get(get_server).delete(delete_server))
        .route("/servers/{id}/actions", post(server_action))
        .route_layer(middleware::from_fn(mw_auth::mw_require_auth))
}

async fn get_user(
    State(controller): State<Controller>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>> {
    let user_id = claims.user_id;
    let user = controller.get_user_by_id(user_id).await?;

    let body = Json(json!({
        "result": {
            "user_id": user_id,
        }
    }));

    Ok(body)
}

async fn get_servers(State(controller): State<Controller>, Extension(claims): Extension<Claims>) {}
async fn new_server(State(controller): State<Controller>, Extension(claims): Extension<Claims>) {}
async fn get_server(State(controller): State<Controller>, Extension(claims): Extension<Claims>) {}
async fn delete_server(State(controller): State<Controller>, Extension(claims): Extension<Claims>) {
}
async fn server_action(State(controller): State<Controller>, Extension(claims): Extension<Claims>) {
}
