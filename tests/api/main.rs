mod auth_api;
mod user_api;

mod helpers {
    use axum::Router;
    use dashboard::prelude::AppState;
    use dashboard::web::{routes_login, routes_server};
    use reqwest::Client;
    use serde_json::Value;
    use sqlx::PgPool;
    use std::sync::Arc;
    use tokio::net::TcpListener;

    pub struct TestApp {
        pub url: String,
        #[allow(unused)]
        pub state: AppState,
        pub client: Client,
    }

    pub async fn spawn_app(pool: PgPool) -> TestApp {
        let state = AppState {
            pool,
            proxmox: Arc::new(MockProxmoxClient::default()),
        };
        let router = Router::new()
            .merge(routes_login::routes())
            .merge(routes_server::routes())
            .with_state(state.clone());

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        TestApp {
            url: format!("http://{}", address),
            state,
            client: Client::new(),
        }
    }

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
