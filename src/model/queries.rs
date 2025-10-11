use crate::config::CONFIG;
use crate::model::types::{ApiServer, ApiUser, DbUser, NewUser, ServiceStatus};
use crate::prelude::Result;
use crate::proxmox::types::VmRef;
use crate::web::auth::password::hash;
use crate::web::types::NewServerPayload;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, PgTransaction};
use uuid::Uuid;

#[tracing::instrument(level = "trace", target = "-- database")]
pub async fn connect_to_db() -> Result<PgPool> {
    let database_url = &CONFIG.get_database_url();
    let pool = PgPoolOptions::new().connect(database_url).await?;

    Ok(pool)
}

pub(crate) async fn add_new_user(pool: &PgPool, new_user: NewUser) -> Result<ApiUser> {
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
RETURNING *
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

pub(crate) async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<ApiUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
SELECT * FROM users WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?
    .into())
}

pub(crate) async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<DbUser> {
    Ok(sqlx::query_as!(
        DbUser,
        r#"
SELECT * FROM users WHERE email = $1
        "#,
        email
    )
    .fetch_one(pool)
    .await?)
}

pub(crate) async fn get_servers_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiServer>> {
    let rows = sqlx::query!(
        r#"
SELECT
	svc.id AS "service_id",
	srv.id AS "server_id",
	srv.vm_id,
	srv.node_name,
	ip.ip_address,
	svc.status
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

/// Record in the `servers` table.
///
pub(crate) async fn create_server_record(
    transaction: &mut PgTransaction<'_>,
    payload: &NewServerPayload,
) -> Result<Uuid> {
    let record = sqlx::query!(
        r#"
INSERT INTO servers (host_name)
VALUES ($1)
RETURNING id
        "#,
        payload.host_name
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(record.id)
}

/// Find an available IP and assign it to the new server.
///
pub(crate) async fn reserve_ip_for_server(
    transaction: &mut PgTransaction<'_>,
    server_id: Uuid,
    payload: &NewServerPayload,
) -> Result<String> {
    let record = sqlx::query!(
        r#"
WITH available_ip AS (
	SELECT ip.id, ip.ip_address
	FROM ip_addresses AS ip
	JOIN network AS n ON ip.network_id = n.id
	WHERE ip.server_id IS NULL AND n.datacenter_name = $1
	LIMIT 1
	FOR UPDATE SKIP LOCKED
)
UPDATE ip_addresses SET server_id = $2
WHERE id = (SELECT id FROM available_ip)
RETURNING ip_address
		"#,
        payload.data_center,
        server_id,
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(record.ip_address)
}

/// Create a `services` record to link the user, server, and product.
///
pub(crate) async fn create_service_record(
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    server_id: Uuid,
    payload: &NewServerPayload,
) -> Result<Uuid> {
    let record = sqlx::query!(
        r#"
INSERT INTO services (status, user_id, server_id, product_id)
VALUES ($1, $2, $3, $4)
RETURNING id
        "#,
        ServiceStatus::Pending.to_string(),
        user_id,
        server_id,
        payload.product_id
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(record.id)
}

/// Configurable options (CPU, RAM).
///
pub(crate) async fn save_config_values(
    transaction: &mut PgTransaction<'_>,
    service_id: Uuid,
    payload: &NewServerPayload,
) -> Result<()> {
    let config_options = sqlx::query!(
        r#"
SELECT id, name FROM config_options WHERE name IN ('cpu_cores', 'ram_gb')
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

/// Custom fields (OS, Datacenter).
///
pub(crate) async fn save_custom_values(
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
            "datacenter" => &payload.data_center,
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

pub(crate) async fn update_initial_server(
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

pub(crate) async fn find_template(
    transaction: &mut PgTransaction<'_>,
    product_id: Uuid,
) -> Result<VmRef> {
    let record = sqlx::query!(
        r#"
SELECT * FROM products
WHERE id = $1
		"#,
        product_id
    )
    .fetch_one(&mut **transaction)
    .await?;

    let vm = VmRef::new(&record.template_node, record.template_id);
    Ok(vm)
}

pub(crate) async fn update_service_status(
    transaction: &mut PgTransaction<'_>,
    service_id: Uuid,
    status: ServiceStatus,
) -> Result<()> {
    sqlx::query!(
        r#"
UPDATE services SET status = $2
WHERE id = $1
		"#,
        service_id,
        status.to_string(),
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_user() -> NewUser {
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

    #[sqlx::test]
    async fn add_new_user_should_works(pool: PgPool) {
        // Arrange
        let test_user = test_user();
        // Act
        let new_user = add_new_user(&pool, test_user.clone()).await.unwrap();
        // Assert
        assert_eq!(new_user.email, test_user.email);
    }

    #[sqlx::test]
    async fn get_user_by_id_should_works(pool: PgPool) {
        // Arrange
        let test_user = test_user();
        let new_user = add_new_user(&pool, test_user.clone()).await.unwrap();
        // Act
        let found_user = get_user_by_id(&pool, new_user.id).await.unwrap();
        // Assert
        assert_eq!(found_user.id, new_user.id);
    }

    #[sqlx::test]
    async fn get_user_by_email_should_works(pool: PgPool) {
        // Arrange
        let test_user = test_user();
        let new_user = add_new_user(&pool, test_user.clone()).await.unwrap();
        // Act
        let found_user = get_user_by_email(&pool, &new_user.email).await.unwrap();
        // Assert
        assert_eq!(found_user.email, new_user.email);
    }
}
