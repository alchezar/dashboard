use crate::cli::Cli;
use crate::model::whmcs;
use crate::model::whmcs::DashboardTable;
use crate::prelude::Result;
use futures::StreamExt;
use futures::stream::TryStreamExt;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::{MySqlPool, PgPool, PgTransaction, Row};
use std::collections::HashMap;
use uuid::Uuid;

const CHUNK_SIZE: usize = 1024;

/// Holds the context and shared state for the entire migration process.
///
pub struct Migration {
    source_pool: MySqlPool,
    target_pool: PgPool,
    dry_run: bool,
    statistic: HashMap<DashboardTable, u64>,
}

impl Migration {
    /// Creates new `Migration` instance.
    ///
    /// # Arguments
    ///
    /// * `cli`: Parsed command-line arguments.
    ///
    pub async fn new(cli: &Cli) -> Result<Self> {
        let source_pool = MySqlPoolOptions::new().connect(&cli.source_url).await?;
        let target_pool = PgPoolOptions::new().connect(&cli.target_url).await?;
        tracing::info!(?cli.source_url, ?cli.target_url, "Database pools created.");

        Ok(Self {
            source_pool,
            target_pool,
            dry_run: cli.dry_run,
            statistic: HashMap::new(),
        })
    }

    /// Runs the complete, ordered migration process.
    ///
    pub async fn run(&mut self) -> Result<()> {
        let mut transaction = self.target_pool.begin().await?;
        // Users
        self.migrate_users(&mut transaction).await?;
        // Products
        self.migrate_product_groups(&mut transaction).await?;
        self.migrate_products(&mut transaction).await?;
        self.migrate_custom_fields(&mut transaction).await?;
        self.migrate_config_options(&mut transaction).await?;
        // Servers
        self.migrate_servers(&mut transaction).await?;
        self.migrate_networks(&mut transaction).await?;
        self.migrate_ip_addresses(&mut transaction).await?;
        self.migrate_templates().await?;
        // Services
        self.migrate_services().await?;
        self.migrate_custom_values().await?;
        self.migrate_config_values().await?;

        match self.dry_run {
            true => transaction.rollback().await?,
            false => transaction.commit().await?,
        }
        tracing::info!(dry_run = %self.dry_run, statistic = ?self.statistic, "Migration completed.");
        Ok(())
    }

    /// Migrates active users from the WHMCS `tblclients` to the `users` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_users(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_active_clients.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::Client>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS clients.");
            total_affected += insert_users(tx, chunk).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::Users);
        tracing::debug!("Users migration completed.");
        Ok(())
    }

    /// Migrates product groups from the WHMCS `tblproductgroups` to the
    /// `product_groups` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_product_groups(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_product_groups.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::ProductGroup>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS product groups.");
            total_affected += insert_product_groups(tx, chunk).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::ProductGroups);
        tracing::debug!("Product groups migration completed.");
        Ok(())
    }

    /// Migrates product from the WHMCS `tblproducts` to the
    /// `products` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_products(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_products.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::Product>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS products.");
            let groups_map = self
                .get_existing_ids(tx, DashboardTable::ProductGroups)
                .await?;
            total_affected += insert_products(tx, chunk, groups_map).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::Products);
        tracing::debug!("Products migration completed.");
        Ok(())
    }

    /// Migrates custom fields from the WHMCS `tblcustomfields` to the
    /// `config_options` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_custom_fields(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_custom_fields.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::CustomField>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS custom fields.");
            let products_map = self.get_existing_ids(tx, DashboardTable::Products).await?;
            total_affected += insert_custom_fields(tx, chunk, &products_map).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::CustomFields);
        tracing::debug!("Custom fields migration completed.");
        Ok(())
    }

    /// Migrates config options from the WHMCS `tblproductconfigoptions` to the
    /// `config_options` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_config_options(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_config_options.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::ConfigOption>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS configurable options.");
            total_affected += insert_config_options(tx, chunk).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::ConfigOptions);
        tracing::debug!("Configurable options migration completed.");
        Ok(())
    }

    /// Migrates servers from the WHMCS `mod_pvewhmcs_wms` to the `servers`
    /// table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_servers(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_servers.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::VmRecord>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS servers.");
            total_affected += insert_servers(tx, chunk).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::Servers);
        tracing::debug!("Servers migration completed.");
        Ok(())
    }

    /// Migrates networks from the WHMCS `mod_pvewhmcs_ip_pools` to the
    /// `networks` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_networks(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_networks.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::Network>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS network.");
            total_affected += insert_networks(tx, chunk).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::Networks);
        tracing::debug!("Networks migration completed.");
        Ok(())
    }

    /// Migrates ip addresses from the WHMCS `mod_pvewhmcs_ip_addresses` to the
    /// `networks` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_ip_addresses(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let query = include_str!("sql/get_ip_addresses.sql");
        let mut chunks = sqlx::query_as::<_, whmcs::IpAddress>(query)
            .fetch(&self.source_pool)
            .try_chunks(CHUNK_SIZE);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), "Fetching a chunk of WHMCS networks.");
            let servers_map = self.get_existing_ids(tx, DashboardTable::Servers).await?;
            let networks_map = self.get_existing_ids(tx, DashboardTable::Networks).await?;
            total_affected += insert_ip_addresses(tx, chunk, &servers_map, &networks_map).await?;
        }
        drop(chunks);

        self.collect_statistics(total_affected, DashboardTable::IpAddresses);
        tracing::debug!("IP addresses migration completed.");
        Ok(())
    }

    async fn migrate_templates(&mut self) -> Result<()> {
        tracing::debug!("Templates migration completed.");
        Ok(())
    }

    async fn migrate_services(&mut self) -> Result<()> {
        tracing::debug!("Services migration completed.");
        Ok(())
    }

    async fn migrate_custom_values(&mut self) -> Result<()> {
        tracing::debug!("Custom field values migration completed.");
        Ok(())
    }

    async fn migrate_config_values(&mut self) -> Result<()> {
        tracing::debug!("Configurable option values migration completed.");
        Ok(())
    }

    // -------------------------------------------------------------------------

    /// Returns all existing ids (WHMCS and Dashboard) from the specific
    /// Dashboard table.
    ///
    /// # Arguments
    ///
    /// * `table`: Table to select WHMCS ids from.
    ///
    /// # Returns
    ///
    /// `HashMap` of relationship between the WHMCS id and the Dashboard id.
    ///
    async fn get_existing_ids(
        &self,
        tx: &mut PgTransaction<'_>,
        table: DashboardTable,
    ) -> Result<HashMap<i32, Uuid>> {
        let mut builder = sqlx::QueryBuilder::<sqlx::Postgres>::default();
        let query = builder
            .push("SELECT whmcs_id, id FROM ")
            .push(table)
            .push(" WHERE whmcs_id IS NOT NULL")
            .build();
        Ok(query
            .fetch_all(tx.as_mut())
            .await?
            .into_iter()
            .filter_map(|row| {
                let key = row.try_get("whmcs_id").ok();
                let value = row.try_get("id").ok();
                key.zip(value)
            })
            .collect::<HashMap<i32, Uuid>>())
    }

    /// Accumulates migration statistics for a given table.
    ///
    /// # Arguments
    ///
    /// * `affected`: Number of affected rows.
    /// * `table`: Target table the statistics belongs to.
    ///
    fn collect_statistics(&mut self, affected: u64, table: DashboardTable) {
        if affected > 0 {
            *self.statistic.entry(table).or_default() += affected;
        }
    }
}

// -----------------------------------------------------------------------------

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
#[macro_export]
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

// -----------------------------------------------------------------------------

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
async fn insert_users(tx: &mut PgTransaction<'_>, clients: Vec<whmcs::Client>) -> Result<u64> {
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
async fn insert_product_groups(
    tx: &mut PgTransaction<'_>,
    groups: Vec<whmcs::ProductGroup>,
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
async fn insert_products(
    tx: &mut PgTransaction<'_>,
    products: Vec<whmcs::Product>,
    group_id_map: HashMap<i32, Uuid>,
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
async fn insert_custom_fields(
    tx: &mut PgTransaction<'_>,
    fields: Vec<whmcs::CustomField>,
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
async fn insert_config_options(
    tx: &mut PgTransaction<'_>,
    options: Vec<whmcs::ConfigOption>,
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
async fn insert_servers(
    tx: &mut PgTransaction<'_>,
    vm_records: Vec<whmcs::VmRecord>,
) -> Result<u64> {
    let servers = vm_records
        .into_iter()
        .map(|vm| vm.into())
        .collect::<Vec<whmcs::Server>>();

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
async fn insert_networks(tx: &mut PgTransaction<'_>, networks: Vec<whmcs::Network>) -> Result<u64> {
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
///
/// # Returns
///
/// On success, the number of affected rows.
///
async fn insert_ip_addresses(
    tx: &mut PgTransaction<'_>,
    address: Vec<whmcs::IpAddress>,
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
                .and_then(|unsigned_id| Some(unsigned_id as i32))
                .and_then(|id| server_map.get(&id))
                .and_then(|uuid| Some(*uuid));
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
