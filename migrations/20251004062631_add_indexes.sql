-- Indexes for foreign key columns to speed up joins and lookups

-- products table
CREATE INDEX idx_products_group_id ON products (group_id);

-- custom_fields table
CREATE INDEX idx_custom_fields_product_id ON custom_fields (product_id);

-- servers table
CREATE INDEX idx_servers_network_id ON servers (network_id);

-- services table
CREATE INDEX idx_services_user_id ON services (user_id);
CREATE INDEX idx_services_server_id ON services (server_id);
CREATE INDEX idx_services_product_id ON services (product_id);

-- custom_values table
CREATE INDEX idx_custom_values_service_id ON custom_values (service_id);
CREATE INDEX idx_custom_values_custom_field_id ON custom_values (custom_field_id);

-- config_values table
CREATE INDEX idx_config_values_service_id ON config_values (service_id);
CREATE INDEX idx_config_values_config_id ON config_values (config_id);