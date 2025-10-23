-- Create products table
CREATE TABLE products
(
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES product_groups (id),
    name     TEXT NOT NULL,
    whmcs_id INT UNIQUE
);

-- Create custom templates table
CREATE TABLE templates
(
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    os_name       TEXT    NOT NULL UNIQUE,
    template_vmid INTEGER NOT NULL UNIQUE,
    template_node TEXT    NOT NULL,
    virtual_type  TEXT    NOT NULL
);

-- Create custom fields table
CREATE TABLE custom_fields
(
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products (id),
    name       TEXT NOT NULL,
    whmcs_id   INT UNIQUE
);
