use crate::prelude::Controller;
use axum::Router;
use axum::extract::State;
use axum::routing::{get, post};

pub fn routes() -> Router {
    Router::new()
        .route("/servers", get(get_servers).post(new_server))
        .route("/servers/{id}", get(get_server).delete(delete_server))
        .route("/servers/{id}/actions", post(server_action))
}

async fn get_servers(State(controller): State<Controller>) {}
async fn new_server(State(controller): State<Controller>) {}
async fn get_server(State(controller): State<Controller>) {}
async fn delete_server(State(controller): State<Controller>) {}
async fn server_action(State(controller): State<Controller>) {}
