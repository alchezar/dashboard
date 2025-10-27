use crate::model::queries;
use crate::model::types::ServerStatus;
use crate::proxmox::Proxmox;
use crate::proxmox::types::TaskRef;
use crate::services;
use crate::state::AppState;
use crate::web::types::ServerAction;
use dashboard_common::error::Result;
use sqlx::PgTransaction;
use std::sync::Arc;
use uuid::Uuid;

/// Public entry point for a server action background task.
///
/// # Arguments
///
/// * `app_state`: Shared application state.
/// * `user_id`: ID of the user performing the action.
/// * `server_id`: ID of the target server.
/// * `action`: Specific action to perform.
///
pub async fn run(app_state: AppState, user_id: Uuid, server_id: Uuid, action: ServerAction) {
    // Find desired statuses and update the server to the transient one
    // immediately.
    let (transient_status, final_status) = match action {
        ServerAction::Start => (ServerStatus::Starting, ServerStatus::Running),
        ServerAction::Stop => (ServerStatus::Stopping, ServerStatus::Stopped),
        ServerAction::Shutdown => (ServerStatus::ShuttingDown, ServerStatus::Stopped),
        ServerAction::Reboot => (ServerStatus::Rebooting, ServerStatus::Running),
    };
    let Ok(old_status) =
        services::set_transient_status(&app_state.pool, user_id, server_id, transient_status).await
    else {
        tracing::error!(target: "service", status = ?transient_status, "Can't update server status to transient state");
        return;
    };

    // Create a transaction for a chain of all sequential queries.
    let Ok(mut transaction) = app_state.pool.begin().await else {
        tracing::error!(target: "service", "Failed to begin transaction!");
        services::set_transient_status(&app_state.pool, user_id, server_id, old_status)
            .await
            .ok();
        return;
    };

    let result = start_action(
        &app_state.proxmox,
        &mut transaction,
        user_id,
        server_id,
        action,
        final_status,
    )
    .await;

    services::finalize_transaction(&result, transaction).await;

    // Return the old status if something went wrong.
    if result.is_err() {
        tracing::error!(target: "service", status = ?old_status, "Action failed, reverting status");
        services::set_transient_status(&app_state.pool, user_id, server_id, old_status)
            .await
            .ok();
    }
}

/// Core logic for a server action, executed within a database transaction.
///
/// # Arguments
///
/// * `proxmox_client`: Client for interacting with the Proxmox API.
/// * `transaction`: Active database transaction.
/// * `user_id`: ID of the user performing the action.
/// * `server_id`: ID of the target server.
/// * `action`: Specific action to perform.
///
/// # Returns
///
/// An empty `Result` on success.
///
#[tracing::instrument(level = "trace", target = "service", skip(proxmox_client, transaction))]
async fn start_action(
    proxmox_client: &Arc<dyn Proxmox + Send + Sync>,
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    server_id: Uuid,
    action: ServerAction,
    final_status: ServerStatus,
) -> Result<()> {
    // Check server.
    let vm = queries::get_server_proxmox_ref(transaction.as_mut(), user_id, server_id).await?;
    let node = vm.node.clone();
    tracing::debug!(target: "service", ?vm, "Found server on Proxmox");

    // Start the action and update the status again once it's done.
    let upid = match action {
        ServerAction::Start => proxmox_client.start(vm).await?,
        ServerAction::Stop => proxmox_client.stop(vm).await?,
        ServerAction::Shutdown => proxmox_client.shutdown(vm).await?,
        ServerAction::Reboot => proxmox_client.reboot(vm).await?,
    };
    tracing::debug!(target: "service", ?upid, "Proxmox action task started, waiting for completion");

    let task = TaskRef::new(&node, &upid);
    services::wait_until_finish(proxmox_client, task, 1, None).await?;
    tracing::info!(target: "service", "Proxmox task finished successfully");

    queries::update_server_status(transaction.as_mut(), server_id, final_status).await?;

    Ok(())
}
