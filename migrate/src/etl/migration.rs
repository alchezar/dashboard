//! This module is responsible for the "Extract" phase of the migration
//! pipeline with overall orchestration of the entire ETL migration process.
//!
//! The central `Migration` struct holds the application's state, including the
//! connection pools for both the source and target databases. The `run` method
//! ensures that all migration steps are executed in the correct order within a
//! single atomic transaction.
//!
//! The generic `migrate_table` function implements the "Extract" logic,
//! streaming data in chunks from the source MySQL database before passing it to
//! the appropriate loader function from the [`loaders`] module.

use crate::cli::Cli;
use crate::etl::loaders;
use crate::etl::types::{self, DashboardTable};
use dashboard_common::error::Result;
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
            |tx, chunk, _| Box::pin(loaders::insert_users(tx, chunk)),
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
            |tx, chunk, _| Box::pin(loaders::insert_product_groups(tx, chunk)),
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
            |tx, chunk, map| Box::pin(loaders::insert_products(tx, chunk, map)),
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
            |tx, chunk, prod_map| Box::pin(loaders::insert_custom_fields(tx, chunk, prod_map)),
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
            |tx, chunk, _| Box::pin(loaders::insert_config_options(tx, chunk)),
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
            |tx, chunk, _| Box::pin(loaders::insert_servers(tx, chunk)),
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
            |tx, chunk, _| Box::pin(loaders::insert_networks(tx, chunk)),
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
                Box::pin(loaders::insert_ip_addresses(tx, chunk, serv_map, net_map))
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
            |tx, chunk, _| Box::pin(loaders::insert_templates(tx, chunk)),
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
                Box::pin(loaders::insert_services(
                    tx, chunk, user_map, serv_map, prod_map, temp_map,
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
                Box::pin(loaders::insert_custom_values(tx, chunk, serv_map, cust_map))
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
                Box::pin(loaders::insert_config_values(tx, chunk, serv_map, conf_map))
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
