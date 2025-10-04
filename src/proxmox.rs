pub mod client;
pub mod types;

// -----------------------------------------------------------------------------

use crate::prelude::Result;
use crate::proxmox::types::*;
use async_trait::async_trait;

#[async_trait]
pub trait Proxmox {
    async fn create(&self, options: VmOptions) -> Result<ProcessId>;
    async fn start(&self, vm: VmRef) -> Result<ProcessId>;
    async fn stop(&self, vm: VmRef) -> Result<ProcessId>;
    async fn reboot(&self, vm: VmRef) -> Result<ProcessId>;
    async fn delete(&self, vm: VmRef) -> Result<ProcessId>;
    async fn task_status(&self, vm: VmRef) -> Result<TaskStatus>;
    async fn vm_status(&self, vm: VmRef) -> Result<VmStatus>;
}
