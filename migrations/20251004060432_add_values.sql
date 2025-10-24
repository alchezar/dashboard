-- Create custom fields values table
CREATE TABLE custom_values
(
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id      UUID NOT NULL REFERENCES services (id) ON DELETE CASCADE,
    custom_field_id UUID NOT NULL REFERENCES custom_fields (id),
    value           TEXT NOT NULL,
    whmcs_id        INT UNIQUE
);

-- Create configuration option values table
CREATE TABLE config_values
(
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id UUID NOT NULL REFERENCES services (id) ON DELETE CASCADE,
    config_id  UUID NOT NULL REFERENCES config_options (id),
    value      TEXT NOT NULL,
    whmcs_id   INT UNIQUE
);