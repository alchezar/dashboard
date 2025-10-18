use crate::model::queries;
use crate::prelude::{AppState, Proxmox, Result, ServerStatus, TaskRef};
use crate::services;
use crate::services::wait_until_finish;
use sqlx::PgTransaction;
use std::sync::Arc;
use uuid::Uuid;

/// Public entry point for the server deletion background task.
///
/// # Arguments
///
/// * `app_state`: Shared application state.
/// * `user_id`: ID of the user who owns the server.
/// * `server_id`: ID of the server to delete.
///
pub async fn run(app_state: AppState, user_id: Uuid, server_id: Uuid) {
    // Create a transaction for a chain of all sequential queries.
    let Ok(mut transaction) = app_state.pool.begin().await else {
        tracing::error!(target: "service", "Failed to begin transaction!");
        return;
    };

    let result = delete_server(&app_state.proxmox, &mut transaction, user_id, server_id).await;

    services::finalize_transaction(result, transaction).await;
}

/// Core logic for server deletion, executed within a database transaction.
///
/// # Arguments
///
/// * `proxmox_client`: Client for interacting with the Proxmox API.
/// * `transaction`: Active database transaction.
/// * `user_id`: ID of the user who owns the server.
/// * `server_id`: ID of the server to delete.
///
/// # Returns
///
/// An empty `Result` on success.
///
#[tracing::instrument(level = "trace", target = "service", skip(proxmox_client, transaction))]
async fn delete_server(
    proxmox_client: &Arc<dyn Proxmox + Send + Sync>,
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<()> {
    // Check server and immediately update the status to transient.
    let vm = queries::get_server_proxmox_ref(&mut **transaction, user_id, server_id).await?;
    tracing::debug!(target: "service", ?vm, "Found server on Proxmox");
    queries::update_server_status(&mut **transaction, server_id, ServerStatus::Deleting).await?;
    tracing::debug!(target: "service", status = ?ServerStatus::Deleting, "Server status updated to transient state");

    // Delete Proxmox VM and wait until process finish.
    let upid = proxmox_client.delete(vm.clone()).await?;
    tracing::debug!(target: "service", upid = ?upid, "Proxmox delete task started");

    let task = TaskRef::new(&vm.node, &upid);
    wait_until_finish(proxmox_client, task, 1, None).await?;
    tracing::info!(target: "service", "Proxmox VM deletion finished successfully");

    // Then delete server record from the database.
    queries::delete_server_record(transaction, server_id).await?;

    Ok(())
}
