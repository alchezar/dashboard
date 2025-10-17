use crate::prelude::{Error, Result};
use crate::web::types::NewServerPayload;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};

/// Generic wrapper for all successful Proxmox API responses.
///
/// Proxmox API consistently wraps its successful responses in a JSON object
/// with a single `data` field. This struct models that wrapper.
///
/// # Example JSON
///
/// ```json
/// "data": {
///     ...
/// }
/// ```
///
#[derive(Deserialize)]
pub struct Response<T> {
    pub data: T,
}

/// Type-safe representation of a Proxmox Unique Process ID (`UPID`).
///
/// This is a new-type wrapper around a `String` to prevent accidental misuse of
/// a plain string where a UPID is expected. It also provides helper methods for
/// formatting the UPID for use in API URLs.
///
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct UniqueProcessId(String);

impl UniqueProcessId {
    /// Percent-encode the UPID to make it safe for use in a URL path
    ///
    /// For example, characters like `:` and `@` will be encoded to
    /// `%3A` and `%40` respectively.
    ///
    pub fn encoded(&self) -> String {
        utf8_percent_encode(&self.0, NON_ALPHANUMERIC).to_string()
    }

    /// Returns the inner string of the UPID.
    ///
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<&str> for UniqueProcessId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

// -----------------------------------------------------------------------------

/// Specific response structure for endpoints that return a VM's power status.
///
/// # Fields
///
/// * `status`: Current power status of a virtual machine.
///
#[derive(Deserialize)]
pub struct StatusPayload {
    pub status: Status,
}

/// Power status of a virtual machine.
///
#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Stopped,
    Running,
}

/// High-level status of a long-running asynchronous task in Proxmox.
///
#[derive(Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    Completed,
    Failed(String),
}

/// Raw response from the Proxmox task status endpoint.
///
/// # Fields
///
/// * `status`: Current power status of a virtual machine.
/// * `exit_status`: Exit status of the task, present once the task has
///   stopped. Typically, `"OK"` on success.
///
#[derive(Deserialize)]
pub struct TaskResponse {
    pub status: Status,
    #[serde(rename = "exitstatus")]
    pub exit_status: Option<String>,
}

// -----------------------------------------------------------------------------

/// Reference to a specific virtual machine on a Proxmox cluster.
///
/// # Fields
///
/// * `node`: Name of the Proxmox node where the VM is located (e.g., "pve").
/// * `id`: Unique integer ID of the virtual machine (VMID).
///
#[derive(Debug, Clone)]
pub struct VmRef {
    pub node: String,
    pub id: i32,
}

impl VmRef {
    /// Creates a new reference to a virtual machine.
    ///
    pub fn new(node: &str, id: i32) -> Self {
        Self {
            node: node.to_owned(),
            id,
        }
    }
}

/// Reference to a specific asynchronous task on a Proxmox cluster.
///
/// # Fields
///
/// * `node`: Name of the Proxmox node where the task is running.
/// * `upid`: Unique Process ID (UPID) of the task.
///
#[derive(Debug, Clone)]
pub struct TaskRef {
    pub node: String,
    pub upid: UniqueProcessId,
}

impl TaskRef {
    /// Creates a new reference to Proxmox task.
    ///
    pub fn new(node: &str, upid: &UniqueProcessId) -> Self {
        Self {
            node: node.to_owned(),
            upid: upid.clone(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct VmConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipconfig0: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cores: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<i32>,
}

impl VmConfig {
    pub fn new(ip_config: String, cpu_cores: Option<i32>, memory_gb: Option<i32>) -> Self {
        Self {
            cores: cpu_cores,
            memory: memory_gb.map(|ram| ram * 1024),
            ipconfig0: Some(ip_config),
        }
    }
}

impl TryFrom<NewServerPayload> for VmConfig {
    type Error = Error;
    fn try_from(payload: NewServerPayload) -> Result<Self> {
        Ok(Self {
            cores: payload.cpu_cores,
            memory: payload.ram_gb.map(|ram| ram * 1024),
            ipconfig0: payload.ip_config,
        })
    }
}
