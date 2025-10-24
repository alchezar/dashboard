use crate::helpers::{TestApp, payload, requests};
use dashboard_server::web::types::{TokenPayload, UserResponse};
use sqlx::PgPool;

#[sqlx::test]
async fn should_get_user(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool).await;
    let endpoint = format!("{}/register", &app.url);
    let payload = payload::register_user();
    let token = requests::post_result::<TokenPayload>(&app, &endpoint, &payload)
        .await
        .token;

    // Act
    let endpoint = format!("{}/user/me", &app.url);
    let response = requests::get_response(&app, &endpoint, &token).await;

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<UserResponse>().await.unwrap();
    assert_eq!(
        payload.result.email,
        payload::register_user()["email"].as_str().unwrap()
    );
}
