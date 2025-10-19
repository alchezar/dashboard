-- Add whmcs_id to users table (corresponds to tblclients.id)
ALTER TABLE users
    ADD COLUMN whmcs_id INT UNIQUE;

-- Add whmcs_id to services table (corresponds to tblhosting.id)
ALTER TABLE services
    ADD COLUMN whmcs_id INT UNIQUE;

-- Add whmcs_id to products table (corresponds to tblproducts.id)
ALTER TABLE products
    ADD COLUMN whmcs_id INT UNIQUE;

-- Add whmcs_id to product_groups table (corresponds to tblproductgroups.id)
ALTER TABLE product_groups
    ADD COLUMN whmcs_id INT UNIQUE;

-- Add whmcs_id to config_options table (corresponds to tblproductconfigoptions.id)
ALTER TABLE config_options
    ADD COLUMN whmcs_id INT UNIQUE;

-- Add whmcs_id to custom_fields table (corresponds to tblcustomfields.id)
ALTER TABLE custom_fields
    ADD COLUMN whmcs_id INT UNIQUE;