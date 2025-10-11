use crate::model::queries;
use crate::prelude::AppState;
use crate::prelude::Result;
use crate::proxmox::types::TaskRef;
use crate::services::wait_until_finish;
use uuid::Uuid;

pub async fn delete_server(app_state: AppState, user_id: Uuid, server_id: Uuid) {
    let result: Result<()> = async {
        // Delete Proxmox VM and wait until process finish.
        let vm = queries::get_server_proxmox_ref(&app_state.pool, user_id, server_id).await?;
        let upid = app_state.proxmox.delete(vm.clone()).await?;
        let task = TaskRef::new(&vm.node, &upid);
        wait_until_finish(&app_state, task, 1, None).await?;

        // Then delete server record.
        queries::delete_server_record(&app_state.pool, server_id).await?;

        Ok(())
    }
    .await;

    match result {
        Ok(_) => tracing::info!(target: "delete", "Server deleted"),
        Err(er) => {
            tracing::error!(target: "delete", error = ?er, "Failed to delete server!");
            return;
        }
    }
}
