use crate::error::Error;
use crate::prelude::AppState;
use crate::proxmox::types::{TaskRef, TaskStatus};
use std::time::{Duration, Instant};

pub mod server_deletion;
pub mod server_setup;

/// Wait until task is finished.
///
pub async fn wait_until_finish(
    app_state: &AppState,
    task: TaskRef,
    wait_secs: u64,
    timeout: Option<u64>,
) -> crate::error::Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout.unwrap_or(30));

    loop {
        let elapsed = start.elapsed();
        if elapsed > timeout {
            return Err(Error::Timeout(elapsed.as_secs_f32()));
        }

        match app_state.proxmox.task_status(&task).await? {
            TaskStatus::Pending => tokio::time::sleep(Duration::from_secs(wait_secs)).await,
            TaskStatus::Completed => break,
            TaskStatus::Failed(error) => return Err(Error::Any(error)),
        }
    }

    Ok(())
}
