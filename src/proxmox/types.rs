#![allow(unused)]

pub type ProcessId = String;

pub enum TaskStatus {
    Pending,
    Completed,
    Failed,
}

pub enum VmStatus {
    Running,
    Stopped,
}

pub struct VmOptions {}

#[derive(Debug, Clone)]
pub struct VmRef {
    node: String,
    id: i32,
}
