#![allow(unused)]

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct Response<T> {
    pub data: T,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct UniqueProcessId(String);

impl UniqueProcessId {
    pub fn encoded(&self) -> String {
        // Percent-encode the UPID to make it safe for use in a URL path, like
        // `:` to `%3A` or `@` to `%40`.
        utf8_percent_encode(&self.0, NON_ALPHANUMERIC).to_string()
    }
}

impl From<&str> for UniqueProcessId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

// -----------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct StatusResponse {
    pub status: Status,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Stopped,
    Running,
}

pub enum TaskStatus {
    Pending,
    Completed,
    Failed(String),
}

#[derive(Deserialize)]
pub struct TaskResponse {
    pub status: Status,
    pub exitstatus: Option<String>,
}

// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VmRef {
    pub node: String,
    pub id: i32,
}

#[derive(Debug, Clone)]
pub struct TaskRef {
    pub node: String,
    pub up_id: UniqueProcessId,
}

pub struct VmOptions {}
