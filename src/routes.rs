use axum::Router;
use axum::routing::{get, post};
use sqlx::PgPool;
use crate::services;

pub fn create_routes(pool: PgPool) -> Router {
    Router::new()
        .with_state(pool)
		.merge(create_auth_routes())
		.merge(create_server_routes())
}

pub fn create_auth_routes() -> Router {
	Router::new()
		.route("/login", post(login))
		.route("/users/me", get(get_user))
}

pub fn create_server_routes() -> Router {
	Router::new()
		.route("/servers", get(get_servers).post(new_server))
		.route("/servers/{id}", get(get_server).delete(delete_server))
		.route("/servers/{id}/actions", post(server_action))
}

async fn login() { services::login().await; }
async fn get_user() { services::get_user().await; }
async fn get_servers() { services::get_servers().await; }
async fn new_server() { services::new_server().await; }
async fn get_server() { services::get_server().await; }
async fn delete_server() { services::delete_server().await; }
async fn server_action() { services::server_action().await; }
