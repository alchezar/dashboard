-- Create network table
CREATE TABLE networks
(
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    datacenter_name TEXT NOT NULL,
    gateway         TEXT NOT NULL,
    subnet_mask     TEXT NOT NULL,
    whmcs_id        INT UNIQUE
);

-- Create product groups table
CREATE table product_groups
(
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name     TEXT NOT NULL,
    whmcs_id INT UNIQUE
);

-- Create configurable options (CPU, RAM) table
create table config_options
(
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name     TEXT NOT NULL,
    whmcs_id INT UNIQUE
);