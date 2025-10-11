use crate::error::{Error, Result};
use crate::model::queries;
use crate::model::types::{ApiServer, ServiceStatus};
use crate::prelude::AppState;
use crate::proxmox::types::{TaskRef, TaskStatus, VmRef};
use crate::web::types::NewServerPayload;
use sqlx::PgTransaction;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub async fn setup_new_server(app_state: AppState, user_id: Uuid, payload: NewServerPayload) {
    // Create a transaction for a chain of all sequential queries.
    let transaction_result = app_state.pool.begin().await;
    let mut transaction = match transaction_result {
        Ok(tx) => tx,
        Err(er) => {
            tracing::error!(target: "!! setup", error = ?er, "Failed to begin transaction!");
            return;
        }
    };

    // Initial records in the database.
    let create_result = create_initial_server(&mut transaction, user_id, &payload).await;
    let (server_id, service_id) = match create_result {
        Ok(server) => (server.server_id, server.service_id),
        Err(error) => {
            tracing::error!(target: "!! setup", error = ?error, "Failed to create initial server!");
            return;
        }
    };

    // Setup proxmox server.
    let setup_result = service_setup(&mut transaction, &app_state, server_id, &payload).await;
    let status = match setup_result {
        Ok(_) => ServiceStatus::Active,
        Err(_) => ServiceStatus::Failed,
    };

    // Update server status based on the setup result and finish transaction.
    let update_result = queries::update_service_status(&mut transaction, service_id, status).await;
    finish_transaction(update_result, transaction, service_id).await;
}

async fn create_initial_server(
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    payload: &NewServerPayload,
) -> Result<ApiServer> {
    let server_id = queries::create_server_record(transaction, &payload).await?;
    let ip_address = queries::reserve_ip_for_server(transaction, server_id, &payload).await?;
    let service_id =
        queries::create_service_record(transaction, user_id, server_id, &payload).await?;
    queries::save_config_values(transaction, service_id, &payload).await?;
    queries::save_custom_values(transaction, service_id, &payload).await?;

    Ok(ApiServer {
        server_id,
        service_id,
        vm_id: None,
        node_name: None,
        ip_address,
        status: ServiceStatus::Pending,
    })
}

async fn service_setup(
    transaction: &mut PgTransaction<'_>,
    app_state: &AppState,
    server_id: Uuid,
    payload: &NewServerPayload,
) -> Result<()> {
    // Find template vm in the database.
    let template_vm: VmRef = queries::find_template(transaction, payload.product_id).await?;

    // Clone new Proxmox server.
    let (new_vmid, clone_upid) = app_state.proxmox.create(template_vm.clone()).await?;
    let clone_task = TaskRef::new(&template_vm.node, &clone_upid);
    wait_until_finish(&app_state, clone_task, 1, None).await?;

    // Save vmid to the database.
    let new_vm = VmRef::new(&template_vm.node, new_vmid);
    queries::update_initial_server(transaction, server_id, new_vm.clone()).await?;

    // Setup configuration (CPU, RAM).
    let config_upid = app_state
        .proxmox
        .vm_config(new_vm, payload.clone().try_into()?)
        .await?;
    let config_task = TaskRef::new(&template_vm.node, &config_upid);
    wait_until_finish(&app_state, config_task, 1, None).await?;

    Ok(())
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

async fn finish_transaction(
    update_result: Result<()>,
    transaction: PgTransaction<'_>,
    service_id: Uuid,
) {
    match update_result {
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
