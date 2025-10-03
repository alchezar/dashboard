use crate::helpers::{self, spawn_app};
use dashboard::web::types::{Response, TokenPayload};
use sqlx::PgPool;

// curl --location 'http://127.0.0.1:8080/register' --header 'Content-Type: application/json' --data '{ "first_name": "John", "last_name": "Doe", "email": "john.doe@example.com", "password": "secure_password_123", "address": "123 Main St", "city": "Anytown", "state": "Any-state", "post_code": "12345", "country": "USA", "phone_number": "555-1234" }'
#[sqlx::test]
async fn should_register(pool: PgPool) {
    // Arrange
    let app = spawn_app(pool).await;
    let endpoint = format!("{}/register", app.url);
    let user_payload = helpers::register_user_payload();

    // Act
    let response = app
        .client
        .post(&endpoint)
        .json(&user_payload)
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<Response<TokenPayload>>().await.unwrap();
    assert!(!payload.result.token.is_empty());
    println!("{}", payload.result.token);
}

// curl --location 'http://127.0.0.1:8080/login' --header 'Content-Type: application/json' --data '{ "email": "john.doe@example.com", "password": "secure_password_123" }'
#[sqlx::test]
async fn should_login(pool: PgPool) {
    // Arrange
    let app = spawn_app(pool).await;
    let register_endpoint = format!("{}/register", app.url);
    let login_endpoint = format!("{}/login", app.url);
    app.client
        .post(&register_endpoint)
        .json(&helpers::register_user_payload())
        .send()
        .await
        .unwrap();

    // Act
    let response = app
        .client
        .post(&login_endpoint)
        .json(&helpers::login_user_payload())
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<Response<TokenPayload>>().await.unwrap();
    assert!(!payload.result.token.is_empty());
    println!("{}", payload.result.token);
}
