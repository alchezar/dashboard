mod auth_api;
mod user_api;

// -----------------------------------------------------------------------------

mod helpers {
    use dashboard::app::App;
    use dashboard::prelude::AppState;
    use reqwest::Client;
    use serde_json::Value;
    use sqlx::PgPool;
    use std::sync::Arc;

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

    // -------------------------------------------------------------------------

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

    // -------------------------------------------------------------------------

    use async_trait::async_trait;
    use dashboard::prelude::Result;
    use dashboard::proxmox::Proxmox;
    use dashboard::proxmox::types::{Status, TaskRef, TaskStatus, UniqueProcessId, VmConfig, VmRef};

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
            Ok("mock_process_id".into())
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
}
