use crate::config::CONFIG;
use crate::model::types::*;
use crate::proxmox::types::VmRef;
use crate::web::auth::password::hash;
use crate::web::types::NewServerPayload;
use dashboard_common::error::{Error, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, PgTransaction, Postgres};
use uuid::Uuid;

/// Creates and returns a connection pool to the database.
///
/// # Returns
///
/// A `PgPool` connection pool instance.
///
#[tracing::instrument(level = "trace", target = "database")]
pub async fn connect_to_db() -> Result<PgPool> {
    let database_url = &CONFIG.get_database_url();
    let pool = PgPoolOptions::new().connect(database_url).await?;

    Ok(pool)
}

/// Updates a user's password hash in the database.
///
/// # Arguments
///
/// * `pool`: Reference to the `PgPool`.
/// * `user_id`: UUID of the user whose password should be updated.
/// * `new_hash`: New, hashed password to be stored in the database.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
pub async fn update_password(pool: &PgPool, user_id: &Uuid, new_hash: &str) -> Result<()> {
    sqlx::query!(
        r#"
UPDATE users SET password = $2
WHERE id = $1
		"#,
        user_id,
        new_hash
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Inserts a new user into the database.
///
/// # Arguments
///
/// * `pool`: Reference to the `PgPool`.
/// * `new_user`: `NewUser` struct containing the new user's information.
///
/// # Returns
///
/// `ApiUser` struct representing the newly created user.
///
pub async fn add_new_user(pool: &PgPool, new_user: NewUser) -> Result<ApiUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
INSERT INTO users (
    first_name,
    last_name,
    email,
    address,
    city,
    state,
    post_code,
    country,
    phone_number,
    password)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
RETURNING
    id,
    first_name,
    last_name,
    email,
    address,
    city,
    state,
    post_code,
    country,
    phone_number,
    password,
    created_at,
    updated_at
		"#,
        new_user.first_name,
        new_user.last_name,
        new_user.email,
        new_user.address,
        new_user.city,
        new_user.state,
        new_user.post_code,
        new_user.country,
        new_user.phone_number,
        hash(&new_user.plain_password)?
    )
    .fetch_one(pool)
    .await?
    .into())
}

/// Retrieves a user from the database by their ID.
///
/// # Arguments
///
/// * `pool`: Reference to the `PgPool`.
/// * `user_id`: UUID of the user to retrieve.
///
/// # Returns
///
/// `ApiUser` struct for the found user.
///
pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<ApiUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
SELECT
    id,
    first_name,
    last_name,
    email,
    address,
    city,
    state,
    post_code,
    country,
    phone_number,
    password,
    created_at,
    updated_at
FROM users
WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?
    .into())
}

/// Retrieves a user from the database by their email address.
///
/// # Arguments
///
/// * `pool`: Reference to the `PgPool`.
/// * `email`: Email address of the user to retrieve.
///
/// # Returns
///
/// `DbUser` struct for the found user, including the password hash.
///
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<DbUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
SELECT
    id,
    first_name,
    last_name,
    email,
    address,
    city,
    state,
    post_code,
    country,
    phone_number,
    password,
    created_at,
    updated_at
FROM users
WHERE email = $1
        "#,
        email
    )
    .fetch_one(pool)
    .await?)
}

/// Retrieves all servers associated with a specific user.
///
/// # Arguments
///
/// * `pool`: Reference to the `PgPool`.
/// * `user_id`: UUID of the user whose servers are to be retrieved.
///
/// # Returns
///
/// `Vec<ApiServer>` containing the list of servers for the user.
///
pub async fn get_servers_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiServer>> {
    let rows = sqlx::query!(
        r#"
SELECT
	svc.id AS "service_id",
	srv.id AS "server_id",
	srv.vm_id,
	srv.node_name,
	ip.ip_address,
	srv.status
FROM services AS svc
JOIN servers AS srv ON srv.id = svc.server_id
INNER JOIN ip_addresses as ip ON ip.server_id = srv.id
WHERE svc.user_id = $1
		"#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ApiServer {
            service_id: row.service_id,
            server_id: row.server_id,
            vm_id: row.vm_id,
            node_name: row.node_name,
            ip_address: row.ip_address,
            status: row.status.as_str().into(),
        })
        .collect::<Vec<_>>())
}

/// Creates a new server record in the `servers` table.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `payload`: `NewServerPayload` containing details for the new server.
///
/// # Returns
///
/// UUID of the newly created server record.
///
pub async fn create_server_record(
    transaction: &mut PgTransaction<'_>,
    payload: &NewServerPayload,
) -> Result<Uuid> {
    let record = sqlx::query!(
        r#"
INSERT INTO servers (host_name, status)
VALUES ($1, $2)
RETURNING id
        "#,
        payload.host_name,
        ServerStatus::SettingUp.to_string(),
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(record.id)
}

/// Finds an available IP address and assigns it to a server.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `server_id`: UUID of the server to assign the IP to.
/// * `payload`: `NewServerPayload` containing the datacenter information.
///
/// # Returns
///
/// `IpConfig` struct with the reserved IP address details.
///
pub async fn reserve_ip_for_server(
    transaction: &mut PgTransaction<'_>,
    server_id: Uuid,
    payload: &NewServerPayload,
) -> Result<IpConfig> {
    // Find available IP address.
    let network_details = sqlx::query!(
        r#"
SELECT
	ip.id AS "ip_id",
	ip.ip_address,
	n.gateway,
	n.subnet_mask
FROM ip_addresses AS ip
JOIN networks AS n ON ip.network_id = n.id
WHERE ip.server_id IS NULL AND n.datacenter_name = $1
LIMIT 1
FOR UPDATE SKIP LOCKED
		"#,
        payload.datacenter,
    )
    .fetch_one(&mut **transaction)
    .await?;

    // Reserve IP address.
    sqlx::query!(
        r#"
UPDATE ip_addresses SET server_id = $1
WHERE id = $2
		"#,
        server_id,
        network_details.ip_id,
    )
    .execute(&mut **transaction)
    .await?;

    Ok(IpConfig {
        ip_address: network_details.ip_address,
        gateway: network_details.gateway,
        subnet_mask: network_details.subnet_mask,
    })
}

/// Creates a service record to link a user, server, and product.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `user_id`: UUID of the user.
/// * `server_id`: UUID of the server.
/// * `template_id`: UUID of the template.
/// * `payload`: `NewServerPayload` containing the product ID.
///
/// # Returns
///
/// UUID of the newly created service record.
///
pub async fn create_service_record(
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    server_id: Uuid,
    template_id: Uuid,
    payload: &NewServerPayload,
) -> Result<Uuid> {
    let record = sqlx::query!(
        r#"
INSERT INTO services (status, user_id, server_id, product_id, template_id)
VALUES ($1, $2, $3, $4, $5)
RETURNING id
        "#,
        ServiceStatus::Pending.to_string(),
        user_id,
        server_id,
        payload.product_id,
        template_id,
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(record.id)
}

/// Saves configurable option values (like CPU, RAM) for a service.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `service_id`: UUID of the service.
/// * `payload`: `NewServerPayload` containing the configuration values.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
pub async fn save_config_values(
    transaction: &mut PgTransaction<'_>,
    service_id: Uuid,
    payload: &NewServerPayload,
) -> Result<()> {
    let config_options = sqlx::query!(
        r#"
SELECT id, name FROM config_options
WHERE name IN ('cpu_cores', 'ram_gb')
        "#
    )
    .fetch_all(&mut **transaction)
    .await?;

    for option in config_options {
        let value = match option.name.as_str() {
            "cpu_cores" => payload.cpu_cores.unwrap_or(2).to_string(),
            "ram_gb" => payload.ram_gb.unwrap_or(2).to_string(),
            _ => continue,
        };

        sqlx::query!(
            r#"
INSERT INTO config_values (service_id, config_id, value)
VALUES ($1, $2, $3)
            "#,
            service_id,
            option.id,
            value
        )
        .execute(&mut **transaction)
        .await?;
    }

    Ok(())
}

/// Saves custom field values (like OS, Datacenter) for a service.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `service_id`: UUID of the service.
/// * `payload`: `NewServerPayload` containing the custom values.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
pub async fn save_custom_values(
    transaction: &mut PgTransaction<'_>,
    service_id: Uuid,
    payload: &NewServerPayload,
) -> Result<()> {
    let custom_fields = sqlx::query!(
        r#"
SELECT id, name FROM custom_fields
WHERE name IN ('os', 'datacenter')
        "#
    )
    .fetch_all(&mut **transaction)
    .await?;

    for field in custom_fields {
        let value = match field.name.as_str() {
            "os" => &payload.os,
            "datacenter" => &payload.datacenter,
            _ => continue,
        };

        sqlx::query!(
            r#"
INSERT INTO custom_values (service_id, custom_field_id, value)
VALUES ($1, $2, $3)
            "#,
            service_id,
            field.id,
            value
        )
        .execute(&mut **transaction)
        .await?;
    }

    Ok(())
}

/// Updates a server record with its initial Proxmox VM details.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `server_id`: UUID of the server to update.
/// * `new_vm`: `VmRef` struct containing the VM ID and node name.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
pub async fn update_initial_server(
    transaction: &mut PgTransaction<'_>,
    server_id: Uuid,
    new_vm: VmRef,
) -> Result<()> {
    sqlx::query!(
        r#"
UPDATE servers SET vm_id = $2, node_name = $3
WHERE id = $1
		"#,
        server_id,
        new_vm.id,
        new_vm.node,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

/// Finds the ID of a template by OS name.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `os_name`: Name of the OS to find the template for.
///
/// # Returns
///
/// UUID of the found template.
///
pub async fn find_template_id(transaction: &mut PgTransaction<'_>, os_name: &str) -> Result<Uuid> {
    let record = sqlx::query!(
        r#"
SELECT * FROM templates
WHERE os_name = $1
		"#,
        os_name,
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(record.id)
}

/// Finds the Proxmox template details for a given service.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `service_id`: UUID of the service.
///
/// # Returns
///
/// `VmRef` struct containing the template's node and VMID.
///
pub async fn find_template(transaction: &mut PgTransaction<'_>, service_id: Uuid) -> Result<VmRef> {
    let record = sqlx::query!(
        r#"
SELECT
	t.template_node,
	t.template_vmid
FROM services AS s
JOIN templates AS t ON t.id = s.template_id
WHERE s.id = $1
		"#,
        service_id
    )
    .fetch_one(&mut **transaction)
    .await?;

    let vm = VmRef::new(&record.template_node, record.template_vmid);
    Ok(vm)
}

/// Updates the status of a service.
///
/// # Arguments
///
/// * `executor`: Database executor (pool or transaction).
/// * `service_id`: UUID of the service to update.
/// * `status`: New `ServiceStatus`.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
pub async fn update_service_status<'e, E>(
    executor: E,
    service_id: Uuid,
    status: ServiceStatus,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query!(
        r#"
UPDATE services SET status = $2
WHERE id = $1
		"#,
        service_id,
        status.to_string(),
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Updates the status of a server.
///
/// # Arguments
///
/// * `executor`: Database executor (pool or transaction).
/// * `server_id`: UUID of the server to update.
/// * `status`: New `ServerStatus`.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
pub async fn update_server_status<'e, E>(
    executor: E,
    server_id: Uuid,
    status: ServerStatus,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query!(
        r#"
UPDATE servers SET status = $2
WHERE id = $1
		"#,
        server_id,
        status.to_string(),
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Retrieves a single server by its ID.
///
/// # Arguments
///
/// * `executor`: Database executor (pool or transaction).
/// * `user_id`: UUID of the user who owns the server.
/// * `server_id`: UUID of the server to retrieve.
///
/// # Returns
///
/// `ApiServer` struct for the found server.
///
pub async fn get_server_by_id<'e, E>(
    executor: E,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<ApiServer>
where
    E: Executor<'e, Database = Postgres>,
{
    let server = sqlx::query_as!(
        ApiServer,
        r#"
SELECT
	svc.id AS "service_id",
	srv.id AS "server_id",
	srv.vm_id,
	srv.node_name,
	ip.ip_address,
	srv.status
FROM services AS svc
JOIN servers AS srv ON srv.id = svc.server_id
INNER JOIN ip_addresses AS ip ON ip.server_id = srv.id
WHERE svc.user_id = $1 AND srv.id = $2
		"#,
        user_id,
        server_id,
    )
    .fetch_one(executor)
    .await?;

    Ok(server)
}

/// Deletes a server record and releases its associated IP address.
///
/// # Arguments
///
/// * `transaction`: Mutable reference to a `PgTransaction`.
/// * `server_id`: UUID of the server to delete.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
pub async fn delete_server_record(
    transaction: &mut PgTransaction<'_>,
    server_id: Uuid,
) -> Result<()> {
    // Clear IP address.
    sqlx::query!(
        r#"
UPDATE ip_addresses SET server_id = NULL
WHERE server_id = $1
        "#,
        server_id,
    )
    .execute(&mut **transaction)
    .await?;

    // Delete the server.
    sqlx::query!(
        r#"
DELETE FROM servers
WHERE id = $1
		"#,
        server_id,
    )
    .execute(&mut **transaction)
    .await?;

    Ok(())
}

/// Retrieves the Proxmox `VmRef` for a server owned by a user.
///
/// # Arguments
///
/// * `executor`: Database executor (pool or transaction).
/// * `user_id`: UUID of the user who owns the server.
/// * `server_id`: UUID of the server.
///
/// # Returns
///
/// `VmRef` struct for the server.
///
pub async fn get_server_proxmox_ref<'e, E>(
    executor: E,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<VmRef>
where
    E: Executor<'e, Database = Postgres>,
{
    let record = sqlx::query!(
        r#"
SELECT
	srv.vm_id,
	srv.node_name
FROM servers AS srv
JOIN services AS svc ON svc.server_id = srv.id
WHERE svc.user_id = $1 AND srv.id = $2
		"#,
        user_id,
        server_id,
    )
    .fetch_one(executor)
    .await?;

    match (record.node_name, record.vm_id) {
        (Some(node_name), Some(vm_id)) => Ok(VmRef::new(&node_name, vm_id)),
        _ => Err(Error::NotReady(format!("Server: {}", server_id))),
    }
}

// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::Server;
    use crate::web::types::NewServerPayload;

    #[sqlx::test(migrations = "../../migrations")]
    async fn update_password_should_works(pool: PgPool) {
        // Arrange
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let new_password = "new_secure_password";
        let new_hash = bcrypt::hash(&new_password, 10).unwrap();
        // Act
        update_password(&pool, &user.id, &new_hash).await.unwrap();
        // Assert
        let new_user = get_user_by_email(&pool, &user.email).await.unwrap();
        assert!(bcrypt::verify(&new_password, &new_user.password).is_ok());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn add_new_user_should_works(pool: PgPool) {
        // Arrange
        let test_user = payload::test_user();
        // Act
        let new_user = add_new_user(&pool, test_user.clone()).await.unwrap();
        // Assert
        assert_eq!(new_user.email, test_user.email);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn get_user_by_id_should_works(pool: PgPool) {
        // Arrange
        let test_user = payload::test_user();
        let new_user = add_new_user(&pool, test_user.clone()).await.unwrap();
        // Act
        let found_user = get_user_by_id(&pool, new_user.id).await.unwrap();
        // Assert
        assert_eq!(found_user.id, new_user.id);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn get_user_by_email_should_works(pool: PgPool) {
        // Arrange
        let test_user = payload::test_user();
        let new_user = add_new_user(&pool, test_user.clone()).await.unwrap();
        // Act
        let found_user = get_user_by_email(&pool, &new_user.email).await.unwrap();
        // Assert
        assert_eq!(found_user.email, new_user.email);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn get_servers_for_user_should_works(pool: PgPool) {
        // Arrange
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;
        create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();
        let network_id = helpers::test_network_id(&mut tx).await;
        helpers::test_ip_id(&mut tx, Some(server_id), network_id).await;
        tx.commit().await.unwrap();

        // Act
        let servers = get_servers_for_user(&pool, user.id).await.unwrap();

        // Assert
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].server_id, server_id);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn create_server_record_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let payload = payload::test_server(None);

        // Act
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();

        // Assert
        let server = helpers::test_get_server(&mut tx, server_id).await.unwrap();
        assert_eq!(server.id, server_id);
        assert_eq!(server.host_name, payload.host_name);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn reserve_ip_for_server_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let network_id = helpers::test_network_id(&mut tx).await;
        let ip_id = helpers::test_ip_id(&mut tx, None, network_id).await;

        // Act
        let ip_config = reserve_ip_for_server(&mut tx, server_id, &payload)
            .await
            .unwrap();

        // Assert
        assert_eq!(ip_config.ip_address, "10.0.0.101");
        let ip_server_id = helpers::test_get_server_id_from_ip(&mut tx, ip_id).await;
        assert_eq!(ip_server_id, Some(server_id));
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn create_service_record_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;

        // Act
        let service_id = create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();
        let service = sqlx::query!("SELECT * FROM services WHERE id = $1", service_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();

        // Assert
        assert_eq!(service.user_id, user.id);
        assert_eq!(service.server_id, server_id);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn save_config_values_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;
        let service_id = create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();
        let cpu_option_id = helpers::test_config_option(&mut tx, "cpu_cores").await;
        let ram_option_id = helpers::test_config_option(&mut tx, "ram_gb").await;

        // Act
        save_config_values(&mut tx, service_id, &payload)
            .await
            .unwrap();

        // Assert
        let cpu_value = helpers::test_config_value(&mut tx, service_id, cpu_option_id).await;
        assert_eq!(cpu_value, payload.cpu_cores.unwrap().to_string());
        let ram_value = helpers::test_config_value(&mut tx, service_id, ram_option_id).await;
        assert_eq!(ram_value, payload.ram_gb.unwrap().to_string());
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn save_custom_values_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;
        let service_id = create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();

        let os_field_id = helpers::test_custom_option(&mut tx, "os", product_id).await;
        let dc_field_id = helpers::test_custom_option(&mut tx, "datacenter", product_id).await;

        // Act
        save_custom_values(&mut tx, service_id, &payload)
            .await
            .unwrap();

        // Assert
        let os_value = helpers::test_custom_value(&mut tx, service_id, os_field_id).await;
        assert_eq!(os_value, payload.os);
        let dc_value = helpers::test_custom_value(&mut tx, service_id, dc_field_id).await;
        assert_eq!(dc_value, payload.datacenter);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn update_initial_server_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let payload = payload::test_server(None);
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let vm_ref = VmRef::new("test-node", 123);

        // Act
        update_initial_server(&mut tx, server_id, vm_ref.clone())
            .await
            .unwrap();

        // Assert
        let server = helpers::test_get_server(&mut tx, server_id).await.unwrap();
        assert_eq!(server.vm_id, Some(vm_ref.id));
        assert_eq!(server.node_name, Some(vm_ref.node));
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn find_template_id_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let expected_id = helpers::test_template_id(&mut tx).await;

        // Act
        let found_id = find_template_id(&mut tx, "os").await.unwrap();

        // Assert
        assert_eq!(found_id, expected_id);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn find_template_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;
        let service_id = create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();

        // Act
        let vm_ref = find_template(&mut tx, service_id).await.unwrap();

        // Assert
        assert_eq!(vm_ref.node, "node");
        assert_eq!(vm_ref.id, 1);
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn update_service_status_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;
        let service_id = create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();
        let new_status = ServiceStatus::Active;

        // Act
        update_service_status(&mut *tx, service_id, new_status.clone())
            .await
            .unwrap();

        // Assert
        let service = sqlx::query!("SELECT status FROM services WHERE id = $1", service_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(service.status, new_status.to_string());
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn update_server_status_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let payload = payload::test_server(None);
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let new_status = ServerStatus::Stopped;

        // Act
        update_server_status(&mut *tx, server_id, new_status.clone())
            .await
            .unwrap();

        // Assert
        let server = sqlx::query!("SELECT status FROM servers WHERE id = $1", server_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(server.status, new_status.to_string());
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn get_server_by_id_should_works(pool: PgPool) {
        // Arrange
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;
        create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();
        let network_id = helpers::test_network_id(&mut tx).await;
        helpers::test_ip_id(&mut tx, Some(server_id), network_id).await;
        tx.commit().await.unwrap();

        // Act
        let server = get_server_by_id(&pool, user.id, server_id).await.unwrap();

        // Assert
        assert_eq!(server.server_id, server_id);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn delete_server_record_should_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let payload = payload::test_server(None);
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let network_id = helpers::test_network_id(&mut tx).await;
        let ip_id = helpers::test_ip_id(&mut tx, Some(server_id), network_id).await;

        // Act
        delete_server_record(&mut tx, server_id).await.unwrap();

        // Assert
        let server = helpers::test_get_server(&mut tx, server_id).await;
        assert!(server.is_none());
        let ip_server_id = helpers::test_get_server_id_from_ip(&mut tx, ip_id).await;
        assert!(ip_server_id.is_none());
        tx.commit().await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn get_server_proxmox_ref_should_works(pool: PgPool) {
        // Arrange
        let user = add_new_user(&pool, payload::test_user()).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        let product_id = helpers::test_product(&mut tx).await;
        let payload = payload::test_server(Some(product_id));
        let server_id = create_server_record(&mut tx, &payload).await.unwrap();
        let template_id = helpers::test_template_id(&mut tx).await;
        create_service_record(&mut tx, user.id, server_id, template_id, &payload)
            .await
            .unwrap();

        let vm_ref = VmRef::new("test-node", 42);
        update_initial_server(&mut tx, server_id, vm_ref.clone())
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // Act
        let found_ref = get_server_proxmox_ref(&pool, user.id, server_id)
            .await
            .unwrap();

        // Assert
        assert_eq!(found_ref.id, vm_ref.id);
        assert_eq!(found_ref.node, vm_ref.node);
    }

    // -------------------------------------------------------------------------

    pub mod payload {
        use super::*;

        pub fn test_user() -> NewUser {
            NewUser {
                first_name: "John".to_owned(),
                last_name: "Doe".to_owned(),
                email: "john.doe@example.com".to_owned(),
                address: "123 Main St".to_owned(),
                city: "Anytown".to_owned(),
                state: "Any-state".to_owned(),
                post_code: "12345".to_owned(),
                country: "USA".to_owned(),
                phone_number: "555-1234".to_owned(),
                plain_password: "secure_password_123".to_owned(),
            }
        }

        pub fn test_server(product_id: Option<Uuid>) -> NewServerPayload {
            NewServerPayload {
                product_id: product_id.unwrap_or(Uuid::new_v4()),
                os: "ubuntu".to_owned(),
                datacenter: "dc-1".to_owned(),
                host_name: "test-server".to_owned(),
                cpu_cores: Some(4),
                ram_gb: Some(8),
                ip_config: None,
            }
        }
    }

    // -------------------------------------------------------------------------

    pub mod helpers {
        use super::*;

        pub async fn test_product(transaction: &mut PgTransaction<'_>) -> Uuid {
            let group_id = sqlx::query!(
                r#"
INSERT INTO product_groups (name)
VALUES ('g1')
RETURNING id "#
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .id;

            sqlx::query!(
                r#"
INSERT INTO products (group_id, name)
VALUES ($1, 'p1')
RETURNING id"#,
                group_id
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .id
        }

        pub async fn test_template_id(transaction: &mut PgTransaction<'_>) -> Uuid {
            sqlx::query!(
                r#"
INSERT INTO templates (os_name, template_vmid, template_node, virtual_type)
VALUES ('os', 1, 'node', 'qemu')
RETURNING id"#
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .id
        }

        pub async fn test_network_id(transaction: &mut PgTransaction<'_>) -> Uuid {
            sqlx::query!(
                r#"
INSERT INTO networks (datacenter_name, gateway, subnet_mask)
VALUES ('dc-1', '10.0.0.1', '255.255.255.0')
RETURNING id"#
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .id
        }

        pub async fn test_ip_id(
            transaction: &mut PgTransaction<'_>,
            server_id: Option<Uuid>,
            network_id: Uuid,
        ) -> Uuid {
            sqlx::query!(
                r#"
INSERT INTO ip_addresses (ip_address, network_id, server_id)
VALUES ('10.0.0.101', $1, $2)
RETURNING id"#,
                network_id,
                server_id
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .id
        }

        pub async fn test_get_server(
            transaction: &mut PgTransaction<'_>,
            server_id: Uuid,
        ) -> Option<Server> {
            sqlx::query!(
                r#"
SELECT * FROM servers
WHERE id = $1"#,
                server_id
            )
            .fetch_optional(transaction.as_mut())
            .await
            .unwrap()
            .map(|rec| Server {
                id: rec.id,
                vm_id: rec.vm_id,
                node_name: rec.node_name,
                host_name: rec.host_name,
            })
        }

        pub async fn test_get_server_id_from_ip(
            transaction: &mut PgTransaction<'_>,
            ip_id: Uuid,
        ) -> Option<Uuid> {
            sqlx::query!(
                r#"
SELECT server_id FROM ip_addresses
WHERE id = $1"#,
                ip_id
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .server_id
        }

        pub async fn test_config_option(
            transaction: &mut PgTransaction<'_>,
            field_name: &str,
        ) -> Uuid {
            sqlx::query!(
                r#"
INSERT INTO config_options (name)
VALUES ($1) RETURNING id"#,
                field_name
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .id
        }

        pub async fn test_config_value(
            transaction: &mut PgTransaction<'_>,
            service_id: Uuid,
            option_id: Uuid,
        ) -> String {
            sqlx::query!(
                r#"
SELECT value FROM config_values
WHERE service_id = $1 AND config_id = $2"#,
                service_id,
                option_id
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .value
        }

        pub async fn test_custom_option(
            transaction: &mut PgTransaction<'_>,
            field_name: &str,
            product_id: Uuid,
        ) -> Uuid {
            sqlx::query!(
                r#"
INSERT INTO custom_fields (name, product_id)
VALUES ($1, $2) RETURNING id"#,
                field_name,
                product_id
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .id
        }

        pub async fn test_custom_value(
            transaction: &mut PgTransaction<'_>,
            service_id: Uuid,
            field_id: Uuid,
        ) -> String {
            sqlx::query!(
                r#"
SELECT value FROM custom_values
WHERE service_id = $1 AND custom_field_id = $2"#,
                service_id,
                field_id
            )
            .fetch_one(transaction.as_mut())
            .await
            .unwrap()
            .value
        }
    }
}
