use crate::error::{Error, Result};
use crate::model::queries;
use crate::model::types::ServiceStatus;
use crate::prelude::AppState;
use crate::proxmox::types::{TaskRef, TaskStatus, VmConfig, VmRef};
use crate::web::types::NewServerPayload;
use sqlx::PgTransaction;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub async fn setup_new_server(app_state: AppState, user_id: Uuid, payload: NewServerPayload) {
    let mut transaction = match app_state.pool.begin().await {
        Ok(tx) => tx,
        Err(er) => {
            tracing::error!(target: "!! setup", error = ?er, "Failed to begin transaction!");
            return;
        }
    };

    // Initial records in the database (dashboard service, custom fields and
    // configurable options).
    let result = queries::create_initial_server(&mut transaction, user_id, &payload).await;
    let (server_id, service_id) = match result {
        Ok(server) => (server.server_id, server.service_id),
        Err(error) => {
            tracing::error!(target: "!! setup", error = ?error, "Failed to create initial server!");
            return;
        }
    };

    // Service setup.
    let result = async {
        // Find template vm in the database.
        let template_vm: VmRef =
            queries::find_template(&mut transaction, payload.product_id).await?;

        // Clone new Proxmox server.
        let (new_vmid, clone_upid) = app_state.proxmox.create(template_vm.clone()).await?;
        let clone_task = TaskRef::new(&template_vm.node, &clone_upid);
        wait_until_finish(&app_state, clone_task, 1, None).await?;

        // Save vmid to the database.
        let new_vm = VmRef::new(&template_vm.node, new_vmid);
        queries::update_initial_server(&mut transaction, server_id, new_vm.clone()).await?;

        // Setup configuration (CPU, RAM).
        let config_upid = app_state.proxmox.vm_config(new_vm, payload.into()).await?;
        let config_task = TaskRef::new(&template_vm.node, &config_upid);
        wait_until_finish(&app_state, config_task, 1, None).await?;
        Ok(())
    }
    .await;

    update_service_status(result, transaction, service_id).await;
}

/// Wait until task is finished.
///
async fn wait_until_finish(
    app_state: &AppState,
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

        match app_state.proxmox.task_status(&task).await? {
            TaskStatus::Pending => tokio::time::sleep(Duration::from_secs(wait_secs)).await,
            TaskStatus::Completed => break,
            TaskStatus::Failed(error) => return Err(Error::Any(error)),
        }
    }

    Ok(())
}

/// Update server status based on the setup result.
///
async fn update_service_status(
    result: Result<()>,
    mut transaction: PgTransaction<'_>,
    service_id: Uuid,
) {
    let status = match result {
        Ok(_) => ServiceStatus::Active,
        Err(error) => {
            tracing::error!(
                target: "!! setup",
                service_id = ?service_id,
                error = ?error,
                "Failed to set up server!"
            );
            ServiceStatus::Failed
        }
    };
    match queries::update_service_status(&mut transaction, service_id, status).await {
        Ok(_) => match transaction.commit().await {
            Ok(_) => tracing::info!(target: ">> setup", "Server set up and state committed!"),
            Err(commit_error) => {
                tracing::error!(target: "!! setup", error = ?commit_error, "Failed to commit transaction!")
            }
        },
        Err(status_error) => {
            if let Err(rollback_error) = transaction.rollback().await {
                tracing::error!(target: "!! setup", error = ?rollback_error, "Failed to rollback transaction!")
            };
            tracing::error!(
                target: "!! setup",
                service_id = ?service_id,
                error = ?status_error,
                "Failed to update service status!"
            );
        }
    }
}
