use crate::cli::Cli;
use crate::model::whmcs;
use crate::prelude::Result;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::{MySqlPool, PgPool};
use std::collections::HashSet;

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
        self.migrate_users().await?;
        self.migrate_product_groups().await?;
        self.migrate_config_options().await?;
        self.migrate_products().await?;
        self.migrate_custom_fields().await?;
        self.migrate_servers().await?;
        self.migrate_services().await?;

        tracing::info!(dry_run = %self.dry_run, "Migration completed.");
        Ok(())
    }

    /// Migrates active users from the WHMCS database to the `users` table.
    ///
    async fn migrate_users(&self) -> Result<()> {
        // All WHMCS clients.
        let query = include_str!("sql/get_active_clients.sql");
        let mut whmcs_clients = sqlx::query_as::<_, whmcs::Client>(query)
            .fetch_all(&self.source_pool)
            .await?;

        // All existing Dashboard users.
        let existing_whmcs_ids = sqlx::query!(
            r#"
SELECT whmcs_id FROM users
            "#
        )
        .fetch_all(&self.target_pool)
        .await?
        .into_iter()
        .filter_map(|rec| rec.whmcs_id)
        .collect::<HashSet<_>>();

        // Leave only new WHMCS clients.
        whmcs_clients.retain(|client| !existing_whmcs_ids.contains(&client.id));
        tracing::trace!(new_clients = %whmcs_clients.len(), "Fetching new active WHMCS clients completed.");

        // Show new clients in log on dry run.
        if self.dry_run {
            for client in whmcs_clients {
                tracing::info!(target: "dry run", ?client, "New active WHMCS client.");
            }
        } else {
            insert_users(&self.target_pool, whmcs_clients).await?;
        }

        tracing::debug!("Users migration completed.");
        Ok(())
    }

    async fn migrate_product_groups(&self) -> Result<()> {
        tracing::debug!("Product groups migration completed.");
        Ok(())
    }

    async fn migrate_config_options(&self) -> Result<()> {
        tracing::debug!("Configurable options migration completed.");
        Ok(())
    }

    async fn migrate_products(&self) -> Result<()> {
        tracing::debug!("Products migration completed.");
        Ok(())
    }

    async fn migrate_custom_fields(&self) -> Result<()> {
        tracing::debug!("Custom fields migration completed.");
        Ok(())
    }

    async fn migrate_servers(&self) -> Result<()> {
        tracing::debug!("Servers migration completed.");
        Ok(())
    }

    async fn migrate_services(&self) -> Result<()> {
        tracing::debug!("Services migration completed.");
        Ok(())
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
        &first_names, &last_names, &emails, &addresses, &cities, &states, &post_codes, &countries, &phone_numbers, &passwords, &whmcs_ids )
        .execute(pool)
        .await?;

    Ok(())
}
