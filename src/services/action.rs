use crate::model::queries;
use crate::model::types::ServerStatus;
use crate::prelude::Result;
use crate::prelude::{AppState, Proxmox};
use crate::proxmox::types::TaskRef;
use crate::services;
use crate::web::types::ServerAction;
use sqlx::PgTransaction;
use std::sync::Arc;
use uuid::Uuid;

pub async fn run(app_state: AppState, user_id: Uuid, server_id: Uuid, action: ServerAction) {
    // Create a transaction for a chain of all sequential queries.
    let transaction_result = app_state.pool.begin().await;
    let mut transaction = match transaction_result {
        Ok(tx) => tx,
        Err(er) => {
            tracing::error!(target: "setup", error = ?er, "Failed to begin transaction!");
            return;
        }
    };

    let result = start_action(
        &app_state.proxmox,
        &mut transaction,
        user_id,
        server_id,
        action,
    )
    .await;
    services::finish_service(result, transaction).await;
}

async fn start_action(
    proxmox_client: &Arc<dyn Proxmox + Send + Sync>,
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    server_id: Uuid,
    action: ServerAction,
) -> Result<()> {
    // Check server.
    let vm = queries::get_server_proxmox_ref(&mut **transaction, user_id, server_id).await?;
    let node = vm.node.clone();
    tracing::info!(target: "action", action = ?action, "Server action");

    // Immediately update the status to transient.
    let (transient_status, final_status) = match action {
        ServerAction::Start => (ServerStatus::Starting, ServerStatus::Running),
        ServerAction::Stop => (ServerStatus::Stopping, ServerStatus::Stopped),
        ServerAction::Shutdown => (ServerStatus::ShuttingDown, ServerStatus::Stopped),
        ServerAction::Reboot => (ServerStatus::Rebooting, ServerStatus::Running),
    };
    queries::update_server_status(&mut **transaction, server_id, transient_status).await?;
    tracing::info!(target: "action", action = ?action, "Server status updated to transient state");

    // Start the action and update the status again once it's done.
    let upid = match action {
        ServerAction::Start => proxmox_client.start(vm).await?,
        ServerAction::Stop => proxmox_client.stop(vm).await?,
        ServerAction::Shutdown => proxmox_client.shutdown(vm).await?,
        ServerAction::Reboot => proxmox_client.reboot(vm).await?,
    };
    let task = TaskRef::new(&node, &upid);
    services::wait_until_finish(&proxmox_client, task, 1, None).await?;
    queries::update_server_status(&mut **transaction, server_id, final_status).await?;

    Ok(())
}
