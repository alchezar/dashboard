use axum::Router;
use dashboard::web::{routes_login, routes_server};
use reqwest::Client;
use serde_json::Value;
use sqlx::PgPool;
use tokio::net::TcpListener;

#[allow(unused)]
pub struct TestApp {
    pub url: String,
    pub pool: PgPool,
    pub client: Client,
}

pub async fn spawn_app(pool: PgPool) -> TestApp {
    let router = Router::new()
        .merge(routes_login::routes())
        .merge(routes_server::routes())
        .with_state(pool.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    TestApp {
        url: format!("http://{}", address),
        pool,
        client: Client::new(),
    }
}

pub fn register_user_payload() -> Value {
    serde_json::json!({
      "first_name": "John",
      "last_name": "Doe",
      "email": "john.doe.reqwest@example.com",
      "password": "secure_password_123",
      "address": "123 Main St",
      "city": "Anytown",
      "state": "Any-state",
      "post_code": "12345",
      "country": "USA",
      "phone_number": "555-1234"
    })
}

pub fn login_user_payload() -> Value {
    serde_json::json!({
      "email": "john.doe.reqwest@example.com",
      "password": "secure_password_123",
    })
}
