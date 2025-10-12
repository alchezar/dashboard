-- Create servers table
CREATE TABLE servers
(
    id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vm_id     INTEGER,
    node_name TEXT,
    host_name TEXT NOT NULL,
    status    TEXT NOT NULL
);

-- Create services table
CREATE TABLE services
(
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status     TEXT NOT NULL,
    user_id    UUID NOT NULL REFERENCES users (id),
    server_id  UUID NOT NULL REFERENCES servers (id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products (id)
);

-- Create IP addresses table
CREATE TABLE ip_addresses
(
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ip_address TEXT                                NOT NULL,
    network_id UUID                                NOT NULL REFERENCES network (id),
    server_id  UUID REFERENCES servers (id) UNIQUE NULL
)