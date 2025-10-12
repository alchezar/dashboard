use crate::model::queries;
use crate::model::types::{ApiServer, ServerStatus, ServiceStatus};
use crate::prelude::{AppState, Result};
use crate::proxmox::types::{TaskRef, VmRef};
use crate::services;
use crate::web::types::NewServerPayload;
use sqlx::PgTransaction;
use uuid::Uuid;

/// Public entry point for the new server setup background task.
///
/// # Arguments
///
/// * `app_state`: Shared application state.
/// * `user_id`: ID of the user for whom to create the server.
/// * `payload`: Specifications for the new server.
///
pub async fn run(app_state: AppState, user_id: Uuid, payload: NewServerPayload) {
    // Create a transaction for a chain of all sequential queries.
    let Ok(mut transaction) = app_state.pool.begin().await else {
        tracing::error!(target: "service", "Failed to begin transaction!");
        return;
    };

    let result = async {
        // Initial records in the database.
        let ApiServer {
            service_id,
            server_id,
            ..
        } = create_initial_server(&mut transaction, user_id, &payload).await?;
        tracing::info!(target: "service", %service_id, %server_id, "Server record created");

        // Setup proxmox server.
        service_setup(&mut transaction, &app_state, server_id, &payload).await?;
        queries::update_server_status(&mut *transaction, server_id, ServerStatus::Stopped).await?;
        queries::update_service_status(&mut *transaction, service_id, ServiceStatus::Active)
            .await?;

        tracing::info!(target: "service", "Proxmox VM setup finished successfully");
        Ok(())
    }
    .await;

    services::finalize_transaction(result, transaction).await;
}

/// Creates all initial database records for a new server within a transaction.
///
/// # Arguments
///
/// * `transaction`: Active database transaction.
/// * `user_id`: ID of the user.
/// * `payload`: Specifications for the new server.
///
/// # Returns
///
/// `ApiServer` struct representing the newly created server records.
///
#[tracing::instrument(level = "trace", target = "service",
	skip(transaction, payload),
	fields(host_name = %payload.host_name))]
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

    tracing::debug!(target: "service", %server_id, %service_id, "Initial DB records created successfully");
    Ok(ApiServer {
        server_id,
        service_id,
        vm_id: None,
        node_name: None,
        ip_address,
        status: ServerStatus::SettingUp,
    })
}

/// Orchestrates the Proxmox-side setup for a new VM, including cloning and
/// configuration.
///
/// # Arguments
///
/// * `transaction`: Active database transaction.
/// * `app_state`: Shared application state.
/// * `server_id`: ID of the server record to associate with the new VM.
/// * `payload`: Specifications for the new server.
///
/// # Returns
///
/// An empty `Result` on success.
///
#[tracing::instrument(level = "trace", target = "service",
	skip(transaction, app_state, payload),
	fields(host_name = %payload.host_name))]
async fn service_setup(
    transaction: &mut PgTransaction<'_>,
    app_state: &AppState,
    server_id: Uuid,
    payload: &NewServerPayload,
) -> Result<()> {
    // Find template vm in the database.
    let template_vm: VmRef = queries::find_template(transaction, payload.product_id).await?;
    tracing::debug!(target: "service", template_vmid = %template_vm.id, "Found VM template");

    // Clone new Proxmox server.
    let (new_vmid, clone_upid) = app_state.proxmox.create(template_vm.clone()).await?;
    tracing::debug!(target: "service", upid = ?clone_upid, "Proxmox clone task started");
    let clone_task = TaskRef::new(&template_vm.node, &clone_upid);
    services::wait_until_finish(&app_state.proxmox, clone_task, 1, None).await?;
    tracing::debug!(target: "service", %new_vmid, "VM cloned successfully");

    // Save vmid to the database.
    let new_vm = VmRef::new(&template_vm.node, new_vmid);
    queries::update_initial_server(transaction, server_id, new_vm.clone()).await?;
    tracing::info!(target: "service", "Server record updated");

    // Setup configuration (CPU, RAM).
    let config_upid = app_state
        .proxmox
        .vm_config(new_vm, payload.clone().try_into()?)
        .await?;
    tracing::debug!(%server_id, upid = ?config_upid, "Proxmox config task started");

    let config_task = TaskRef::new(&template_vm.node, &config_upid);
    services::wait_until_finish(&app_state.proxmox, config_task, 1, None).await?;
    tracing::info!(%server_id, %new_vmid, "VM configuration applied successfully");

    Ok(())
}
