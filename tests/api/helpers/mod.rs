pub mod database;
pub mod payload;
pub mod requests;

// -------------------------------------------------------------------------

use async_trait::async_trait;
use dashboard::app::App;
use dashboard::model::queries;
use dashboard::model::types::ApiServer;
use dashboard::prelude::{AppState, Proxmox, Result};
use dashboard::proxmox::types::{Status, TaskRef, TaskStatus, UniqueProcessId, VmConfig, VmRef};
use dashboard::web::types::TokenPayload;
use reqwest::Client;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// Test helper that runs a server instance in the background and provides a
/// `reqwest::Client` for making API calls.
///
pub struct TestApp {
    pub url: String,
    pub client: Client,
}

impl TestApp {
    /// Creates a new `TestApp`.
    ///
    /// # Arguments
    ///
    /// * `pool`: Test pool provided by the `#[sqlx::test]` macro.
    ///
    pub async fn new(pool: PgPool) -> Self {
        // Create testable application instance.
        let proxmox = Arc::new(MockProxmoxClient::default());
        let state = AppState { pool, proxmox };
        let application = App::build(state, "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let url = application.get_url().unwrap();

        // Spawn application without blocking the execution.
        tokio::spawn(async move {
            application.run().await.unwrap();
        });

        TestApp {
            url,
            client: Client::new(),
        }
    }
}

/// Test helper that creates and holds base default data for the database.
///
pub struct TestData {
    pub token: String,
    pub user_id: Uuid,
    pub product_id: Uuid,
}

impl TestData {
    /// Creates new user and product.
    ///
    pub async fn new(app: &TestApp, pool: &PgPool) -> TestData {
        let endpoint = format!("{}/register", &app.url);
        let register_payload = payload::register_user();
        let token = requests::post_result::<TokenPayload>(&app, &endpoint, &register_payload)
            .await
            .token;
        let user_id =
            queries::get_user_by_email(&pool, register_payload["email"].as_str().unwrap())
                .await
                .unwrap()
                .id;

        let product_id = database::populate_product(&pool).await;

        TestData {
            token,
            user_id,
            product_id,
        }
    }

    /// Creates new server for the user.
    ///
    pub async fn create_server(
        &self,
        app: &TestApp,
        pool: &PgPool,
    ) -> (reqwest::Response, ApiServer) {
        let endpoint = format!("{}/servers", &app.url);
        let payload = payload::new_server(self.product_id);
        let response = requests::post_response(&app, &endpoint, &self.token, &payload).await;

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let servers = queries::get_servers_for_user(&pool, self.user_id)
            .await
            .unwrap();

        (response, servers.first().unwrap().clone())
    }
}

// -------------------------------------------------------------------------

/// Mock Proxmox client for testing.
///
#[derive(Default)]
pub struct MockProxmoxClient;

#[async_trait]
impl Proxmox for MockProxmoxClient {
    async fn start(&self, _vm: VmRef) -> Result<UniqueProcessId> {
        Ok("mock_process_id".into())
    }
    async fn shutdown(&self, _vm: VmRef) -> Result<UniqueProcessId> {
        Ok("mock_process_id".into())
    }
    async fn stop(&self, _vm: VmRef) -> Result<UniqueProcessId> {
        Ok("mock_process_id".into())
    }
    async fn reboot(&self, _vm: VmRef) -> Result<UniqueProcessId> {
        Ok("mock_process_id".into())
    }
    async fn create(&self, _vm: VmRef) -> Result<(i32, UniqueProcessId)> {
        Ok((101, "mock_process_id".into()))
    }
    async fn delete(&self, _vm: VmRef) -> Result<UniqueProcessId> {
        Ok("mock_process_id".into())
    }
    async fn vm_config(&self, _vm: VmRef, _config: VmConfig) -> Result<UniqueProcessId> {
        Ok("mock_process_id".into())
    }
    async fn vm_status(&self, _vm: VmRef) -> Result<Status> {
        Ok(Status::Running)
    }
    async fn task_status(&self, _task: &TaskRef) -> Result<TaskStatus> {
        Ok(TaskStatus::Completed)
    }
}
