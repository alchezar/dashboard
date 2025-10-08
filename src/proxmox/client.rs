#![allow(unused)]

use crate::error::ProxmoxError;
use crate::prelude::{Error, Result};
use crate::proxmox::Proxmox;
use crate::proxmox::types::*;
use async_trait::async_trait;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
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
    ///
    /// # Returns
    ///
    /// Deserialized data of type `T`.
    ///
    async fn get_data<T>(&self, url: &str) -> Result<T>
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
                Err(Error::Proxmox(ProxmoxError::Status, status, text))
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
        let new_id = self.get_data::<String>(&nextid_url).await?;

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
        let data = self.get_data::<StatusPayload>(&url).await?;
        Ok(data.status)
    }

    async fn task_status(&self, task: TaskRef) -> Result<TaskStatus> {
        let url = format!(
            "{}/nodes/{}/tasks/{}/status",
            self.url,
            task.node,
            task.upid.encoded()
        );
        let data = self.get_data::<TaskResponse>(&url).await?;
        Ok(match (data.status, data.exit_status.as_deref()) {
            (Status::Running, _) => TaskStatus::Pending,
            (Status::Stopped, Some("OK")) => TaskStatus::Completed,
            (Status::Stopped, Some(exit_status)) => TaskStatus::Failed(exit_status.to_owned()),
            (Status::Stopped, None) => TaskStatus::Failed("Unexpected".to_owned()),
        })
    }
}
