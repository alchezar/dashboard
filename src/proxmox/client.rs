#![allow(unused)]

use async_trait::async_trait;
use crate::prelude::Error;
use crate::proxmox::Proxmox;
use crate::proxmox::types::{ProcessId, TaskStatus, VmOptions, VmRef, VmStatus};
use reqwest::Client;

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
}

#[async_trait]
impl Proxmox for ProxmoxClient {
    async fn create(&self, options: VmOptions) -> crate::error::Result<ProcessId> {
        Err(Error::Any("Unimplemented!".to_owned()))
    }

    async fn start(&self, vm: VmRef) -> crate::error::Result<ProcessId> {
        Err(Error::Any("Unimplemented!".to_owned()))
    }

    async fn stop(&self, vm: VmRef) -> crate::error::Result<ProcessId> {
        Err(Error::Any("Unimplemented!".to_owned()))
    }

    async fn reboot(&self, vm: VmRef) -> crate::error::Result<ProcessId> {
        Err(Error::Any("Unimplemented!".to_owned()))
    }

    async fn delete(&self, vm: VmRef) -> crate::error::Result<ProcessId> {
        Err(Error::Any("Unimplemented!".to_owned()))
    }

    async fn task_status(&self, vm: VmRef) -> crate::error::Result<TaskStatus> {
        Err(Error::Any("Unimplemented!".to_owned()))
    }

    async fn vm_status(&self, vm: VmRef) -> crate::error::Result<VmStatus> {
        Err(Error::Any("Unimplemented!".to_owned()))
    }
}
