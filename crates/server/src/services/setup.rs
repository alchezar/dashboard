use crate::model::queries;
use crate::model::types::{ServerStatus, ServiceStatus};
use crate::proxmox::Proxmox;
use crate::proxmox::types::{TaskRef, VmConfig, VmRef};
use crate::services;
use crate::state::AppState;
use crate::web::types::NewServerPayload;
use dashboard_common::error::Result;
use sqlx::PgTransaction;
use std::sync::Arc;
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

    let result = create_server(&app_state.proxmox, &mut transaction, user_id, &payload).await;

    services::finalize_transaction(result, transaction).await;
}

/// Creates all initial database records for a new server within a transaction.
/// and orchestrates the Proxmox-side setup for a new VM, including cloning and
/// configuration.
///
/// # Arguments
///
/// * `proxmox_client`: Client for interacting with the Proxmox API.
/// * `transaction`: Active database transaction.
/// * `user_id`: ID of the user who owns the server.
/// * `payload`: Specifications for the new server.
///
/// # Returns
///
/// An empty `Result` on success.
///
async fn create_server(
    proxmox_client: &Arc<dyn Proxmox + Send + Sync>,
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    payload: &NewServerPayload,
) -> Result<()> {
    // Create initial server.

    let server_id = queries::create_server_record(transaction, payload).await?;
    tracing::info!(target: "service", %server_id, "Initial server record created");

    let template_id = queries::find_template_id(transaction, &payload.os).await?;
    let service_id =
        queries::create_service_record(transaction, user_id, server_id, template_id, payload)
            .await?;
    tracing::info!(target: "service", %service_id, "Service record created");

    queries::save_config_values(transaction, service_id, payload).await?;
    queries::save_custom_values(transaction, service_id, payload).await?;
    tracing::info!(target: "service", "Custom field and configurable option records created");

    let ip_config = queries::reserve_ip_for_server(transaction, server_id, payload).await?;
    let vm_config = VmConfig::new(ip_config.form()?, payload.cpu_cores, payload.ram_gb);
    tracing::info!(target: "service", %server_id, %service_id, "IP and VM config created");

    // Setup service.

    let template_vm: VmRef = queries::find_template(transaction, service_id).await?;
    tracing::info!(target: "service", template_vmid = %template_vm.id, "Found VM template");

    // Clone new Proxmox server.
    let (new_vmid, clone_upid) = proxmox_client.create(template_vm.clone()).await?;
    tracing::info!(target: "service", upid = ?clone_upid, "Proxmox clone task started");
    let clone_task = TaskRef::new(&template_vm.node, &clone_upid);
    services::wait_until_finish(proxmox_client, clone_task, 1, None).await?;
    tracing::info!(target: "service", %new_vmid, "Proxmox VM cloned");

    // Save vmid to the database.
    let new_vm = VmRef::new(&template_vm.node, new_vmid);
    queries::update_initial_server(transaction, server_id, new_vm.clone()).await?;
    tracing::info!(target: "service", "Server record updated");

    // Setup configuration (IP, CPU, RAM).
    let config_upid = proxmox_client.vm_config(new_vm, vm_config).await?;
    tracing::info!(%server_id, upid = ?config_upid, "Proxmox config task started");

    let config_task = TaskRef::new(&template_vm.node, &config_upid);
    services::wait_until_finish(proxmox_client, config_task, 1, None).await?;
    tracing::info!(%server_id, %new_vmid, "VM configuration applied");

    queries::update_server_status(transaction.as_mut(), server_id, ServerStatus::Stopped).await?;
    queries::update_service_status(transaction.as_mut(), service_id, ServiceStatus::Active).await?;
    tracing::info!(target: "service", "Proxmox VM setup finished successfully");

    Ok(())
}
