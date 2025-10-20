use crate::cli::Cli;
use crate::model::whmcs;
use crate::prelude::Result;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::{MySqlPool, PgPool, Row};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Holds the context and shared state for the entire migration process.
///
pub struct Migration {
    source_pool: MySqlPool,
    target_pool: PgPool,
    dry_run: bool,
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
        tracing::info!("Database pools created.");

        Ok(Self {
            source_pool,
            target_pool,
            dry_run: cli.dry_run,
        })
    }

    /// Runs the complete, ordered migration process.
    ///
    pub async fn run(&self) -> Result<()> {
        // Users
        self.migrate_users().await?;
        // Products
        self.migrate_product_groups().await?;
        self.migrate_products().await?;
        self.migrate_custom_fields().await?;
        self.migrate_config_options().await?;
        // Servers
        self.migrate_servers().await?;
        self.migrate_network().await?;
        self.migrate_ip_addresses().await?;
        self.migrate_templates().await?;
        // Services
        self.migrate_services().await?;
        self.migrate_custom_values().await?;
        self.migrate_config_values().await?;

        tracing::info!(dry_run = %self.dry_run, "Migration completed.");
        Ok(())
    }

    /// Migrates active users from the WHMCS `tblclients` to the `users` table.
    ///
    async fn migrate_users(&self) -> Result<()> {
        // All WHMCS clients.
        let query = include_str!("sql/get_active_clients.sql");
        let mut whmcs_clients = sqlx::query_as::<_, whmcs::Client>(query)
            .fetch_all(&self.source_pool)
            .await?;

        // All existing Dashboard users.
        let existing_user_ids = self.get_existing_whmcs_ids("users").await?;

        // Leave only new WHMCS clients.
        whmcs_clients.retain(|client| !existing_user_ids.contains(&client.id));
        tracing::trace!(new_clients = %whmcs_clients.len(), "Fetching new active WHMCS clients completed.");

        // Show new clients in log on dry run or insert into database.
        match self.dry_run {
            true => self.log_dry_run_items(&whmcs_clients, "client"),
            false => insert_users(&self.target_pool, whmcs_clients).await?,
        }

        tracing::debug!("Users migration completed.");
        Ok(())
    }

    /// Migrates product groups from the WHMCS `tblproductgroups` to the
    /// `product_groups` table.
    ///
    async fn migrate_product_groups(&self) -> Result<()> {
        // All WHMCS product groups.
        let query = include_str!("sql/get_product_groups.sql");
        let mut whmcs_groups = sqlx::query_as::<_, whmcs::ProductGroup>(query)
            .fetch_all(&self.source_pool)
            .await?;

        // All existing Dashboard product groups.
        let existing_group_ids = self.get_existing_whmcs_ids("product_groups").await?;

        // Leave only new WHMCS product groups.
        whmcs_groups.retain(|group| !existing_group_ids.contains(&group.id));
        tracing::trace!(new_product_groups = %whmcs_groups.len(), "Fetching new WHMCS product groups completed.");

        // Show new product groups in log on dry run or insert into database.
        match self.dry_run {
            true => self.log_dry_run_items(&whmcs_groups, "product group"),
            false => insert_product_groups(&self.target_pool, whmcs_groups).await?,
        }

        tracing::debug!("Product groups migration completed.");
        Ok(())
    }

    /// Migrates product from the WHMCS `tblproducts` to the
    /// `products` table.
    ///
    async fn migrate_products(&self) -> Result<()> {
        // All WHMCS products.
        let query = include_str!("sql/get_products.sql");
        let mut whmcs_products = sqlx::query_as::<_, whmcs::Product>(query)
            .fetch_all(&self.source_pool)
            .await?;

        // All existing Dashboard products and groups ID map.
        let existing_product_ids = self.get_existing_whmcs_ids("products").await?;
        let group_id_map = self.get_existing_all_ids("product_groups").await?;

        // Leave only new WHMCS products.
        whmcs_products.retain(|product| !existing_product_ids.contains(&product.id));
        tracing::trace!(new_products = %whmcs_products.len(), "Fetching new WHMCS products completed.");

        // Show new products in log on dry run or insert into database.
        match self.dry_run {
            true => self.log_dry_run_items(&whmcs_products, "product"),
            false => insert_products(&self.target_pool, whmcs_products, group_id_map).await?,
        }

        tracing::debug!("Products migration completed.");
        Ok(())
    }

    /// Migrates custom fields from the WHMCS `tblcustomfields` to the
    /// `config_options` table.
    ///
    async fn migrate_custom_fields(&self) -> Result<()> {
        // All WHMCS custom fields for products.
        let query = include_str!("sql/get_custom_fields.sql");
        let mut whmcs_fields = sqlx::query_as::<_, whmcs::CustomField>(query)
            .fetch_all(&self.source_pool)
            .await?;

        // All existing Dashboard custom fields and products ID map.
        let existing_field_ids = self.get_existing_whmcs_ids("custom_fields").await?;
        let product_id_map = self.get_existing_all_ids("products").await?;

        // Leave only new WHMCS custom fields.
        whmcs_fields.retain(|field| !existing_field_ids.contains(&field.id));
        tracing::trace!(new_custom_fields = %whmcs_fields.len(), "Fetching new WHMCS custom fields completed.");

        // Show new custom fields in log on dry run or insert into database.
        match self.dry_run {
            true => self.log_dry_run_items(&whmcs_fields, "custom_fields"),
            false => insert_custom_fields(&self.target_pool, whmcs_fields, &product_id_map).await?,
        }

        tracing::debug!("Custom fields migration completed.");
        Ok(())
    }

    /// Migrates config options from the WHMCS `tblproductconfigoptions` to the
    /// `config_options` table.
    ///
    async fn migrate_config_options(&self) -> Result<()> {
        // All WHMCS config options.
        let query = include_str!("sql/get_config_options.sql");
        let mut whmcs_options = sqlx::query_as::<_, whmcs::ConfigOption>(query)
            .fetch_all(&self.source_pool)
            .await?;

        // All existing Dashboard config options.
        let existing_option_ids = self.get_existing_whmcs_ids("config_options").await?;

        // Leave only new WHMCS config options.
        whmcs_options.retain(|option| !existing_option_ids.contains(&option.id));
        tracing::trace!(new_config_options = %whmcs_options.len(), "Fetching new WHMCS config options completed.");

        // Show new config options in log on dry run or insert into database.
        match self.dry_run {
            true => self.log_dry_run_items(&whmcs_options, "config option"),
            false => insert_config_options(&self.target_pool, whmcs_options).await?,
        }

        tracing::debug!("Configurable options migration completed.");
        Ok(())
    }

    async fn migrate_servers(&self) -> Result<()> {
        tracing::debug!("Servers migration completed.");
        Ok(())
    }

    async fn migrate_network(&self) -> Result<()> {
        tracing::debug!("Network migration completed.");
        Ok(())
    }
    async fn migrate_ip_addresses(&self) -> Result<()> {
        tracing::debug!("IP addresses migration completed.");
        Ok(())
    }
    async fn migrate_templates(&self) -> Result<()> {
        tracing::debug!("Templates migration completed.");
        Ok(())
    }

    async fn migrate_services(&self) -> Result<()> {
        tracing::debug!("Services migration completed.");
        Ok(())
    }

    async fn migrate_custom_values(&self) -> Result<()> {
        tracing::debug!("Custom field values migration completed.");
        Ok(())
    }

    async fn migrate_config_values(&self) -> Result<()> {
        tracing::debug!("Configurable option values migration completed.");
        Ok(())
    }

    // -------------------------------------------------------------------------

    /// Returns all existing WHMCS ids from the specific Dashboard table.
    ///
    /// # Arguments
    ///
    /// * `table_name`: Table to select WHMCS ids from.
    ///
    /// # Returns
    ///
    /// `HashSet` of WHMCS ids for quicker search.
    ///
    async fn get_existing_whmcs_ids(&self, table_name: &str) -> Result<HashSet<i32>> {
        Ok(sqlx::query(&format!("SELECT whmcs_id FROM {}", table_name))
            .fetch_all(&self.target_pool)
            .await?
            .into_iter()
            .filter_map(|row| row.try_get("whmcs_id").ok())
            .collect())
    }

    /// Returns all existing ids (WHMCS and Dashboard) from the specific
    /// Dashboard table.
    ///
    /// # Arguments
    ///
    /// * `table_name`: Table to select WHMCS ids from.
    ///
    /// # Returns
    ///
    /// `HashMap` of relationship between the WHMCS id and the Dashboard id.
    ///
    async fn get_existing_all_ids(&self, table_name: &str) -> Result<HashMap<i32, Uuid>> {
        let query = format!(
            "SELECT whmcs_id, id FROM {} WHERE whmcs_id IS NOT NULL",
            table_name
        );
        Ok(sqlx::query(&query)
            .fetch_all(&self.target_pool)
            .await?
            .into_iter()
            .filter_map(|row| {
                let key = row.try_get("whmcs_id").ok();
                let value = row.try_get("id").ok();
                key.zip(value)
            })
            .collect::<HashMap<i32, Uuid>>())
    }

    /// Logs the items that would be inserted if this were not a dry run.
    ///
    /// # Types
    ///
    /// * `T`: Type of items being processed.
    ///
    /// # Arguments
    ///
    /// * `items`: Slice of data structures to be logged.
    /// * `item_name`: Name of the item being processed.
    ///
    fn log_dry_run_items<T>(&self, items: &[T], item_name: &str)
    where
        T: std::fmt::Debug,
    {
        for row in items {
            tracing::info!(target: "dry run", ?row, "New WHMCS {}.", item_name);
        }
    }
}

/// Helper function to bulk insert users into the target database.
///
/// # Arguments
///
/// * `pool`: Target PostgreSQL connection pool.
/// * `clients`: Vector of `whmcs::Client` structs to be inserted.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
async fn insert_users(pool: &PgPool, clients: Vec<whmcs::Client>) -> Result<()> {
    // Collect all fields.
    let count = clients.len();
    let (
        whmcs_ids,
        first_names,
        last_names,
        emails,
        addresses,
        cities,
        states,
        post_codes,
        countries,
        phone_numbers,
        passwords,
    ) = clients.into_iter().fold(
        (
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
        ),
        |mut acc, client| {
            acc.0.push(client.id);
            acc.1.push(client.firstname);
            acc.2.push(client.lastname);
            acc.3.push(client.email);
            acc.4.push(client.address1);
            acc.5.push(client.city);
            acc.6.push(client.state);
            acc.7.push(client.postcode);
            acc.8.push(client.country);
            acc.9.push(client.phonenumber);
            acc.10.push(client.password);
            acc
        },
    );
    tracing::trace!("Collecting all fields completed.");

    // Insert collected users.
    sqlx::query!(
            r#"
INSERT INTO users ( first_name, last_name, email, address, city, state, post_code, country, phone_number, password, whmcs_id )
SELECT * FROM UNNEST( $1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::text[], $9::text[], $10::text[], $11::int4[] )
            "#,
        &first_names, &last_names, &emails, &addresses, &cities, &states, &post_codes, &countries, &phone_numbers, &passwords, &whmcs_ids ).execute(pool).await?;

    Ok(())
}

/// Helper function to bulk insert product groups into the target database.
///
/// # Arguments
///
/// * `pool`: Target PostgreSQL connection pool.
/// * `groups`: Vector of `whmcs::ProductGroup` structs to be inserted.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
async fn insert_product_groups(pool: &PgPool, groups: Vec<whmcs::ProductGroup>) -> Result<()> {
    let count = groups.len();
    let (whmcs_ids, whmcs_names) = groups.into_iter().fold(
        (Vec::with_capacity(count), Vec::with_capacity(count)),
        |mut acc, group| {
            acc.0.push(group.id);
            acc.1.push(group.name);
            acc
        },
    );
    tracing::trace!("Collecting all fields completed.");

    // Insert collected users.
    sqlx::query!(
        r#"
INSERT INTO product_groups (name, whmcs_id)
SELECT * FROM UNNEST($1::text[], $2::int4[])
        "#,
        &whmcs_names,
        &whmcs_ids
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper function to bulk insert products into the target database.
///
/// # Arguments
///
/// * `pool`: Target PostgreSQL connection pool.
/// * `products`: Vector of `whmcs::Product` structs to be inserted.
/// * `group_id_map`: Relationship between the WHMCS id and the Dashboard id for
///   product groups.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
async fn insert_products(
    pool: &PgPool,
    products: Vec<whmcs::Product>,
    group_id_map: HashMap<i32, Uuid>,
) -> Result<()> {
    let count = products.len();
    #[rustfmt::skip]
    let fields_iter = products
        .into_iter()
        .filter_map(|product| match group_id_map.get(&product.gid) {
            Some(group_uuid) => Some((group_uuid.clone(), product.name, product.id)),
            _ => {
                tracing::warn!(product_id = ?product.id, group_id = ?product.gid,
                    "Skipping product with non-migrated product group." );
                None
            }
        });

    let (group_ids, product_names, whmcs_ids) = fields_iter.fold(
        (
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
        ),
        |mut acc, (group_uuid, name, id)| {
            acc.0.push(group_uuid);
            acc.1.push(name);
            acc.2.push(id);
            acc
        },
    );
    tracing::trace!("Collecting all fields completed.");

    // Insert collected products.
    sqlx::query!(
        r#"
INSERT INTO products (group_id, name, whmcs_id)
SELECT * FROM UNNEST($1::uuid[], $2::text[], $3::int4[])
        "#,
        &group_ids,
        &product_names,
        &whmcs_ids
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper function to bulk insert custom fields into the target database.
///
/// # Arguments
///
/// * `pool`: Target PostgreSQL connection pool.
/// * `fields`: Vector of `whmcs::CustomField` structs to be inserted.
/// * `product_id_map`:  Relationship between the WHMCS id and the Dashboard id
///   for products.
///
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
async fn insert_custom_fields(
    pool: &PgPool,
    fields: Vec<whmcs::CustomField>,
    product_id_map: &HashMap<i32, Uuid>,
) -> Result<()> {
    let count = fields.len();
    #[rustfmt::skip]
    let fields_iter = fields
        .into_iter()
        .filter_map(|field| match product_id_map.get(&field.relid) {
            Some(product_uuid) => Some((product_uuid.clone(), field.fieldname, field.id)),
            _ => {
                tracing::warn!(custom_field_id = ?field.id, product_id = ?field.relid,
                    "Skipping custom field with non-migrated product." );
                None
            }
        });

    let (product_ids, field_names, whmcs_ids) = fields_iter.fold(
        (
            Vec::with_capacity(count),
            Vec::with_capacity(count),
            Vec::with_capacity(count),
        ),
        |mut acc, (product_uuid, name, id)| {
            acc.0.push(product_uuid);
            acc.1.push(name);
            acc.2.push(id);
            acc
        },
    );
    tracing::trace!("Collecting all fields completed.");

    // Insert collected custom fields.
    sqlx::query!(
        r#"
INSERT INTO custom_fields (product_id, name, whmcs_id)
SELECT * FROM UNNEST($1::uuid[], $2::text[], $3::int4[])
        "#,
        &product_ids,
        &field_names,
        &whmcs_ids
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper function to bulk insert config options into the target database.
///
/// # Arguments
///
/// * `pool`: Target PostgreSQL connection pool.
/// * `options`: Vector of `whmcs::ConfigOption` structs to be inserted.
///
/// # Returns
///
/// Empty `Ok(())` on success.
///
async fn insert_config_options(pool: &PgPool, options: Vec<whmcs::ConfigOption>) -> Result<()> {
    let count = options.len();
    let (whmcs_ids, whmcs_names) = options.into_iter().fold(
        (Vec::with_capacity(count), Vec::with_capacity(count)),
        |mut acc, option| {
            acc.0.push(option.id);
            acc.1.push(option.optionname);
            acc
        },
    );
    tracing::trace!("Collecting all fields completed.");

    // Insert collected config options.
    sqlx::query!(
        r#"
INSERT INTO config_options (name, whmcs_id)
SELECT * FROM UNNEST($1::text[], $2::int4[])
        "#,
        &whmcs_names,
        &whmcs_ids
    )
    .execute(pool)
    .await?;

    Ok(())
}
