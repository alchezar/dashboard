use migration_utility::cli::Cli;
use migration_utility::etl::migration::Migration;
use sqlx::PgPool;

#[sqlx::test]
async fn idempotency_should_be_preserved(pool: PgPool) {
    // Arrange
    let mut migration = setup_migration(pool).await;

    // Act
    let first_stat = migration.run().await.unwrap();
    let second_stat = migration.run().await.unwrap();

    // Assert
    assert!(!first_stat.is_empty());
    assert!(second_stat.is_empty());
}

async fn setup_migration(pool: PgPool) -> Migration {
    // Create new migration object.
    dotenv::dotenv().ok();
    let mut migration = {
        Migration::new(&Cli {
            dry_run: false,
            chunk_size: 1024,
            source_url: std::env::var("SOURCE_URL").unwrap().into(),
            target_url: std::env::var("TARGET_URL").unwrap().into(),
        })
        .await
        .unwrap()
    };

    // Change target pool to the test one.
    migration.target_pool = pool;
    sqlx::migrate!("../../migrations")
        .run(&migration.target_pool)
        .await
        .unwrap();

    migration
}
