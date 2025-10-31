-- Enable pgcrypto extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create users table
CREATE TABLE users
(
    id           UUID PRIMARY KEY         DEFAULT gen_random_uuid(),
    first_name   TEXT NOT NULL,
    last_name    TEXT NOT NULL,
    email        TEXT NOT NULL UNIQUE,
    address      TEXT NOT NULL,
    city         TEXT NOT NULL,
    state        TEXT NOT NULL,
    post_code    TEXT NOT NULL,
    country      TEXT NOT NULL,
    phone_number TEXT NOT NULL,
    password     TEXT NOT NULL,
    created_at   TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    whmcs_id     INT UNIQUE
);

-- Create index on phone number
CREATE INDEX idx_users_phone_number ON users (phone_number);