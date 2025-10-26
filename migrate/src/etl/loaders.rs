//! This module is responsible for the "Load" phase of the migration pipeline.
//!
//! It contains a collection of functions, each tailored to bulk-insert a
//! specific type of data (e.g., users, products, services) into the target
//! PostgreSQL database.
//! These loaders receive data deserialized from the source database, handle any
//! final transformations (such as mapping legacy IDs to new UUIDs), and execute
//! efficient `INSERT` operations within the context of a single database
//! transaction.

use crate::etl::types;
use dashboard_common::error::Result;
use sqlx::PgTransaction;
use std::collections::HashMap;
use uuid::Uuid;

/// Performs a bulk `INSERT ... ON CONFLICT DO NOTHING` operation using
/// PostgreSQL's `UNNEST` function.
///
/// Efficiently inserts multiple items from an iterator into a specified
/// database table. Conflicts on unique constraints are handled by doing
/// nothing.
///
/// # Arguments
///
/// * `$iterator`: Iterator over the items to be inserted.
/// * `$db_table_name`: Name of the target database table.
/// * `$executor`: An expression that evaluates to a `&mut sqlx::Executor`.
///
/// * `$index`: Index for the `UNNEST` placeholder.
/// * `$item_field`: The field name for structs or index for tuples.
/// * `$db_field`: Database column name where this data will be inserted.
/// * `$sql_type`: PostgreSQL SQL type for the array.
///
/// # Returns
///
/// Expands to an expression of type `Result<u64>`, representing the number of
/// rows affected by the `INSERT` operation.
///
macro_rules! unnest_insert {
    (
        $iterator:expr =>
        $db_table_name:ident =>
        $executor:expr,
        [
            (
                $first_index:literal,
                $first_item_field:tt,
                $first_db_field:ident,
                $first_sql_type:ident
            ),
            $((
                $index:literal,
                $item_field:tt,
                $db_field:ident,
                $sql_type:ident
            )),* $(,)?
        ]
    ) => {{
        let items = $iterator.collect::<Vec<_>>();
        let count = items.len();

        // Collect all fields.
        let mut $first_db_field = Vec::with_capacity(count);
        $(let mut $db_field = Vec::with_capacity(count);)*
        for item in items {
            $first_db_field.push(item.$first_item_field);
            $($db_field.push(item.$item_field);)*
        }
        tracing::trace!("Collecting all fields completed.");

        // Insert collected items.
        let mut builder = sqlx::QueryBuilder::<sqlx::Postgres>::default();
        let query = builder
            .push("INSERT INTO ")
            .push(stringify!($db_table_name))
            .push(" (")
            .push(stringify!($first_db_field))
        $(
            .push(", ")
            .push(stringify!($db_field))
        )*
            .push(")\nSELECT * FROM UNNEST(")
            .push(format!("${}::{}[]", $first_index, stringify!($first_sql_type)))
        $(
            .push(", ")
            .push(format!("${}::{}[]", $index, stringify!($sql_type)))
        )*
            .push(")\nON CONFLICT DO NOTHING")
            .build();

        query
            .bind(&$first_db_field)
        $(
            .bind(&$db_field)
        )*
            .execute($executor.as_mut())
            .await
            .map(|result| result.rows_affected())
    }};
}

/// Helper function to bulk insert users into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `clients`: Vector of `whmcs::Client` structs to be inserted.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_users(tx: &mut PgTransaction<'_>, clients: Vec<types::Client>) -> Result<u64> {
    Ok(unnest_insert!(
        clients.into_iter() => users => tx,
        [
            (1, id, whmcs_id, int4),
            (2, firstname, first_name, text),
            (3, lastname, last_name, text),
            (4, email, email, text),
            (5, address1, address, text),
            (6, city, city, text),
            (7, state, state, text),
            (8, postcode, post_code, text),
            (9, country, country, text),
            (10, phonenumber, phone_number, text),
            (11, password, password, text),
        ]
    )?)
}

/// Helper function to bulk insert product groups into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `groups`: Vector of `whmcs::ProductGroup` structs to be inserted.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_product_groups(
    tx: &mut PgTransaction<'_>,
    groups: Vec<types::ProductGroup>,
) -> Result<u64> {
    Ok(unnest_insert!(
        groups.into_iter() => product_groups => tx,
        [(1, name, name, text), (2, id, whmcs_id, int4)]
    )?)
}

/// Helper function to bulk insert products into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `products`: Vector of `whmcs::Product` structs to be inserted.
/// * `group_id_map`: Relationship between the WHMCS id and the Dashboard id for
///   product groups.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_products(
    tx: &mut PgTransaction<'_>,
    products: Vec<types::Product>,
    group_id_map: &HashMap<i32, Uuid>,
) -> Result<u64> {
    #[rustfmt::skip]
    let fields_iter = products
        .into_iter()
        .filter_map(|product| match group_id_map.get(&product.gid) {
            Some(group_uuid) => Some((*group_uuid, product.name, product.id)),
            _ => {
                tracing::warn!(product_id = ?product.id, group_id = ?product.gid,
                    "Skipping product with non-migrated product group." );
                None
            }
        });

    Ok(unnest_insert!(
        fields_iter => products => tx,
        [
            (1, 0, group_id, uuid),
            (2, 1, name, text),
            (3, 2, whmcs_id, int4)
        ]
    )?)
}

/// Helper function to bulk insert custom fields into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `fields`: Vector of `whmcs::CustomField` structs to be inserted.
/// * `product_id_map`:  Relationship between the WHMCS id and the Dashboard id
///   for products.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_custom_fields(
    tx: &mut PgTransaction<'_>,
    fields: Vec<types::CustomField>,
    product_id_map: &HashMap<i32, Uuid>,
) -> Result<u64> {
    #[rustfmt::skip]
    let fields_iter = fields
        .into_iter()
        .filter_map(|field| match product_id_map.get(&field.relid) {
            Some(product_uuid) => Some((*product_uuid, field.fieldname, field.id)),
            _ => {
                tracing::warn!(custom_field_id = ?field.id, product_id = ?field.relid,
                    "Skipping custom field with non-migrated product." );
                None
            }
        });

    Ok(unnest_insert!(
        fields_iter => custom_fields => tx,
        [
            (1, 0, product_id, uuid),
            (2, 1, name, text),
            (3, 2, whmcs_id, int4)
        ]
    )?)
}

/// Helper function to bulk insert config options into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `options`: Vector of `whmcs::ConfigOption` structs to be inserted.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_config_options(
    tx: &mut PgTransaction<'_>,
    options: Vec<types::ConfigOption>,
) -> Result<u64> {
    Ok(unnest_insert!(
        options.into_iter() => config_options => tx,
        [(1, optionname, name, text), (2, id, whmcs_id, int4)]
    )?)
}

/// Helper function to bulk insert servers into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `vm_records`: Vector of `whmcs::VmRecord` structs to be inserted.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_servers(
    tx: &mut PgTransaction<'_>,
    vm_records: Vec<types::VmRecord>,
) -> Result<u64> {
    let servers = vm_records
        .into_iter()
        .map(types::Server::from)
        .collect::<Vec<types::Server>>();

    Ok(unnest_insert!(
        servers.into_iter() => servers => tx,
        [
            (1, vmid, vm_id, int8),
            (2, node, node_name, text),
            (3, hostname, host_name, text),
            (4, status, status, text),
            (5, id, whmcs_id, int4),
        ]
    )?)
}

/// Helper function to bulk insert networks into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `networks`: Vector of `whmcs::Network` structs to be inserted.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_networks(
    tx: &mut PgTransaction<'_>,
    networks: Vec<types::Network>,
) -> Result<u64> {
    Ok(unnest_insert!(
        networks.into_iter() => networks => tx,
        [
            (1, title, datacenter_name, text),
            (2, gateway, gateway, text),
            (3, mask, subnet_mask, text),
            (4, id, whmcs_id, int4),
        ]
    )?)
}

/// Helper function to bulk insert ip addresses into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `address`: Vector of `whmcs::IpAddress` structs to be inserted.
/// * `server_map`: WHMCS ID to Dashboard UUID relationship for servers.
/// * `network_map`: WHMCS ID to Dashboard UUID relationship for networks.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_ip_addresses(
    tx: &mut PgTransaction<'_>,
    address: Vec<types::IpAddress>,
    server_map: &HashMap<i32, Uuid>,
    network_map: &HashMap<i32, Uuid>,
) -> Result<u64> {
    let addresses_iter = address
        .into_iter()
        .filter_map(|field| match network_map.get(&field.pool_id) {
            Some(network_uuid) => Some((field.ipaddress, *network_uuid, field.server_id, field.id)),
            _ => {
                tracing::warn!(ip_address_id = ?field.id, network_id = ?field.pool_id,
                    "Skipping ip address with non-migrated network." );
                None
            }
        })
        .map(|(ip_address, network_uuid, server_id, id)| {
            let server_uuid = server_id
                .map(|unsigned_id| unsigned_id as i32)
                .and_then(|id| server_map.get(&id).copied());
            (ip_address, network_uuid, server_uuid, id)
        });

    Ok(unnest_insert!(
        addresses_iter => ip_addresses => tx,
        [
            (1, 0, ip_address, text),
            (2, 1, network_id, uuid),
            (3, 2, server_id, uuid),
            (4, 3, whmcs_id, int4),
        ]
    )?)
}

/// Helper function to bulk insert templates into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `temp_fields`: Vector of `whmcs::TemplateFields` structs to be inserted.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_templates(
    tx: &mut PgTransaction<'_>,
    temp_fields: Vec<types::TemplateField>,
) -> Result<u64> {
    let templates = temp_fields
        .into_iter()
        .flat_map(types::TemplateField::extract)
        .collect::<Vec<_>>();

    Ok(unnest_insert!(
        templates.into_iter() => templates => tx,
        [
            (1, os_name, os_name, text),
            (2, template_vmid, template_vmid, int4),
            (3, template_node, template_node, text),
            (4, virtual_type, virtual_type, text),
        ]
    )?)
}

/// Helper function to bulk insert services into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `services`: Vector of `whmcs::Service` structs to be inserted.
/// * `user_map`: WHMCS ID to Dashboard UUID relationship for users.
/// * `server_map`: WHMCS ID to Dashboard UUID relationship for servers.
/// * `product_map`: WHMCS ID to Dashboard UUID relationship for products.
/// * `template_map`: WHMCS ID to Dashboard UUID relationship for templates.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_services(
    tx: &mut PgTransaction<'_>,
    services: Vec<types::Service>,
    user_map: &HashMap<i32, Uuid>,
    server_map: &HashMap<i32, Uuid>,
    product_map: &HashMap<i32, Uuid>,
    template_map: &HashMap<i32, Uuid>,
) -> Result<u64> {
    let iter = services.into_iter().filter_map(|service| {
        let user_uuid = *user_map.get(&service.userid)?;
        let product_uuid = *product_map.get(&service.packageid)?;
        let server_uuid = *server_map.get(&service.id)?;
        let template_uuid = *template_map.get(&service.packageid)?;

        Some((
            service.domainstatus,
            user_uuid,
            server_uuid,
            product_uuid,
            template_uuid,
            service.id,
        ))
    });

    Ok(unnest_insert!(
        iter => services => tx,
        [
            (1, 0, status, text),
            (2, 1, user_id, uuid),
            (3, 2, server_id, uuid),
            (4, 3, product_id, uuid),
            (5, 4, template_id, uuid),
            (6, 5, whmcs_id, int4),
        ]
    )?)
}

/// Helper function to bulk insert services into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `values`: Vector of `whmcs::CustomValue` structs to be inserted.
/// * `service_map`: WHMCS ID to Dashboard UUID relationship for services.
/// * `custom_map`: WHMCS ID to Dashboard UUID relationship for custom fields.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_custom_values(
    tx: &mut PgTransaction<'_>,
    values: Vec<types::CustomValue>,
    service_map: &HashMap<i32, Uuid>,
    custom_map: &HashMap<i32, Uuid>,
) -> Result<u64> {
    let iter = values.into_iter().filter_map(|value| {
        let service_uuid = *service_map.get(&value.relid)?;
        let config_uuid = *custom_map.get(&value.fieldid)?;
        let whmcs_id = value.id as i32;
        Some((service_uuid, config_uuid, value.value, whmcs_id))
    });

    Ok(unnest_insert!(
        iter => custom_values => tx,
        [
            (1, 0, service_id, uuid),
            (2, 1, custom_field_id, uuid),
            (3, 2, value, text),
            (4, 3, whmcs_id, int4),
        ]
    )?)
}

/// Helper function to bulk insert services into the target database.
///
/// # Arguments
///
/// * `tx`: In-progress transaction for target database.
/// * `values`: Vector of `whmcs::ConfigValue` structs to be inserted.
/// * `service_map`: WHMCS ID to Dashboard UUID relationship for services.
/// * `config_map`: WHMCS ID to Dashboard UUID relationship for config options.
///
/// # Returns
///
/// On success, the number of affected rows.
///
pub async fn insert_config_values(
    tx: &mut PgTransaction<'_>,
    values: Vec<types::ConfigValue>,
    service_map: &HashMap<i32, Uuid>,
    config_map: &HashMap<i32, Uuid>,
) -> Result<u64> {
    let iter = values.into_iter().filter_map(|value| {
        let service_uuid = *service_map.get(&value.relid)?;
        let config_uuid = *config_map.get(&value.configid)?;
        let name = value
            .optionname
            .chars()
            .take_while(|char| char.is_numeric())
            .collect::<String>();
        Some((service_uuid, config_uuid, name, value.id))
    });

    Ok(unnest_insert!(
        iter => config_values => tx,
        [
            (1, 0, service_id, uuid),
            (2, 1, config_id, uuid),
            (3, 2, value, text),
            (4, 3, whmcs_id, int4),
        ]
    )?)
}

// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::etl::types::ProductGroup;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_users_works(pool: PgPool) {
        // Arrange
        let clients = vec![types::Client {
            id: 1,
            firstname: "John".to_owned(),
            lastname: "Doe".to_owned(),
            email: "john.doe@example.com".to_owned(),
            address1: "123 Main St".to_owned(),
            city: "Anytown".to_owned(),
            state: "CA".to_owned(),
            postcode: "12345".to_owned(),
            country: "US".to_owned(),
            phonenumber: "555-1234".to_owned(),
            password: "password123".to_owned(),
        }];
        let mut tx = pool.begin().await.unwrap();

        // Act
        let affected_rows = insert_users(&mut tx, clients).await.unwrap();

        // Assert
        let user = sqlx::query!("SELECT email FROM users WHERE whmcs_id = 1")
            .fetch_one(tx.as_mut())
            .await
            .unwrap();

        assert_eq!(affected_rows, 1);
        assert_eq!(user.email, "john.doe@example.com");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_insert_product_groups(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let groups = vec![
            ProductGroup::new(1, "Group1"),
            ProductGroup::new(2, "Group2"),
        ];

        // Act
        let affected_rows = insert_product_groups(&mut tx, groups).await.unwrap();

        // Assert
        let inserted_groups = sqlx::query!("SELECT whmcs_id, name FROM product_groups")
            .fetch_all(tx.as_mut())
            .await
            .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(inserted_groups.len(), 2);

        assert_eq!(inserted_groups[0].whmcs_id, Some(1));
        assert_eq!(inserted_groups[0].name, "Group1");

        assert_eq!(inserted_groups[1].whmcs_id, Some(2));
        assert_eq!(inserted_groups[1].name, "Group2");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_products_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let group_map = helpers::populate_product_groups(&mut tx).await;
        let products = vec![
            types::Product::new(100, 1, "Product1"),
            types::Product::new(101, 999, "Product2"),
        ];

        // Act
        let affected_rows = insert_products(&mut tx, products, &group_map)
            .await
            .unwrap();

        // Assert
        let all_products = sqlx::query!("SELECT name, group_id FROM products")
            .fetch_all(tx.as_mut())
            .await
            .unwrap();

        assert_eq!(affected_rows, 1);
        assert_eq!(all_products.len(), 1);

        assert_eq!(all_products[0].name, "Product1");
        assert_eq!(all_products[0].group_id, group_map[&1]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_custom_fields_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let group_map = helpers::populate_product_groups(&mut tx).await;
        let product_map = helpers::populate_products(&mut tx, &group_map).await;
        let fields = vec![
            types::CustomField::new(1, "Field1", 1),
            types::CustomField::new(2, "Field2", 999),
        ];

        // Act
        let affected_rows = insert_custom_fields(&mut tx, fields, &product_map)
            .await
            .unwrap();

        // Assert
        let all_fields = sqlx::query!("SELECT name, product_id FROM custom_fields")
            .fetch_all(tx.as_mut())
            .await
            .unwrap();

        assert_eq!(affected_rows, 1);
        assert_eq!(all_fields.len(), 1);

        assert_eq!(all_fields[0].name, "Field1");
        assert_eq!(all_fields[0].product_id, product_map[&1]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_config_options_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let options = vec![
            types::ConfigOption::new(1, "CPU"),
            types::ConfigOption::new(2, "RAM"),
        ];

        // Act
        let affected_rows = insert_config_options(&mut tx, options).await.unwrap();

        // Assert
        let inserted_options =
            sqlx::query!("SELECT whmcs_id as id, name FROM config_options ORDER BY whmcs_id")
                .fetch_all(tx.as_mut())
                .await
                .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(inserted_options.len(), 2);

        assert_eq!(inserted_options[0].id, Some(1));
        assert_eq!(inserted_options[0].name, "CPU");

        assert_eq!(inserted_options[1].id, Some(2));
        assert_eq!(inserted_options[1].name, "RAM");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_servers_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let vm_records = vec![
            types::VmRecord::new(1, 101, Some("pve1"), "server1.test.com", "Active"),
            types::VmRecord::new(2, 102, None, "server2.test.com", "Suspended"),
        ];

        // Act
        let affected_rows = insert_servers(&mut tx, vm_records).await.unwrap();

        // Assert
        let servers = sqlx::query!(
            "SELECT whmcs_id, vm_id, node_name, host_name, status FROM servers ORDER BY whmcs_id"
        )
        .fetch_all(tx.as_mut())
        .await
        .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(servers.len(), 2);

        assert_eq!(servers[0].whmcs_id, Some(1));
        assert_eq!(servers[0].vm_id, Some(101));
        assert_eq!(servers[0].node_name, Some("pve1".to_owned()));
        assert_eq!(servers[0].host_name, "server1.test.com");
        assert_eq!(servers[0].status, "active");

        assert_eq!(servers[1].whmcs_id, Some(2));
        assert_eq!(servers[1].vm_id, Some(102));
        assert_eq!(servers[1].node_name, Some("pve".to_owned()));
        assert_eq!(servers[1].host_name, "server2.test.com");
        assert_eq!(servers[1].status, "suspended");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_networks_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let networks = vec![
            types::Network::new(1, "Pool1", "192.168.1.1", "255.255.255.0"),
            types::Network::new(2, "Pool2", "10.0.0.1", "255.0.0.0"),
        ];

        // Act
        let affected_rows = insert_networks(&mut tx, networks).await.unwrap();

        // Assert
        let inserted_networks = sqlx::query!(
            "SELECT whmcs_id, datacenter_name, gateway, subnet_mask FROM networks ORDER BY whmcs_id"
        )
        .fetch_all(tx.as_mut())
        .await
        .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(inserted_networks.len(), 2);

        assert_eq!(inserted_networks[0].whmcs_id, Some(1));
        assert_eq!(inserted_networks[0].datacenter_name, "Pool1");
        assert_eq!(inserted_networks[0].gateway, "192.168.1.1");
        assert_eq!(inserted_networks[0].subnet_mask, "255.255.255.0");

        assert_eq!(inserted_networks[1].whmcs_id, Some(2));
        assert_eq!(inserted_networks[1].datacenter_name, "Pool2");
        assert_eq!(inserted_networks[1].gateway, "10.0.0.1");
        assert_eq!(inserted_networks[1].subnet_mask, "255.0.0.0");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_ip_addresses_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let network_map = helpers::populate_networks(&mut tx).await;
        let server_map = helpers::populate_servers(&mut tx).await;
        let addresses = vec![
            types::IpAddress::new(1, 2, "10.10.0.10", Some(1)),
            types::IpAddress::new(2, 2, "10.10.0.11", None),
            types::IpAddress::new(3, 999, "10.10.0.12", None),
        ];

        // Act
        let affected_rows = insert_ip_addresses(&mut tx, addresses, &server_map, &network_map)
            .await
            .unwrap();

        // Assert
        let inserted_ips = sqlx::query!(
            "SELECT ip_address, network_id, server_id FROM ip_addresses ORDER BY ip_address"
        )
        .fetch_all(tx.as_mut())
        .await
        .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(inserted_ips.len(), 2);

        assert_eq!(inserted_ips[0].ip_address, "10.10.0.10");
        assert_eq!(inserted_ips[0].network_id, network_map[&2]);
        assert_eq!(inserted_ips[0].server_id, Some(server_map[&1]));

        assert_eq!(inserted_ips[1].ip_address, "10.10.0.11");
        assert_eq!(inserted_ips[1].network_id, network_map[&2]);
        assert_eq!(inserted_ips[1].server_id, None);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_templates_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let template_fields = vec![
            types::TemplateField::new(1, "Ubuntu 22.04|9000,CentOS 9|9002"),
            types::TemplateField::new(2, "Debian 11|9001"),
            types::TemplateField::new(3, ""),
        ];

        // Act
        let affected_rows = insert_templates(&mut tx, template_fields).await.unwrap();

        // Assert
        let templates = sqlx::query!(
            "SELECT os_name, template_vmid, template_node, virtual_type FROM templates ORDER BY template_vmid"
        )
            .fetch_all(tx.as_mut())
            .await
            .unwrap();

        assert_eq!(affected_rows, 3);
        assert_eq!(templates.len(), 3);

        assert_eq!(templates[0].os_name, "Ubuntu 22.04");
        assert_eq!(templates[0].template_vmid, 9000);
        assert_eq!(templates[0].template_node, "pve");
        assert_eq!(templates[0].virtual_type, "qemu");

        assert_eq!(templates[1].os_name, "Debian 11");
        assert_eq!(templates[1].template_vmid, 9001);

        assert_eq!(templates[2].os_name, "CentOS 9");
        assert_eq!(templates[2].template_vmid, 9002);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_services_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user_map = helpers::populate_users(&mut tx).await;
        let server_map = helpers::populate_servers(&mut tx).await;
        let groups_map = helpers::populate_product_groups(&mut tx).await;
        let product_map = helpers::populate_products(&mut tx, &groups_map).await;
        let template_map = helpers::populate_templates(&mut tx, &product_map).await;
        let services = vec![
            types::Service::new(1, "Active", 1, 1),
            types::Service::new(2, "Active", 2, 2),
        ];

        // Act
        let affected_rows = insert_services(
            &mut tx,
            services,
            &user_map,
            &server_map,
            &product_map,
            &template_map,
        )
        .await
        .unwrap();

        // Assert
        let inserted_services = sqlx::query!(
            "SELECT user_id, server_id, product_id, template_id FROM services ORDER BY whmcs_id"
        )
        .fetch_all(tx.as_mut())
        .await
        .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(inserted_services.len(), 2);

        assert_eq!(inserted_services[0].user_id, user_map[&1]);
        assert_eq!(inserted_services[0].server_id, server_map[&1]);
        assert_eq!(inserted_services[0].product_id, product_map[&1]);
        assert_eq!(inserted_services[0].template_id, template_map[&1]);

        assert_eq!(inserted_services[1].user_id, user_map[&2]);
        assert_eq!(inserted_services[1].server_id, server_map[&2]);
        assert_eq!(inserted_services[1].product_id, product_map[&2]);
        assert_eq!(inserted_services[1].template_id, template_map[&2]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_custom_values_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user_map = helpers::populate_users(&mut tx).await;
        let server_map = helpers::populate_servers(&mut tx).await;
        let group_map = helpers::populate_product_groups(&mut tx).await;
        let product_map = helpers::populate_products(&mut tx, &group_map).await;
        let template_map = helpers::populate_templates(&mut tx, &product_map).await;
        let service_map = helpers::populate_services(
            &mut tx,
            &user_map,
            &server_map,
            &product_map,
            &template_map,
        )
        .await;
        let custom_map = helpers::populate_custom_fields(&mut tx, &product_map).await;
        let values = vec![
            types::CustomValue::new(100, 10, 1, "Value1"),
            types::CustomValue::new(101, 11, 2, "Value2"),
            types::CustomValue::new(102, 10, 999, "Value3"),
        ];

        // Act
        let affected_rows = insert_custom_values(&mut tx, values, &service_map, &custom_map)
            .await
            .unwrap();

        // Assert
        let inserted_values = sqlx::query!(
            "SELECT service_id, custom_field_id, value FROM custom_values ORDER BY value"
        )
        .fetch_all(tx.as_mut())
        .await
        .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(inserted_values.len(), 2);

        assert_eq!(inserted_values[0].value, "Value1");
        assert_eq!(inserted_values[0].service_id, service_map[&1]);
        assert_eq!(inserted_values[0].custom_field_id, custom_map[&10]);

        assert_eq!(inserted_values[1].value, "Value2");
        assert_eq!(inserted_values[1].service_id, service_map[&2]);
        assert_eq!(inserted_values[1].custom_field_id, custom_map[&11]);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn insert_config_values_works(pool: PgPool) {
        // Arrange
        let mut tx = pool.begin().await.unwrap();
        let user_map = helpers::populate_users(&mut tx).await;
        let server_map = helpers::populate_servers(&mut tx).await;
        let group_map = helpers::populate_product_groups(&mut tx).await;
        let product_map = helpers::populate_products(&mut tx, &group_map).await;
        let template_map = helpers::populate_templates(&mut tx, &product_map).await;
        let service_map = helpers::populate_services(
            &mut tx,
            &user_map,
            &server_map,
            &product_map,
            &template_map,
        )
        .await;
        let config_map = helpers::populate_config_options(&mut tx).await;
        let values = vec![
            types::ConfigValue::new(1, 1, 1, "4 GB"),
            types::ConfigValue::new(2, 2, 2, "2 Cores"),
            types::ConfigValue::new(3, 999, 1, "1 GB"),
        ];

        // Act
        let affected_rows = insert_config_values(&mut tx, values, &service_map, &config_map)
            .await
            .unwrap();

        // Assert
        let inserted_values =
            sqlx::query!("SELECT service_id, config_id, value FROM config_values")
                .fetch_all(tx.as_mut())
                .await
                .unwrap();

        assert_eq!(affected_rows, 2);
        assert_eq!(inserted_values.len(), 2);

        let ram_val = inserted_values
            .iter()
            .find(|v| v.config_id == config_map[&1])
            .unwrap();
        assert_eq!(ram_val.value, "4");
        assert_eq!(ram_val.service_id, service_map[&1]);

        let cpu_val = inserted_values
            .iter()
            .find(|v| v.config_id == config_map[&2])
            .unwrap();
        assert_eq!(cpu_val.value, "2");
        assert_eq!(cpu_val.service_id, service_map[&2]);
    }

    mod helpers {
        use super::*;
        use crate::etl::types;
        use crate::etl::types::ProductGroup;
        use std::collections::HashMap;
        use uuid::Uuid;

        pub async fn populate_users(tx: &mut PgTransaction<'_>) -> HashMap<i32, Uuid> {
            let clients = vec![
                types::Client {
                    id: 1,
                    firstname: "John".to_owned(),
                    lastname: "Doe".to_owned(),
                    email: "john.doe@example.com".to_owned(),
                    address1: "123 Main St".to_owned(),
                    city: "Anytown".to_owned(),
                    state: "CA".to_owned(),
                    postcode: "12345".to_owned(),
                    country: "US".to_owned(),
                    phonenumber: "555-1234".to_owned(),
                    password: "password123".to_owned(),
                },
                types::Client {
                    id: 2,
                    firstname: "Jane".to_owned(),
                    lastname: "Doe".to_owned(),
                    email: "jane.doe@example.com".to_owned(),
                    address1: "123 Main St".to_owned(),
                    city: "Anytown".to_owned(),
                    state: "CA".to_owned(),
                    postcode: "12345".to_owned(),
                    country: "US".to_owned(),
                    phonenumber: "555-1235".to_owned(),
                    password: "password124".to_owned(),
                },
            ];

            insert_users(tx, clients).await.unwrap();

            sqlx::query!("SELECT whmcs_id, id FROM users")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }

        pub async fn populate_product_groups(tx: &mut PgTransaction<'_>) -> HashMap<i32, Uuid> {
            let groups = vec![
                ProductGroup::new(1, "Group1"),
                ProductGroup::new(2, "Group2"),
            ];
            insert_product_groups(tx, groups).await.unwrap();

            sqlx::query!("SELECT id, whmcs_id FROM product_groups")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }

        pub async fn populate_products(
            tx: &mut PgTransaction<'_>,
            group_map: &HashMap<i32, Uuid>,
        ) -> HashMap<i32, Uuid> {
            let products = vec![
                types::Product::new(1, 1, "Product1"),
                types::Product::new(2, 2, "Product2"),
            ];
            insert_products(tx, products, &group_map).await.unwrap();

            sqlx::query!("SELECT id, whmcs_id FROM products")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }

        pub async fn populate_custom_fields(
            tx: &mut PgTransaction<'_>,
            product_map: &HashMap<i32, Uuid>,
        ) -> HashMap<i32, Uuid> {
            let fields = vec![
                types::CustomField::new(10, "CustomField1", 1),
                types::CustomField::new(11, "CustomField2", 2),
            ];
            insert_custom_fields(tx, fields, product_map).await.unwrap();

            sqlx::query!("SELECT id, whmcs_id FROM custom_fields")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }

        pub async fn populate_config_options(tx: &mut PgTransaction<'_>) -> HashMap<i32, Uuid> {
            let options = vec![
                types::ConfigOption::new(1, "RAM"),
                types::ConfigOption::new(2, "CPU"),
            ];
            insert_config_options(tx, options).await.unwrap();

            sqlx::query!("SELECT id, whmcs_id FROM config_options")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }

        pub async fn populate_servers(tx: &mut PgTransaction<'_>) -> HashMap<i32, Uuid> {
            let vm_records = vec![
                types::VmRecord::new(1, 101, Some("pve1"), "server1.test.com", "Active"),
                types::VmRecord::new(2, 102, None, "server2.test.com", "Active"),
            ];
            insert_servers(tx, vm_records).await.unwrap();

            sqlx::query!("SELECT id, whmcs_id FROM servers")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }

        pub async fn populate_networks(tx: &mut PgTransaction<'_>) -> HashMap<i32, Uuid> {
            let networks = vec![
                types::Network::new(1, "Pool1", "192.168.1.1", "255.255.255.0"),
                types::Network::new(2, "Pool2", "10.0.0.1", "255.0.0.0"),
            ];
            insert_networks(tx, networks).await.unwrap();

            sqlx::query!("SELECT id, whmcs_id FROM networks")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }

        pub async fn populate_templates(
            tx: &mut PgTransaction<'_>,
            product_map: &HashMap<i32, Uuid>,
        ) -> HashMap<i32, Uuid> {
            let template_fields = vec![types::TemplateField::new(
                1,
                "Ubuntu 22.04|9000,CentOS 9|9002, Debian 11|9001",
            )];
            insert_templates(tx, template_fields).await.unwrap();

            sqlx::query!("SELECT id FROM templates ORDER BY template_vmid")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .zip(product_map.iter().map(|prod| *prod.0))
                .map(|(rec, prod_id)| (prod_id, rec.id))
                .collect()
        }

        pub async fn populate_services(
            tx: &mut PgTransaction<'_>,
            user_map: &HashMap<i32, Uuid>,
            server_map: &HashMap<i32, Uuid>,
            product_map: &HashMap<i32, Uuid>,
            template_map: &HashMap<i32, Uuid>,
        ) -> HashMap<i32, Uuid> {
            let services = vec![
                types::Service::new(1, "Active", 1, 1),
                types::Service::new(2, "Active", 2, 2),
            ];
            insert_services(
                tx,
                services,
                user_map,
                server_map,
                product_map,
                template_map,
            )
            .await
            .unwrap();

            sqlx::query!("SELECT id, whmcs_id FROM services")
                .fetch_all(tx.as_mut())
                .await
                .unwrap()
                .into_iter()
                .map(|rec| (rec.whmcs_id.unwrap(), rec.id))
                .collect()
        }
    }
}
