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
	srv.ip_address,
	svc.status
FROM services AS svc
JOIN servers AS srv ON srv.id = svc.server_id
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

pub(crate) async fn create_initial_server(
    transaction: &mut PgTransaction<'_>,
    user_id: Uuid,
    payload: &NewServerPayload,
) -> Result<ApiServer> {
    todo!()
}

pub(crate) async fn update_initial_server(
    transaction: &mut PgTransaction<'_>,
    server_id: Uuid,
    new_vm: VmRef,
) -> Result<()> {
    todo!()
}

pub(crate) async fn find_template(
    transaction: &mut PgTransaction<'_>,
    product_id: Uuid,
) -> Result<VmRef> {
    todo!("Update products table to")
    // CREATE TABLE products
    // 	(
    //      id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    //      group_id     UUID NOT NULL REFERENCES product_groups (id),
    //      name         TEXT NOT NULL,
    //      virtual_type TEXT NOT NULL
    //      template_id   INTEGER NOT NULL, +
    //      template_node TEXT NOT NULL, +
    // 	);
}

pub(crate) async fn update_service_status(
    transaction: &mut PgTransaction<'_>,
    service_id: Uuid,
    status: ServiceStatus,
) -> Result<()> {
    todo!()
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
