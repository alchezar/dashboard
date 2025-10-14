use crate::helpers::{TestApp, payload, requests};
use dashboard::web::types::TokenResponse;
use sqlx::PgPool;

#[sqlx::test]
async fn should_register(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool).await;

    // Act
    let endpoint = format!("{}/register", &app.url);
    let payload = payload::register_user();
    let response = requests::post_response(&app, &endpoint, "", &payload).await;

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<TokenResponse>().await.unwrap();
    assert!(!payload.result.token.is_empty());
}

#[sqlx::test]
async fn should_login(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool).await;
    let endpoint = format!("{}/register", &app.url);
    let payload = payload::register_user();
    requests::post_response(&app, &endpoint, "", &payload).await;

    // Act
    let endpoint = format!("{}/login", &app.url);
    let payload = payload::login_user();
    let response = requests::post_response(&app, &endpoint, "", &payload).await;

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<TokenResponse>().await.unwrap();
    assert!(!payload.result.token.is_empty());
}
