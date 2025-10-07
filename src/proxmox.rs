pub mod client;
pub mod types;

// -----------------------------------------------------------------------------

use crate::prelude::Result;
use crate::proxmox::types::*;
use async_trait::async_trait;

#[async_trait]
pub trait Proxmox {
    /// Start virtual machine.
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`POST /api2/json/nodes/{node}/qemu/{vmid}/status/start`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/status/start)
    ///
    async fn start(&self, vm: VmRef) -> Result<UniqueProcessId>;

    /// Shutdown virtual machine. This is similar to pressing the power button
    /// on a physical machine. This will send an ACPI event for the guest OS,
    /// which should then proceed to a clean shutdown.
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`POST /api2/json/nodes/{node}/qemu/{vmid}/status/shutdown`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/status/shutdown)
    ///
    async fn shutdown(&self, vm: VmRef) -> Result<UniqueProcessId>;

    /// Stop virtual machine. The qemu process will exit immediately. This is
    /// akin to pulling the power plug of a running computer and may damage the
    /// VM data.
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`POST /api2/json/nodes/{node}/qemu/{vmid}/status/stop`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/status/stop)
    ///
    async fn stop(&self, vm: VmRef) -> Result<UniqueProcessId>;

    /// Reboot the VM by shutting it down, and starting it again. Applies
    /// pending changes
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`POST /api2/json/nodes/{node}/qemu/{vmid}/status/reboot`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/status/reboot)
    ///
    async fn reboot(&self, vm: VmRef) -> Result<UniqueProcessId>;

    /// Create a copy of virtual machine/template.
    ///
    /// # Arguments
    ///
    /// * `options`:
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`HTTP: POST /api2/json/nodes/{node}/qemu/{vmid}/clone`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/clone)
    ///
    async fn create(&self, options: VmOptions) -> Result<UniqueProcessId>;

    /// Destroy the VM and all used/owned volumes. Removes any VM specific
    /// permissions and firewall rules
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`HTTP: DELETE /api2/json/nodes/{node}/qemu/{vmid}`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid})
    ///
    async fn delete(&self, vm: VmRef) -> Result<UniqueProcessId>;

    /// Get virtual machine status.
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`GET /api2/json/nodes/{node}/qemu/{vmid}/status/current`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/status/current)
    ///
    async fn vm_status(&self, vm: VmRef) -> Result<Status>;

    /// Read task status.
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoint:
    ///
    /// [`GET /api2/json/nodes/{node}/tasks/{upid}/status`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/tasks/{upid}/status)
    ///
    async fn task_status(&self, task: TaskRef) -> Result<TaskStatus>;
}
