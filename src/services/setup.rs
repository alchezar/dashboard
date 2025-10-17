use crate::model::queries;
use crate::model::types::{ApiServer, ServerStatus, ServiceStatus};
use crate::prelude::{AppState, Result};
use crate::proxmox::types::{TaskRef, VmConfig, VmRef};
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
pub async fn run(app_state: AppState, user_id: Uuid, mut payload: NewServerPayload) {
    // Create a transaction for a chain of all sequential queries.
    let Ok(mut transaction) = app_state.pool.begin().await else {
        tracing::error!(target: "service", "Failed to begin transaction!");
        return;
    };

    let result = async {
        // Initial records in the database.
        let (server, vm_config) =
            create_initial_server(&mut transaction, user_id, &mut payload).await?;
        tracing::info!(target: "service", service_id = %server.service_id, server_id = %server.server_id, "Server record created");

        // Setup proxmox server.
        service_setup(
            &mut transaction,
            &app_state,
			vm_config,
            &server,
        )
        .await?;
        queries::update_server_status(&mut *transaction, server.server_id, ServerStatus::Stopped).await?;
        queries::update_service_status(&mut *transaction, server.service_id, ServiceStatus::Active)
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
    payload: &mut NewServerPayload,
) -> Result<(ApiServer, VmConfig)> {
    let server_id = queries::create_server_record(transaction, payload).await?;
    let template_id = queries::find_template_id(transaction, &payload.os).await?;
    let service_id =
        queries::create_service_record(transaction, user_id, server_id, template_id, payload)
            .await?;
    queries::save_config_values(transaction, service_id, payload).await?;
    queries::save_custom_values(transaction, service_id, payload).await?;

    let ip_config = queries::reserve_ip_for_server(transaction, server_id, payload).await?;
    let ip_address = ip_config.ip_address.clone();
    let vm_config = VmConfig::new(ip_config.form()?, payload.cpu_cores, payload.ram_gb);

    tracing::info!(target: "service", %server_id, %service_id, "Initial DB records created successfully");
    let server = ApiServer {
        server_id,
        service_id,
        vm_id: None,
        node_name: None,
        ip_address,
        status: ServerStatus::SettingUp,
    };

    Ok((server, vm_config))
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
#[tracing::instrument(level = "trace", target = "service", skip(transaction, app_state))]
async fn service_setup(
    transaction: &mut PgTransaction<'_>,
    app_state: &AppState,
    vm_config: VmConfig,
    server: &ApiServer,
) -> Result<()> {
    // Find template vm in the database.
    let template_vm: VmRef = queries::find_template(transaction, &server.service_id).await?;
    tracing::info!(target: "service", template_vmid = %template_vm.id, "Found VM template");

    // Clone new Proxmox server.
    let (new_vmid, clone_upid) = app_state.proxmox.create(template_vm.clone()).await?;
    tracing::info!(target: "service", upid = ?clone_upid, "Proxmox clone task started");
    let clone_task = TaskRef::new(&template_vm.node, &clone_upid);
    services::wait_until_finish(&app_state.proxmox, clone_task, 1, None).await?;
    tracing::info!(target: "service", %new_vmid, "VM cloned successfully");

    // Save vmid to the database.
    let new_vm = VmRef::new(&template_vm.node, new_vmid);
    queries::update_initial_server(transaction, server.server_id, new_vm.clone()).await?;
    tracing::info!(target: "service", "Server record updated");

    // Setup configuration (IP, CPU, RAM).
    let config_upid = app_state.proxmox.vm_config(new_vm, vm_config).await?;
    tracing::info!(server_id = %server.server_id, upid = ?config_upid, "Proxmox config task started");

    let config_task = TaskRef::new(&template_vm.node, &config_upid);
    services::wait_until_finish(&app_state.proxmox, config_task, 1, None).await?;
    tracing::info!(server_id = %server.server_id, %new_vmid, "VM configuration applied successfully");

    Ok(())
}
