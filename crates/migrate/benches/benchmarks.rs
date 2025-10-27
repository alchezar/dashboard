use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use migration_utility::cli::Cli;
use migration_utility::etl::loaders::*;
use migration_utility::etl::migration::Migration;
use migration_utility::etl::types::*;
use std::collections::HashMap;
use tokio::runtime::Runtime;

const BATCH_SIZE: i32 = 100;

/// Defines and runs all migration benchmarks.
///
/// # Arguments
///
/// * `criterion`: Benchmark manager.
///
fn migration_benchmarks(criterion: &mut Criterion) {
    // Setup asynchronous runtime and migration object.
    dotenv::dotenv().ok();
    let runtime = Runtime::new().unwrap();
    let migration = runtime.block_on(async {
        Migration::new(&Cli {
            dry_run: true,
            chunk_size: 1024,
            source_url: std::env::var("SOURCE_URL").unwrap(),
            target_url: std::env::var("TARGET_URL").unwrap(),
        })
        .await
        .unwrap()
    });

    // Group for the slow, full migration test.
    let mut migration_group = criterion.benchmark_group("Full Migration");
    migration_group.sample_size(10);
    migration_group.bench_function("Full migration run", |b| {
        full_migration_bench(b, &runtime, &migration)
    });
    migration_group.finish();

    // Group for fast, partial loader tests.
    let mut loaders_group = criterion.benchmark_group("Partial Loaders");
    loaders_group.sample_size(100);
    loaders_group
        .bench_function(format!("Insert {} Clients", BATCH_SIZE), |b| {
            insert_clients_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Product Groups", BATCH_SIZE), |b| {
            insert_product_groups_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Products", BATCH_SIZE), |b| {
            insert_products_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Custom Fields", BATCH_SIZE), |b| {
            insert_custom_fields_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Config Options", BATCH_SIZE), |b| {
            insert_config_options_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Servers", BATCH_SIZE), |b| {
            insert_servers_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Networks", BATCH_SIZE), |b| {
            insert_networks_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Ip Addresses", BATCH_SIZE), |b| {
            insert_ip_addresses_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Templates", BATCH_SIZE), |b| {
            insert_templates_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Services", BATCH_SIZE), |b| {
            insert_services_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Custom field values", BATCH_SIZE), |b| {
            insert_custom_values_bench(b, &runtime, &migration)
        })
        .bench_function(format!("Insert {} Config option values", BATCH_SIZE), |b| {
            insert_config_values_bench(b, &runtime, &migration)
        });
    loaders_group.finish();
}

criterion_group!(benches, migration_benchmarks);
criterion_main!(benches);

fn full_migration_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    bencher.to_async(runtime).iter(|| {
        let mut value = migration.clone();
        async move {
            value.run().await.ok();
        }
    })
}

fn insert_clients_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let users = (0..BATCH_SIZE)
        .map(|index| Client {
            id: index,
            firstname: "John".to_owned(),
            lastname: "Doe".to_owned(),
            email: format!("john_doe{}@example.com", index),
            address1: "123 Main St".to_owned(),
            city: "Anytown".to_owned(),
            state: "CA".to_owned(),
            postcode: "12345".to_owned(),
            country: "US".to_owned(),
            phonenumber: "555-1234".to_owned(),
            password: "password123".to_owned(),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_users(&mut tx, users.clone()).await.unwrap();
        tx.rollback().await.unwrap();
    });
}

fn insert_product_groups_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let product_groups = (0..BATCH_SIZE)
        .map(|index| ProductGroup {
            id: index,
            name: format!("ProductGroup{}", index),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_product_groups(&mut tx, product_groups.clone())
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    })
}

fn insert_products_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let products = (0..BATCH_SIZE)
        .map(|index| Product {
            id: index,
            gid: index % 10,
            name: format!("Product{}", index),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_products(&mut tx, products.clone(), &HashMap::new())
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    });
}

fn insert_custom_fields_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let custom_fields = (0..BATCH_SIZE)
        .map(|index| CustomField {
            id: index,
            fieldname: format!("ConfigField{}", index),
            relid: index % 20,
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_custom_fields(&mut tx, custom_fields.clone(), &HashMap::new())
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    });
}
fn insert_config_options_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let config_options = (0..BATCH_SIZE)
        .map(|index| ConfigOption {
            id: index,
            optionname: format!("ConfigValue{}", index),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_config_options(&mut tx, config_options.clone())
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    });
}
fn insert_servers_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let servers = (0..BATCH_SIZE)
        .map(|index| VmRecord {
            id: index as u32,
            vmid: (1000 + index) as u32,
            node: None,
            hostname: format!("host_{}.example.com", index),
            status: "Active".to_owned(),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_servers(&mut tx, servers.clone()).await.unwrap();
        tx.rollback().await.unwrap();
    });
}
fn insert_networks_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let networks = (0..BATCH_SIZE)
        .map(|index| Network {
            id: index,
            title: format!("Network{}", index),
            gateway: "10.0.0.1".to_owned(),
            mask: "255.255.255.0".to_owned(),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_networks(&mut tx, networks.clone()).await.unwrap();
        tx.rollback().await.unwrap();
    });
}
fn insert_ip_addresses_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let ip_addresses = (0..BATCH_SIZE)
        .map(|index| IpAddress {
            id: index,
            pool_id: index % 10,
            ipaddress: format!("10.0.0.{}", index),
            server_id: None,
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        let dummy = &HashMap::new();
        insert_ip_addresses(&mut tx, ip_addresses.clone(), &dummy, &dummy)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    });
}
fn insert_templates_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let templates = (0..BATCH_SIZE)
        .map(|index| TemplateField {
            relid: index % 20,
            fieldoptions: format!("Template{}", index),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        insert_templates(&mut tx, templates.clone()).await.unwrap();
        tx.rollback().await.unwrap();
    });
}

fn insert_services_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let services = (0..BATCH_SIZE)
        .map(|index| Service {
            id: index,
            domainstatus: "Active".to_owned(),
            userid: index,
            packageid: index % 50,
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        let dummy = &HashMap::new();
        insert_services(&mut tx, services.clone(), &dummy, &dummy, &dummy, &dummy)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    });
}

fn insert_custom_values_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let custom_values = (0..BATCH_SIZE)
        .map(|index| CustomValue {
            id: index as u32,
            fieldid: index % 10,
            relid: index % 20,
            value: format!("CustomValue{}", index),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        let dummy = &HashMap::new();
        insert_custom_values(&mut tx, custom_values.clone(), &dummy, &dummy)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    });
}

fn insert_config_values_bench(bencher: &mut Bencher, runtime: &Runtime, migration: &Migration) {
    let config_values = (0..BATCH_SIZE)
        .map(|index| ConfigValue {
            id: index,
            relid: index % 20,
            configid: index % 10,
            optionname: format!("ConfigValue{}", index),
        })
        .collect::<Vec<_>>();

    bencher.to_async(runtime).iter(|| async {
        let mut tx = migration.target_pool.begin().await.unwrap();
        let dummy = &HashMap::new();
        insert_config_values(&mut tx, config_values.clone(), &dummy, &dummy)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    });
}
