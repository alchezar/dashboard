use crate::helpers::{self, spawn_app};
use dashboard::web::types::TokenResponse;
use sqlx::PgPool;

// curl --location 'http://127.0.0.1:8080/register' --header 'Content-Type: application/json' --data '{ "first_name": "John", "last_name": "Doe", "email": "john.doe@example.com", "password": "secure_password_123", "address": "123 Main St", "city": "Anytown", "state": "Any-state", "post_code": "12345", "country": "USA", "phone_number": "555-1234" }'
#[sqlx::test]
async fn should_register(pool: PgPool) {
    // Arrange
    let app = spawn_app(pool).await;

    // Act
    let response = app
        .client
        .post(&format!("{}/register", app.url))
        .json(&helpers::register_user_payload())
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<TokenResponse>().await.unwrap();
    assert!(!payload.result.token.is_empty());
    println!("{}", payload.result.token);
}

// curl --location 'http://127.0.0.1:8080/login' --header 'Content-Type: application/json' --data '{ "email": "john.doe@example.com", "password": "secure_password_123" }'
#[sqlx::test]
async fn should_login(pool: PgPool) {
    // Arrange
    let app = spawn_app(pool).await;
    app.client
        .post(&format!("{}/register", app.url))
        .json(&helpers::register_user_payload())
        .send()
        .await
        .unwrap();

    // Act
    let response = app
        .client
        .post(&format!("{}/login", app.url))
        .json(&helpers::login_user_payload())
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<TokenResponse>().await.unwrap();
    assert!(!payload.result.token.is_empty());
    println!("{}", payload.result.token);
}
