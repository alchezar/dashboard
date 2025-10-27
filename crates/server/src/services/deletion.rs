use crate::model::queries;
use crate::model::types::ServerStatus;
use crate::proxmox::Proxmox;
use crate::proxmox::types::TaskRef;
use crate::services;
use crate::services::wait_until_finish;
use crate::state::AppState;
use dashboard_common::error::Result;
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
    // Immediately update the status to `Deleting.
    let Ok(old_status) =
        services::set_transient_status(&app_state.pool, user_id, server_id, ServerStatus::Deleting)
            .await
    else {
        tracing::error!(target: "service", "Can't update server status to 'Deleting' state");
        return;
    };

    // Create a transaction for a chain of all sequential queries.
    let Ok(mut transaction) = app_state.pool.begin().await else {
        tracing::error!(target: "service", "Failed to begin transaction!");
        return;
    };

    let result = delete_server(&app_state.proxmox, &mut transaction, user_id, server_id).await;
    services::finalize_transaction(&result, transaction).await;

    // Return the old status if something went wrong.
    if result.is_err() {
        services::set_transient_status(&app_state.pool, user_id, server_id, old_status)
            .await
            .ok();
    }
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
    let vm = queries::get_server_proxmox_ref(&mut **transaction, user_id, server_id).await?;
    tracing::debug!(target: "service", ?vm, "Found server on Proxmox");

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
