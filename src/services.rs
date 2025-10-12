use crate::prelude::{Error, Proxmox, Result};
use crate::proxmox::types::{TaskRef, TaskStatus};
use sqlx::PgTransaction;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub mod action;
pub mod deletion;
pub mod setup;

/// Wait until task is finished.
///
pub async fn wait_until_finish(
    proxmox_client: &Arc<dyn Proxmox + Send + Sync>,
    task: TaskRef,
    wait_secs: u64,
    timeout: Option<u64>,
) -> Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout.unwrap_or(30));

    loop {
        let elapsed = start.elapsed();
        if elapsed > timeout {
            return Err(Error::Timeout(elapsed.as_secs_f32()));
        }

        match proxmox_client.task_status(&task).await? {
            TaskStatus::Pending => tokio::time::sleep(Duration::from_secs(wait_secs)).await,
            TaskStatus::Completed => break,
            TaskStatus::Failed(error) => return Err(Error::Any(error)),
        }
    }

    Ok(())
}

pub async fn finish_service(service_result: Result<()>, transaction: PgTransaction<'_>) {
    match service_result {
        Ok(_) => match transaction.commit().await {
            Ok(_) => tracing::info!(target: "service", "Service completed"),
            Err(commit_error) => {
                tracing::error!(target: "service", error = ?commit_error, "Failed to commit transaction!")
            }
        },
        Err(finish_error) => match transaction.rollback().await {
            Ok(_) => {
                tracing::error!( target: "service", error = ?finish_error, "Failed to complete service" )
            }
            Err(rollback_error) => {
                tracing::error!(target: "service", error = ?rollback_error, "Failed to rollback transaction!")
            }
        },
    }
}
