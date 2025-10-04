-- Create network table
CREATE TABLE network
(
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gateway     TEXT NOT NULL,
    subnet_mask TEXT NOT NULL
);

-- Create product groups table
CREATE table product_groups
(
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL
);

-- Create configurable options (CPU, RAM) table
create table config_options
(
    id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL
);