use crate::prelude::{Error, Proxmox, ProxmoxError, Result};
use crate::proxmox::types::*;
use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use std::collections::HashMap;

/// Concrete implementation of the `Proxmox` trait using `reqwest` crate.
///
/// Translates the abstract operations defined in the `Proxmox` trait into
/// actual HTTP API calls and manages the state required to communicate with a
/// Proxmox VE server.
///
pub struct ProxmoxClient {
    client: Client,
    url: String,
    auth_header: String,
}

impl ProxmoxClient {
    /// Creates a new instance of the Proxmox client.
    ///
    /// # Arguments
    ///
    /// * `url`: URL of the Proxmox API.
    /// * `auth_header`: The full, pre-formatted authorization header string.
    ///
    pub fn new(url: String, auth_header: String) -> Self {
        Self {
            client: Client::new(),
            url,
            auth_header,
        }
    }

    /// Helper method to execute a `POST` command that returns a task UPID.
    ///
    /// # Arguments
    ///
    /// * `url`: Full URL of the Proxmox API endpoint to call.
    /// * `error_var`: specific `ProxmoxError` variant to use if the API call
    ///   fails.
    ///
    /// # Returns
    ///
    /// `UPID` of the created task.
    ///
    async fn execute_command(&self, url: &str, error_var: ProxmoxError) -> Result<UniqueProcessId> {
        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?;
        match response.status() {
            status if status.is_success() => {
                Ok(response.json::<Response<UniqueProcessId>>().await?.data)
            }
            status => {
                let text = response.text().await?;
                Err(Error::Proxmox(error_var, status, text))
            }
        }
    }

    /// Generic helper method to perform a `GET` request and deserialize the
    /// response data.
    ///
    /// # Types
    ///
    /// * `T`: Target type to deserialize the JSON data into.
    ///
    /// # Arguments
    ///
    /// * `url`: Full URL of the Proxmox API endpoint to call.
    /// * `error_var`: specific `ProxmoxError` variant to use if the API call
    ///   fails.
    ///
    /// # Returns
    ///
    /// Deserialized data of type `T`.
    ///
    async fn get_data<T>(&self, url: &str, error_var: ProxmoxError) -> Result<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?;
        match response.status() {
            status if status.is_success() => Ok(response.json::<Response<T>>().await?.data),
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
        let url = format!("{}/nodes/{}/qemu/{}/status/start", self.url, vm.node, vm.id);
        self.execute_command(&url, ProxmoxError::Start).await
    }

    async fn shutdown(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let url = format!(
            "{}/nodes/{}/qemu/{}/status/shutdown",
            self.url, vm.node, vm.id
        );
        self.execute_command(&url, ProxmoxError::Shutdown).await
    }

    async fn stop(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let url = format!("{}/nodes/{}/qemu/{}/status/stop", self.url, vm.node, vm.id);
        self.execute_command(&url, ProxmoxError::Stop).await
    }

    async fn reboot(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let url = format!(
            "{}/nodes/{}/qemu/{}/status/reboot",
            self.url, vm.node, vm.id
        );
        self.execute_command(&url, ProxmoxError::Reboot).await
    }

    async fn create(&self, template_vm: VmRef) -> Result<UniqueProcessId> {
        // Get next free VMID.
        let nextid_url = format!("{}/cluster/nextid", self.url);
        let new_id = self
            .get_data::<String>(&nextid_url, ProxmoxError::Create)
            .await?;

        // Create a copy of virtual machine/template.
        let url = format!(
            "{}/nodes/{}/qemu/{}/clone",
            self.url, template_vm.node, template_vm.id
        );
        let params = HashMap::from([("newid", new_id); 1]);
        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .form(&params)
            .send()
            .await?;
        match response.status() {
            status if status.is_success() => {
                Ok(response.json::<Response<UniqueProcessId>>().await?.data)
            }
            status => {
                let text = response.text().await?;
                Err(Error::Proxmox(ProxmoxError::Create, status, text))
            }
        }
    }

    async fn delete(&self, vm: VmRef) -> Result<UniqueProcessId> {
        let url = format!("{}/nodes/{}/qemu/{}", self.url, vm.node, vm.id);
        let response = self
            .client
            .delete(&url)
            .header(AUTHORIZATION, &self.auth_header)
            .send()
            .await?;
        match response.status() {
            status if status.is_success() => {
                Ok(response.json::<Response<UniqueProcessId>>().await?.data)
            }
            status => {
                let text = response.text().await?;
                Err(Error::Proxmox(ProxmoxError::Delete, status, text))
            }
        }
    }

    async fn vm_status(&self, vm: VmRef) -> Result<Status> {
        let url = format!(
            "{}/nodes/{}/qemu/{}/status/current",
            self.url, vm.node, vm.id
        );
        let data = self
            .get_data::<StatusPayload>(&url, ProxmoxError::Status)
            .await?;
        Ok(data.status)
    }

    async fn task_status(&self, task: TaskRef) -> Result<TaskStatus> {
        let url = format!(
            "{}/nodes/{}/tasks/{}/status",
            self.url,
            task.node,
            task.upid.encoded()
        );
        let data = self
            .get_data::<TaskResponse>(&url, ProxmoxError::Status)
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup() -> (MockServer, ProxmoxClient) {
        let mock_server = MockServer::start().await;
        let client = ProxmoxClient::new(mock_server.uri(), "PVEAPIToken=".into());

        (mock_server, client)
    }

    #[tokio::test]
    async fn start_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let response_json = json!({"data": expected_upid});
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/start"))
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
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/start"))
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
        let expected_upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let response_json = json!({"data": expected_upid});
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/shutdown"))
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
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/shutdown"))
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
        let expected_upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let response_json = json!({"data": expected_upid});
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/stop"))
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
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/stop"))
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
        let expected_upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let response_json = json!({"data": expected_upid});
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/reboot"))
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
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/status/reboot"))
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
    async fn create_vm_success() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_vmid = "101";
        let response_vmid_json = json!({"data": expected_vmid});
        let expected_upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let response_upid_json = json!({"data": expected_upid});
        Mock::given(method("GET"))
            .and(path("/cluster/nextid"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_vmid_json))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/clone"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_upid_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.create(VmRef::new("pve", 100)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().into_inner(), expected_upid);
    }

    #[tokio::test]
    async fn create_vm_failure_second() {
        // Arrange
        let (mock_server, client) = setup().await;
        let expected_vmid = "101";
        let response_vmid_json = json!({"data": expected_vmid});
        Mock::given(method("GET"))
            .and(path("/cluster/nextid"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_vmid_json))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/nodes/pve/qemu/100/clone"))
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
    async fn create_vm_failure_first() {
        // Arrange
        let (mock_server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/cluster/nextid"))
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
        let expected_upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let response_json = json!({"data": expected_upid});
        Mock::given(method("DELETE"))
            .and(path("/nodes/pve/qemu/100"))
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
        Mock::given(method("DELETE"))
            .and(path("/nodes/pve/qemu/100"))
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
        Mock::given(method("GET"))
            .and(path("/nodes/pve/qemu/100/status/current"))
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
        Mock::given(method("GET"))
            .and(path("/nodes/pve/qemu/100/status/current"))
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
        let upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let encoded_upid = UniqueProcessId::from(upid).encoded();
        let response_json = json!({"data": {"status": "running"}});
        Mock::given(method("GET"))
            .and(path(format!("/nodes/pve/tasks/{}/status", encoded_upid)))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(TaskRef::new("pve", upid)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskStatus::Pending);
    }

    #[tokio::test]
    async fn task_status_completed() {
        // Arrange
        let (mock_server, client) = setup().await;
        let upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let encoded_upid = UniqueProcessId::from(upid).encoded();
        let response_json = json!({"data": {"status": "stopped", "exitstatus": "OK"}});
        Mock::given(method("GET"))
            .and(path(format!("/nodes/pve/tasks/{}/status", encoded_upid)))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(TaskRef::new("pve", upid)).await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TaskStatus::Completed);
    }

    #[tokio::test]
    async fn task_status_failed() {
        // Arrange
        let (mock_server, client) = setup().await;
        let upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let encoded_upid = UniqueProcessId::from(upid).encoded();
        let response_json =
            json!({"data": {"status": "stopped", "exitstatus": "ERROR: command failed"}});
        Mock::given(method("GET"))
            .and(path(format!("/nodes/pve/tasks/{}/status", encoded_upid)))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_json))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(TaskRef::new("pve", upid)).await;

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
        let upid = "UPID:pve:12345678:90ABCDEF:12345678:type:100:id@realm:";
        let encoded_upid = UniqueProcessId::from(upid).encoded();
        Mock::given(method("GET"))
            .and(path(format!("/nodes/pve/tasks/{}/status", encoded_upid)))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        // Act
        let result = client.task_status(TaskRef::new("pve", upid)).await;

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
