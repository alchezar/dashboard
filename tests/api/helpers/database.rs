use sqlx::PgPool;
use uuid::Uuid;

pub async fn populate_product(pool: &PgPool) -> Uuid {
    // New test product group.
    let group_id = sqlx::query!(
        r#"
INSERT INTO product_groups (name)
VALUES ('TestGroup1')
RETURNING id
			"#
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .id;

    // New test product.
    let product_id = sqlx::query!(
        r#"
INSERT INTO products (group_id, name, virtual_type, template_id, template_node)
VALUES ($1, 'Test Product', 'qemu', 100, 'pve')
RETURNING id
			"#,
        group_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .id;

    // New network.
    let network_id = sqlx::query!(
        r#"
INSERT INTO network (datacenter_name, gateway, subnet_mask)
VALUES ('Amsterdam', '192.168.0.1', '255.255.255.255')
RETURNING id
			"#,
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .id;

    // New IP address.
    sqlx::query!(
        r#"
insert into ip_addresses (ip_address, network_id)
values ('192.168.0.100', $1)
            "#,
        network_id
    )
    .execute(pool)
    .await
    .unwrap();

    // New configurable options.
    sqlx::query!(
        r#"
INSERT INTO config_options (name)
VALUES ('cpu_cores'), ('ram_gb')
            "#,
    )
    .execute(pool)
    .await
    .unwrap();

    // New custom fields.
    sqlx::query!(
        r#"
INSERT INTO custom_fields (product_id, name)
VALUES ($1, 'os'), ($1, 'datacenter')
            "#,
        product_id
    )
    .execute(pool)
    .await
    .unwrap();

    product_id
}
