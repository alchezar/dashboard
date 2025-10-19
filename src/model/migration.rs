use crate::prelude::Result;
use sqlx::{MySqlPool, PgPool};

#[allow(unused)]
pub async fn migrate(source_pool: MySqlPool, target_pool: PgPool, dry_run: bool) -> Result<()> {
    Ok(())
}
