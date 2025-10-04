use crate::helpers::{self, spawn_app};
use dashboard::web::types::{TokenResponse, UserResponse};
use sqlx::PgPool;

// curl --location 'http://127.0.0.1:8080/users/me' --header 'Authorization: Bearer <TOKEN>'
#[sqlx::test]
async fn should_get_user(pool: PgPool) {
    // Arrange
    let app = spawn_app(pool).await;
    let token = app
        .client
        .post(&format!("{}/register", app.url))
        .json(&helpers::register_user_payload())
        .send()
        .await
        .unwrap()
        .json::<TokenResponse>()
        .await
        .unwrap()
        .result
        .token;

    // Act
    let response = app
        .client
        .get(&format!("{}/user/me", app.url))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();

    // Assert
    assert!(response.status().is_success());
    let payload = response.json::<UserResponse>().await.unwrap();
    assert_eq!(payload.result.email, "john.doe.reqwest@example.com");
}
