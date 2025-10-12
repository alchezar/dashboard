use crate::model::queries;
use crate::model::types::ServerStatus;
use crate::prelude::Result;
use crate::prelude::{AppState, Proxmox};
use crate::proxmox::types::TaskRef;
use crate::services;
use crate::services::wait_until_finish;
use sqlx::PgTransaction;
use std::sync::Arc;
use uuid::Uuid;

pub async fn run(app_state: AppState, user_id: Uuid, server_id: Uuid) {
    // Create a transaction for a chain of all sequential queries.
    let transaction_result = app_state.pool.begin().await;
    let mut transaction = match transaction_result {
        Ok(tx) => tx,
        Err(er) => {
            tracing::error!(target: "setup", error = ?er, "Failed to begin transaction!");
            return;
        }
    };

    let result = delete_server(&app_state.proxmox, &mut transaction, user_id, server_id).await;
    services::finish_service(result, transaction).await;
}

async fn delete_server(
    proxmox_client: &Arc<dyn Proxmox + Send + Sync>,
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<()> {
    // Check server and immediately update the status to transient.
    let vm = queries::get_server_proxmox_ref(&mut **transaction, user_id, server_id).await?;
    queries::update_server_status(&mut **transaction, server_id, ServerStatus::Deleting).await?;

    // Delete Proxmox VM and wait until process finish.
    let upid = proxmox_client.delete(vm.clone()).await?;
    let task = TaskRef::new(&vm.node, &upid);
    wait_until_finish(&proxmox_client, task, 1, None).await?;

    // Then delete server record from the database.
    queries::delete_server_record(transaction, server_id).await?;

    Ok(())
}
