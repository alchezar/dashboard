use migration_utility::cli::Cli;
use migration_utility::etl::migration::Migration;

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

fn main() {
    // Init profiler with specified path to the `target` folder.
    let path = std::path::PathBuf::from("../../target/dhat-heap.json");
    let profiler = dhat::Profiler::builder().file_name(path).build();

    dotenv::dotenv().ok();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut migration = runtime.block_on(async {
        Migration::new(&Cli {
            dry_run: true,
            chunk_size: 1024,
            source_url: std::env::var("SOURCE_URL").unwrap().into(),
            target_url: std::env::var("TARGET_URL").unwrap().into(),
        })
        .await
        .unwrap()
    });

    println!("dhat: Memory benchmark started.");

    runtime.block_on(async {
        migration.run().await.unwrap();
    });

    // Explicitly destroy the profiler to ensure that we get a report.
    drop(profiler);
}
