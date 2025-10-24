use crate::cli::Cli;
use crate::types;
use crate::types::DashboardTable;
use common::error::Result;
use futures::StreamExt;
use futures::stream::TryStreamExt;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::{MySqlPool, PgPool, PgTransaction, Row};
use std::collections::HashMap;
use std::pin::Pin;
use uuid::Uuid;

/// Holds the context and shared state for the entire migration process.
///
pub struct Migration {
    source_pool: MySqlPool,
    target_pool: PgPool,
    dry_run: bool,
    chunk_size: usize,
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
            chunk_size: cli.chunk_size,
            statistic: HashMap::new(),
        })
    }

    /// Runs the complete, ordered migration process.
    ///
    pub async fn run(&mut self) -> Result<()> {
        let mut transaction = self.target_pool.begin().await?;
        self.migrate_users(&mut transaction).await?;
        self.migrate_product_groups(&mut transaction).await?;
        self.migrate_products(&mut transaction).await?;
        self.migrate_custom_fields(&mut transaction).await?;
        self.migrate_config_options(&mut transaction).await?;
        self.migrate_servers(&mut transaction).await?;
        self.migrate_networks(&mut transaction).await?;
        self.migrate_ip_addresses(&mut transaction).await?;
        self.migrate_templates(&mut transaction).await?;
        self.migrate_services(&mut transaction).await?;
        self.migrate_custom_values(&mut transaction).await?;
        self.migrate_config_values(&mut transaction).await?;

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
        self.migrate_table(
            include_str!("sql/get_active_clients.sql"),
            DashboardTable::Users,
            tx,
            (),
            |tx, chunk, _| Box::pin(insert_users(tx, chunk)),
        )
        .await
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
        self.migrate_table(
            include_str!("sql/get_product_groups.sql"),
            DashboardTable::ProductGroups,
            tx,
            (),
            |tx, chunk, _| Box::pin(insert_product_groups(tx, chunk)),
        )
        .await
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
        let groups_map = self
            .get_existing_ids(tx, DashboardTable::ProductGroups, "whmcs_id")
            .await?;

        self.migrate_table(
            include_str!("sql/get_products.sql"),
            DashboardTable::Products,
            tx,
            groups_map,
            |tx, chunk, map| Box::pin(insert_products(tx, chunk, &map)),
        )
        .await
    }

    /// Migrates custom fields from the WHMCS `tblcustomfields` to the
    /// `custom_fields` table.
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
        let products_map = self
            .get_existing_ids(tx, DashboardTable::Products, "whmcs_id")
            .await?;

        self.migrate_table(
            include_str!("sql/get_custom_fields.sql"),
            DashboardTable::CustomFields,
            tx,
            products_map,
            |tx, chunk, prod_map| Box::pin(insert_custom_fields(tx, chunk, &prod_map)),
        )
        .await
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
        self.migrate_table(
            include_str!("sql/get_config_options.sql"),
            DashboardTable::ConfigOptions,
            tx,
            (),
            |tx, chunk, _| Box::pin(insert_config_options(tx, chunk)),
        )
        .await
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
        self.migrate_table(
            include_str!("sql/get_servers.sql"),
            DashboardTable::Servers,
            tx,
            (),
            |tx, chunk, _| Box::pin(insert_servers(tx, chunk)),
        )
        .await
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
        self.migrate_table(
            include_str!("sql/get_networks.sql"),
            DashboardTable::Networks,
            tx,
            (),
            |tx, chunk, _| Box::pin(insert_networks(tx, chunk)),
        )
        .await
    }

    /// Migrates ip addresses from the WHMCS `mod_pvewhmcs_ip_addresses` to the
    /// `ip_addresses` table.
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
        let servers_map = self
            .get_existing_ids(tx, DashboardTable::Servers, "whmcs_id")
            .await?;
        let networks_map = self
            .get_existing_ids(tx, DashboardTable::Networks, "whmcs_id")
            .await?;

        self.migrate_table(
            include_str!("sql/get_ip_addresses.sql"),
            DashboardTable::IpAddresses,
            tx,
            (servers_map, networks_map),
            |tx, chunk, (serv_map, net_map)| {
                Box::pin(insert_ip_addresses(tx, chunk, &serv_map, &net_map))
            },
        )
        .await
    }

    /// Migrates templates from the WHMCS `tblcustomfields` to the `templates`
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
    async fn migrate_templates(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        self.migrate_table(
            include_str!("sql/get_templates.sql"),
            DashboardTable::Templates,
            tx,
            (),
            |tx, chunk, _| Box::pin(insert_templates(tx, chunk)),
        )
        .await
    }

    /// Migrates services from the WHMCS `tblhosting` to the `services` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_services(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let user_map = self
            .get_existing_ids(tx, DashboardTable::Users, "whmcs_id")
            .await?;
        let serv_map = self
            .get_existing_ids(tx, DashboardTable::Servers, "whmcs_id")
            .await?;
        let prod_map = self
            .get_existing_ids(tx, DashboardTable::Products, "whmcs_id")
            .await?;
        let temp_map = self.get_template_ids(tx).await?;

        self.migrate_table(
            include_str!("sql/get_services.sql"),
            DashboardTable::Services,
            tx,
            (user_map, serv_map, prod_map, temp_map),
            |tx, chunk, (user_map, serv_map, prod_map, temp_map)| {
                Box::pin(insert_services(
                    tx, chunk, &user_map, &serv_map, &prod_map, &temp_map,
                ))
            },
        )
        .await
    }

    /// Migrates custom fields values from the WHMCS `tblcustomfieldsvalues` to
    /// the `custom_values` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_custom_values(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let service_map = self
            .get_existing_ids(tx, DashboardTable::Services, "whmcs_id")
            .await?;
        let custom_map = self
            .get_existing_ids(tx, DashboardTable::CustomFields, "whmcs_id")
            .await?;

        self.migrate_table(
            include_str!("sql/get_custom_values.sql"),
            DashboardTable::CustomValues,
            tx,
            (service_map, custom_map),
            |tx, chunk, (serv_map, cust_map)| {
                Box::pin(insert_custom_values(tx, chunk, &serv_map, &cust_map))
            },
        )
        .await
    }

    /// Migrates configurable option values from the WHMCS
    /// `tblhostingconfigoptions` to the `config_values` table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_config_values(&mut self, tx: &mut PgTransaction<'_>) -> Result<()> {
        let service_map = self
            .get_existing_ids(tx, DashboardTable::Services, "whmcs_id")
            .await?;
        let config_map = self
            .get_existing_ids(tx, DashboardTable::ConfigOptions, "whmcs_id")
            .await?;

        self.migrate_table(
            include_str!("sql/get_config_values.sql"),
            DashboardTable::ConfigValues,
            tx,
            (service_map, config_map),
            |tx, chunk, (serv_map, conf_map)| {
                Box::pin(insert_config_values(tx, chunk, &serv_map, &conf_map))
            },
        )
        .await
    }

    // -------------------------------------------------------------------------

    /// Returns all existing ids (WHMCS and Dashboard) from the specific
    /// Dashboard table.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    /// * `table`: Table to select WHMCS ids from.
    /// * `key_name`: Name of the WHMCS id field.
    ///
    /// # Returns
    ///
    /// `HashMap` of relationship between the WHMCS id and the Dashboard id.
    ///
    async fn get_existing_ids(
        &self,
        tx: &mut PgTransaction<'_>,
        table: DashboardTable,
        key_name: &str,
    ) -> Result<HashMap<i32, Uuid>> {
        let mut builder = sqlx::QueryBuilder::<sqlx::Postgres>::default();
        let query = builder
            .push("SELECT ")
            .push(key_name)
            .push(", id FROM ")
            .push(table)
            .push(" WHERE ")
            .push(key_name)
            .push(" IS NOT NULL")
            .build();
        Ok(query
            .fetch_all(tx.as_mut())
            .await?
            .into_iter()
            .filter_map(|row| {
                let key = row.try_get(key_name).ok();
                let value = row.try_get("id").ok();
                key.zip(value)
            })
            .collect::<HashMap<i32, Uuid>>())
    }

    /// Retrieves a map of WHMCS product IDs to Dashboard template UUIDs.
    ///
    /// # Arguments
    ///
    /// * `tx`: In-progress transaction for target database.
    ///
    /// # Returns
    ///
    /// `HashMap` of relationship between the WHMCS product ID and the Dashboard
    /// template UUID.
    ///
    async fn get_template_ids(&self, tx: &mut PgTransaction<'_>) -> Result<HashMap<i32, Uuid>> {
        // Template field info from source WHMCS.
        let query = include_str!("sql/get_templates.sql");
        let relid_to_vmid = sqlx::query_as::<_, types::TemplateField>(query)
            .fetch_all(&self.source_pool)
            .await?
            .into_iter()
            .map(|temp_field| (temp_field.relid, temp_field.extract()))
            .flat_map(|(relid, vec)| vec.into_iter().map(move |val| (relid, val.template_vmid)))
            .collect::<HashMap<_, _>>();

        // Proxmox template_vmid to Dashboard template UUID relationship.
        let vmid_to_temp_id = self
            .get_existing_ids(tx, DashboardTable::Templates, "template_vmid")
            .await?;

        // Combine to get the final map: WHMCS product_id to Dashboard template_id
        let prod_id_to_temp_id_map = relid_to_vmid
            .into_iter()
            .filter_map(|(whmcs_product_id, template_vmid)| {
                vmid_to_temp_id
                    .get(&template_vmid)
                    .map(|template_id| (whmcs_product_id, *template_id))
            })
            .collect::<HashMap<_, _>>();

        Ok(prod_id_to_temp_id_map)
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

    /// A generic helper function to stream data from the source database,
    /// process it in chunks, and insert it into the target database.
    ///
    /// # Types
    ///
    /// * `C`: Context data structure type, passed to the insertion function.
    /// * `F`: Insertion closure function type.
    /// * `S`: Source data structure type, deserialized from the WHMCS database.
    ///
    /// # Arguments
    ///
    /// * `query`: SQL query string for fetching data from the source database.
    /// * `table`: `DashboardTable` enum variant, for logging and statistics.
    /// * `tx`: In-progress transaction for target database.
    /// * `context`: Context data needed by `insert_fn`.
    /// * `insert_fn`: Closure that handles the transformation and insertion of
    ///   a single chunk of data.
    ///
    /// # Returns
    ///
    /// Empty `Ok(())` on success.
    ///
    async fn migrate_table<C, F, S>(
        &mut self,
        query: &str,
        table: DashboardTable,
        tx: &mut PgTransaction<'_>,
        context: C,
        mut insert_fn: F,
    ) -> Result<()>
    where
        C: Send,
        S: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> + Unpin + Send,
        F: for<'a> FnMut(
            &'a mut PgTransaction<'_>,
            Vec<S>,
            &'a C,
        ) -> Pin<Box<dyn Future<Output = Result<u64>> + Send + 'a>>,
    {
        // Set up a stream to fetch source data in paginated chunks.
        let mut chunks = sqlx::query_as::<_, S>(query)
            .fetch(&self.source_pool)
            .try_chunks(self.chunk_size);
        let mut total_affected = 0;

        while let Some(Ok(chunk)) = chunks.next().await {
            tracing::trace!(size = %chunk.len(), %table, "Insert a chunk of WHMCS data.",);
            total_affected += insert_fn(tx, chunk, &context).await?;
        }

        drop(chunks);
        tracing::debug!("{} migration completed.", table);
        self.collect_statistics(total_affected, table);

        Ok(())
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
async fn insert_users(tx: &mut PgTransaction<'_>, clients: Vec<types::Client>) -> Result<u64> {
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
async fn insert_products(
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
async fn insert_custom_fields(
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
async fn insert_config_options(
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
async fn insert_servers(
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
async fn insert_networks(tx: &mut PgTransaction<'_>, networks: Vec<types::Network>) -> Result<u64> {
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
async fn insert_ip_addresses(
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
async fn insert_templates(
    tx: &mut PgTransaction<'_>,
    temp_fields: Vec<types::TemplateField>,
) -> Result<u64> {
    let templates = temp_fields
        .into_iter()
        .map(types::TemplateField::extract)
        .flatten()
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
async fn insert_services(
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
async fn insert_custom_values(
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
async fn insert_config_values(
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
