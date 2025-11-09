use crate::helpers::{TestApp, TestData, payload, requests};
use axum::http::StatusCode;
use dashboard_server::model::queries;
use dashboard_server::model::types::{ApiServer, ServerStatus};
use dashboard_server::web::types::{Response, TokenPayload};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn server_list_for_new_user_should_be_empty(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool).await;
    let endpoint = format!("{}/register", &app.url);
    let payload = payload::register_user();
    let token = requests::post_result::<TokenPayload>(&app, &endpoint, &payload)
        .await
        .token;

    // Act
    let endpoint = format!("{}/servers", &app.url);
    let response = requests::get_response(&app, &endpoint, &token).await;

    // Assert
    assert_eq!(response.status(), StatusCode::OK);
    let response = response
        .json::<Response<Vec<ApiServer>>>()
        .await
        .unwrap()
        .result;
    assert!(response.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_server_should_works(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool.clone()).await;
    let data = TestData::new(&app, &pool).await;

    // Act
    let (response, server) = data.create_server(&app, &pool).await;

    // Assert
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(server.status, ServerStatus::Stopped);
    assert_eq!(server.ip_address, "192.168.0.100");
    assert_eq!(server.vm_id.unwrap(), 101);
}

#[sqlx::test(migrations = "../../migrations")]
async fn get_server_should_works(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool.clone()).await;
    let data = TestData::new(&app, &pool).await;
    let (_, server) = data.create_server(&app, &pool).await;
    let server_id = server.server_id;

    // Act
    let endpoint = format!("{}/servers/{}", &app.url, server_id);
    let response = requests::get_response(&app, &endpoint, &data.token).await;
    let response_status = response.status();
    let server = response.json::<Response<ApiServer>>().await.unwrap().result;

    // Assert
    assert_eq!(response_status, StatusCode::OK);
    assert_eq!(server.server_id, server_id);
    assert_eq!(server.status, ServerStatus::Stopped);
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_servers_should_works(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool.clone()).await;
    let data = TestData::new(&app, &pool).await;
    let (_, server) = data.create_server(&app, &pool).await;

    // Act
    let endpoint = format!("{}/servers", &app.url);
    let response = requests::get_response(&app, &endpoint, &data.token).await;
    let response_status = response.status();
    let servers = response
        .json::<Response<Vec<ApiServer>>>()
        .await
        .unwrap()
        .result;

    // Assert
    assert_eq!(response_status, StatusCode::OK);
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].server_id, server.server_id);
    assert_eq!(servers[0].ip_address, server.ip_address);
}

#[sqlx::test(migrations = "../../migrations")]
async fn server_action_should_works(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool.clone()).await;
    let data = TestData::new(&app, &pool).await;
    let (_, server) = data.create_server(&app, &pool).await;
    let server_id = server.server_id;
    let status_before = server.status;

    // Act
    let action_payload = json!({ "action": "start" });
    let endpoint = format!("{}/servers/{}/actions", &app.url, server_id);
    let response = requests::post_response(&app, &endpoint, &data.token, &action_payload).await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let status_after = queries::get_servers_for_user(&pool, data.user_id)
        .await
        .unwrap()
        .first()
        .unwrap()
        .status;

    // Assert
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(status_before, ServerStatus::Stopped);
    assert_eq!(status_after, ServerStatus::Running);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_server_should_works(pool: PgPool) {
    // Arrange
    let app = TestApp::new(pool.clone()).await;
    let data = TestData::new(&app, &pool).await;
    let (_, server) = data.create_server(&app, &pool).await;
    let server_id = server.server_id;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let servers_before = queries::get_servers_for_user(&pool, data.user_id)
        .await
        .unwrap();

    // Act
    let endpoint = format!("{}/servers/{}", &app.url, server_id);
    let response = requests::delete_response(&app, &endpoint, &data.token).await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let servers_after = queries::get_servers_for_user(&pool, data.user_id)
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(!servers_before.is_empty());
    assert!(servers_after.is_empty());
}
