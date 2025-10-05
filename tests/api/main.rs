mod auth_api;
mod user_api;

// -----------------------------------------------------------------------------

mod helpers {
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
    use dashboard::app::App;
    use dashboard::prelude::Result;
    use dashboard::proxmox::Proxmox;
    use dashboard::proxmox::types::{ProcessId, TaskStatus, VmOptions, VmRef, VmStatus};

    /// Mock Proxmox client for testing.
    ///
    #[derive(Default)]
    pub struct MockProxmoxClient;

    #[async_trait]
    impl Proxmox for MockProxmoxClient {
        async fn create(&self, _options: VmOptions) -> Result<ProcessId> {
            Ok("mock_process_id".to_string())
        }
        async fn start(&self, _vm: VmRef) -> Result<ProcessId> {
            Ok("mock_process_id".to_string())
        }
        async fn stop(&self, _vm: VmRef) -> Result<ProcessId> {
            Ok("mock_process_id".to_string())
        }
        async fn reboot(&self, _vm: VmRef) -> Result<ProcessId> {
            Ok("mock_process_id".to_string())
        }
        async fn delete(&self, _vm: VmRef) -> Result<ProcessId> {
            Ok("mock_process_id".to_string())
        }
        async fn task_status(&self, _vm: VmRef) -> Result<TaskStatus> {
            Ok(TaskStatus::Completed)
        }
        async fn vm_status(&self, _vm: VmRef) -> Result<VmStatus> {
            Ok(VmStatus::Running)
        }
    }
}
