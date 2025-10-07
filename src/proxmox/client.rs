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

pub struct ProxmoxClient {
    client: Client,
    url: String,
    auth_header: String,
}

impl ProxmoxClient {
    pub fn new(url: String, auth_header: String) -> Self {
        Self {
            client: Client::new(),
            url,
            auth_header,
        }
    }

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

    async fn get_status_data<T>(&self, url: &str) -> Result<T>
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

    async fn create(&self, options: VmOptions) -> Result<UniqueProcessId> {
        unimplemented!()
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
        let data = self.get_status_data::<StatusResponse>(&url).await?;
        Ok(data.status)
    }

    async fn task_status(&self, task: TaskRef) -> Result<TaskStatus> {
        let url = format!(
            "{}/nodes/{}/tasks/{}/status",
            self.url,
            task.node,
            task.up_id.encoded()
        );
        let data = self.get_status_data::<TaskResponse>(&url).await?;
        Ok(match (data.status, data.exitstatus.as_deref()) {
            (Status::Running, _) => TaskStatus::Pending,
            (Status::Stopped, Some("OK")) => TaskStatus::Completed,
            (Status::Stopped, Some(exit_status)) => TaskStatus::Failed(exit_status.to_owned()),
            (Status::Stopped, None) => TaskStatus::Failed("Unexpected".to_owned()),
        })
    }
}
