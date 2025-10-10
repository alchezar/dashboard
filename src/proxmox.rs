pub mod client;
pub mod types;

// -----------------------------------------------------------------------------

use crate::prelude::Result;
use crate::proxmox::types::*;
use async_trait::async_trait;

/// An abstract interface for interacting with the Proxmox VE API.
///
/// Defines a contract for a client that can perform various operations on a
/// Proxmox cluster, such as managing the lifecycle of virtual machines (start,
/// stop, create, delete), and querying the status of VMs and asynchronous
/// tasks.
///
#[async_trait]
pub trait Proxmox {
    /// Start virtual machine.
    ///
    /// # Arguments
    ///
    /// * `vm`: target virtual machine on the Proxmox cluster.
    ///
    /// # Returns
    ///
    /// * `UniqueProcessId` (UPID) of the start task.
    ///   This task can be monitored using the `task_status` method.
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

    /// Get the next free VMID and create a clone of a virtual machine or
    /// template.
    ///
    /// # Arguments
    ///
    /// * `vm`: template virtual machine on the Proxmox cluster to clone.
    ///
    /// # Proxmox API
    ///
    /// This method corresponds to the following Proxmox API endpoints:
    ///
    /// [`HTTP: GET /api2/json/cluster/nextid`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/cluster/nextid)\
    /// [`HTTP: POST /api2/json/nodes/{node}/qemu/{vmid}/clone`](https://pve.proxmox.com/pve-docs/api-viewer/index.html#/nodes/{node}/qemu/{vmid}/clone)
    ///
    async fn create(&self, vm: VmRef) -> Result<(i32, UniqueProcessId)>;

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

	async fn vm_config(&self, vm: VmRef, config: VmConfig) -> Result<UniqueProcessId>;

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
    async fn task_status(&self, task: &TaskRef) -> Result<TaskStatus>;
}
