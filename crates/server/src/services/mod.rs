use crate::model::queries;
use crate::model::types::ServerStatus;
use crate::proxmox::Proxmox;
use crate::proxmox::types::{TaskRef, TaskStatus};
use dashboard_common::error::{Error, Result};
use sqlx::{PgPool, PgTransaction};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

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

/// Sets a server's status to a transient state and commits the change
/// immediately.
///
/// # Arguments
///
/// * `pool`: Database connection pool.
/// * `user_id`: ID of the user who owns the server.
/// * `server_id`: ID of the target server.
/// * `new_status`: New transient status to set.
///
/// # Returns
///
/// The old server status on success.
///
pub async fn set_transient_status(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
    new_status: ServerStatus,
) -> Result<ServerStatus> {
    let mut transaction = pool.begin().await?;

    let old_status = queries::get_server_by_id(transaction.as_mut(), user_id, server_id)
        .await?
        .status;
    queries::update_server_status(transaction.as_mut(), server_id, new_status).await?;

    transaction.commit().await?;

    tracing::debug!(target: "service", status = ?new_status, "Server status updated");
    Ok(old_status)
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
pub async fn finalize_transaction(service_result: &Result<()>, transaction: PgTransaction<'_>) {
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
