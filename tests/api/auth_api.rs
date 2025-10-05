use crate::helpers::{self, TestApp};
use dashboard::web::types::TokenResponse;
use sqlx::PgPool;

#[sqlx::test]
async fn should_register(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool).await;

    // Act
    let response = app
        .client
        .post(&format!("{}/register", &app.url))
        .json(&helpers::register_user_payload())
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<TokenResponse>().await.unwrap();
    assert!(!payload.result.token.is_empty());
}

#[sqlx::test]
async fn should_login(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool).await;
    app.client
        .post(&format!("{}/register", &app.url))
        .json(&helpers::register_user_payload())
        .send()
        .await
        .unwrap();

    // Act
    let response = app
        .client
        .post(&format!("{}/login", &app.url))
        .json(&helpers::login_user_payload())
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<TokenResponse>().await.unwrap();
    assert!(!payload.result.token.is_empty());
}
