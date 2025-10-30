use crate::proxmox::Proxmox;
use crate::proxmox::types::*;
use async_trait::async_trait;
use dashboard_common::prelude::{Error, ProxmoxError, Result};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{Client, Method};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::OnceCell;

/// Concrete implementation of the `Proxmox` trait using `reqwest` crate.
///
/// Translates the abstract operations defined in the `Proxmox` trait into
/// actual HTTP API calls and manages the state required to communicate with a
/// Proxmox VE server.
///
pub struct ProxmoxClient {
    client: OnceCell<Client>,
    url: String,
    auth_header: SecretString,
}

impl ProxmoxClient {
    /// Creates a new instance of the Proxmox client.
    ///
    /// # Arguments
    ///
    /// * `url`: URL of the Proxmox API.
    /// * `auth_header`: The full, pre-formatted authorization header string.
    ///
    pub fn new(url: String, auth_header: SecretString) -> Result<Self> {
        Ok(Self {
            client: OnceCell::new(),
            url,
            auth_header,
        })
    }

    /// Lazily initializes and returns a reference to the `reqwest::Client`.
    ///
    /// If the client has not been initialized yet, it will be built on the
    /// first call with default headers (including Authorization). Subsequent
    /// calls will return the existing client.
    ///
    async fn get_client(&self) -> Result<&Client> {
        self.client
            .get_or_try_init(|| async {
                let mut auth_header = HeaderValue::from_str(self.auth_header.expose_secret())?;
                auth_header.set_sensitive(true);

                let mut headers = HeaderMap::new();
                headers.insert(AUTHORIZATION, auth_header);

                Client::builder()
                    .default_headers(headers)
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true)
                    .use_rustls_tls()
                    .tls_built_in_root_certs(false)
                    .min_tls_version(reqwest::tls::Version::TLS_1_0)
                    .build()
                    .map_err(Error::from)
            })
            .await
    }

    /// Generic helper method to perform a request to the Proxmox API.
    ///
    /// Handles client initialization, request building, sending the request,
    /// and processing the response.
    ///
    /// # Types
    ///
    /// * `B`: Type of the request body, which must be serializable.
    /// * `D`: Type of the response data, which must be deserializable.
    ///
    /// # Arguments
    ///
    /// * `method`: HTTP method to use for the request.
    /// * `path`: API endpoint path.
    /// * `body`: Optional request body.
    /// * `error_var`: Specific error to use if the API call fails.
    ///
    /// # Returns
    ///
    /// Deserialized data from the Proxmox API response.
    ///
    async fn make_request<B, D>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
        error_var: ProxmoxError,
    ) -> Result<D>
    where
        B: Default + Serialize,
        for<'de> D: Deserialize<'de>,
    {
        let client = self.get_client().await?;
        let url = format!("{}{}", self.url, path);

        let response = client
            .request(method, &url)
            .form(&body.unwrap_or_default())
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => Ok(response.json::<Response<D>>().await?.data),
            status => {
                let text = response.text().await?;
                Err(Error::Proxmox(error_var, status, text))
            }
        }
    }
}

#[async_trait]
impl Proxmox for ProxmoxClient {
    async fn start(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let path = format!("/nodes/{}/qemu/{}/status/start", vm.node, vm.id);
        self.make_request(Method::POST, &path, None::<()>, ProxmoxError::Start)
            .await
    }

    async fn shutdown(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let path = format!("/nodes/{}/qemu/{}/status/shutdown", vm.node, vm.id);
        self.make_request(Method::POST, &path, None::<()>, ProxmoxError::Shutdown)
            .await
    }

    async fn stop(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let path = format!("/nodes/{}/qemu/{}/status/stop", vm.node, vm.id);
        self.make_request(Method::POST, &path, None::<()>, ProxmoxError::Stop)
            .await
    }

    async fn reboot(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let path = format!("/nodes/{}/qemu/{}/status/reboot", vm.node, vm.id);
        self.make_request(Method::POST, &path, None::<()>, ProxmoxError::Reboot)
            .await
    }

    async fn create(&self, template_vm: VmRef) -> Result<(i32, UniqueProcessId)> {
        // Get next free VMID.
        let new_id_str: String = self
            .make_request(
                Method::GET,
                "/cluster/nextid",
                None::<()>,
                ProxmoxError::Create,
            )
            .await?;
        let new_id: i32 = new_id_str.parse()?;

        // Create a copy of virtual machine/template.
        let path = format!("/nodes/{}/qemu/{}/clone", template_vm.node, template_vm.id);
        let params = HashMap::from([("newid", new_id)]);
        let upid: UniqueProcessId = self
            .make_request(Method::POST, &path, Some(params), ProxmoxError::Create)
            .await?;

        Ok((new_id, upid))
    }

    async fn delete(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let path = format!("/nodes/{}/qemu/{}", vm.node, vm.id);
        self.make_request(Method::DELETE, &path, None::<()>, ProxmoxError::Delete)
            .await
    }

    async fn vm_config(&self, vm: VmRef, config: VmConfig) -> Result<UniqueProcessId> {
        let path = format!("/nodes/{}/qemu/{}/config", vm.node, vm.id);
        self.make_request(Method::POST, &path, Some(config), ProxmoxError::Create)
            .await
    }

    async fn vm_status(&self, vm: VmRef) -> Result<Status> {
        let path = format!("/nodes/{}/qemu/{}/status/current", vm.node, vm.id);
        let payload: StatusPayload = self
            .make_request(Method::GET, &path, None::<()>, ProxmoxError::Status)
            .await?;
        Ok(payload.status)
    }

    async fn task_status(&self, task: &TaskRef) -> Result<TaskStatus> {
        let path = format!("/nodes/{}/tasks/{}/status", task.node, task.upid.encoded());
        let data: TaskResponse = self
            .make_request(Method::GET, &path, None::<()>, ProxmoxError::Status)
            .await?;
        Ok(match (data.status, data.exit_status.as_deref()) {
            (Status::Running, _) => TaskStatus::Pending,
            (Status::Stopped, Some("OK")) => TaskStatus::Completed,
            (Status::Stopped, Some(exit_status)) => TaskStatus::Failed(exit_status.to_owned()),
            (Status::Stopped, None) => TaskStatus::Failed("Unexpected".to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxmox::types::{TaskRef, VmRef};
    use axum::http::StatusCode;
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const FAKE_UPID: &str = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
    const AUTH_TOKEN: &str = "PVEAPIToken=test@pve!token=uuid";

    async fn setup() -> (MockServer, ProxmoxClient) {
        let mock_server = MockServer::start().await;
        let client = ProxmoxClient::new(mock_server.uri(), AUTH_TOKEN.into()).unwrap();

        (mock_server, client)
    }

    #[tokio::test]
    async fn start_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_upid = FAKE_UPID;
        let response_json = json!({"data": expected_upid});
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/start"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.start(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), expected_upid);
    }

    #[tokio::test]
    async fn start_vm_failure() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/start"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.start(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Start, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn shutdown_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_upid = FAKE_UPID;
        let response_json = json!({"data": expected_upid});
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/shutdown"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.shutdown(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), expected_upid);
    }

    #[tokio::test]
    async fn shutdown_vm_failure() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/shutdown"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.shutdown(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Shutdown, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn stop_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_upid = FAKE_UPID;
        let response_json = json!({"data": expected_upid});
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/stop"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.stop(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), expected_upid);
    }

    #[tokio::test]
    async fn stop_vm_failure() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/stop"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.stop(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Stop, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn reboot_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_upid = FAKE_UPID;
        let response_json = json!({"data": expected_upid});
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/reboot"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.reboot(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), expected_upid);
    }

    #[tokio::test]
    async fn reboot_vm_failure() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/status/reboot"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.reboot(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Reboot, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn clone_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let response_vmid_json = json!({"data": "101"});
        let response_upid_json = json!({"data": FAKE_UPID});
        Mock::given(method(Method::GET))
            .and(path("/cluster/nextid"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_vmid_json))
            .mount(&mock_server)
            .await;
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/clone"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_upid_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.create(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        let (_vmid, upid) = result.unwrap();
        assert_eq!(upid.into_inner(), FAKE_UPID);
    }

    #[tokio::test]
    async fn clone_vm_failure_second() {
        // Arrange
        let (mock_server, client) = setup().await;
        let response_vmid_json = json!({"data": "101"});
        Mock::given(method(Method::GET))
            .and(path("/cluster/nextid"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_vmid_json))
            .mount(&mock_server)
            .await;
        Mock::given(method(Method::POST))
            .and(path("/nodes/pve/qemu/100/clone"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.create(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Create, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn clone_vm_failure_first() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method(Method::GET))
            .and(path("/cluster/nextid"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.create(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Create, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn delete_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_upid = FAKE_UPID;
        let response_json = json!({"data": expected_upid});
        Mock::given(method(Method::DELETE))
            .and(path("/nodes/pve/qemu/100"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.delete(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), expected_upid);
    }

    #[tokio::test]
    async fn delete_vm_failure() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method(Method::DELETE))
            .and(path("/nodes/pve/qemu/100"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.delete(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Delete, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn vm_status_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let response_json = json!({"data": {"status": "running"}});
        Mock::given(method(Method::GET))
            .and(path("/nodes/pve/qemu/100/status/current"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.vm_status(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Status::Running);
    }

    #[tokio::test]
    async fn vm_status_failure() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method(Method::GET))
            .and(path("/nodes/pve/qemu/100/status/current"))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.vm_status(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Status, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }

    #[tokio::test]
    async fn task_status_pending() {
        // Arrange
        let (mock_server, client) = setup().await;
        let upid = UniqueProcessId::from(FAKE_UPID);
        let response_json = json!({"data": {"status": "running"}});
        Mock::given(method(Method::GET))
            .and(path(format!("/nodes/pve/tasks/{}/status", upid.encoded())))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(&TaskRef::new("pve", &upid)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskStatus::Pending);
    }

    #[tokio::test]
    async fn task_status_completed() {
        // Arrange
        let (mock_server, client) = setup().await;
        let upid = UniqueProcessId::from(FAKE_UPID);
        let response_json = json!({"data": {"status": "stopped", "exitstatus": "OK"}});
        Mock::given(method(Method::GET))
            .and(path(format!("/nodes/pve/tasks/{}/status", upid.encoded())))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(&TaskRef::new("pve", &upid)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskStatus::Completed);
    }

    #[tokio::test]
    async fn task_status_failed() {
        // Arrange
        let (mock_server, client) = setup().await;
        let upid = UniqueProcessId::from(FAKE_UPID);
        let response_json =
            json!({"data": {"status": "stopped", "exitstatus": "ERROR: command failed"}});
        Mock::given(method(Method::GET))
            .and(path(format!("/nodes/pve/tasks/{}/status", upid.encoded())))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(&TaskRef::new("pve", &upid)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            TaskStatus::Failed("ERROR: command failed".to_owned())
        );
    }

    #[tokio::test]
    async fn task_status_failure() {
        // Arrange
        let (mock_server, client) = setup().await;
        let upid = UniqueProcessId::from(FAKE_UPID);
        Mock::given(method(Method::GET))
            .and(path(format!("/nodes/pve/tasks/{}/status", upid.encoded())))
            .and(header(AUTHORIZATION.as_str(), AUTH_TOKEN))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(&TaskRef::new("pve", &upid)).await;

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::Proxmox(ProxmoxError::Status, status, text) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(text, "Internal Server Error");
            }
            error => panic!("unexpected error: {}", error),
        }
    }
}
