use crate::proxmox::Proxmox;
use crate::proxmox::types::{TaskRef, TaskStatus};
use common::error::{Error, Result};
use sqlx::PgTransaction;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub mod action;
pub mod deletion;
pub mod setup;

// -----------------------------------------------------------------------------

/// Polls a Proxmox task until it is complete, with a timeout.
///
/// # Arguments
///
/// * `proxmox_client`: Client for interacting with the Proxmox API.
/// * `task`: Proxmox task to monitor.
/// * `wait_secs`: Interval in seconds between polling attempts.
/// * `timeout`: Optional total time in seconds before returning a timeout
///   error.
///
/// # Returns
///
/// An empty `Result` on successful.
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

/// Finalizes a database transaction by committing on success or rolling back on
/// error.
///
/// # Arguments
///
/// * `service_result`: `Result` of the operation performed within the
///   transaction.
/// * `transaction`: Database transaction to be finalized.
///
pub async fn finalize_transaction(service_result: Result<()>, transaction: PgTransaction<'_>) {
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
