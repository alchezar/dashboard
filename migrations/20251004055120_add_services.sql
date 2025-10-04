-- Create servers table
CREATE TABLE servers
(
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    network_id UUID NOT NULL REFERENCES network (id),
    vm_id      INTEGER,
    node_name  TEXT,
    ip_address TEXT NOT NULL
);

-- Create services table
CREATE TABLE services
(
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status     TEXT NOT NULL,
    user_id    UUID NOT NULL REFERENCES users (id),
    server_id  UUID NOT NULL REFERENCES servers (id),
    product_id UUID NOT NULL REFERENCES products (id)
);