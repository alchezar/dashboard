-- Create users table
CREATE TABLE users
(
    id           SERIAL PRIMARY KEY,
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
    updated_at   TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create index on email for faster lookups
CREATE INDEX idx_users_email ON users (email);

-- Create index on phone number
CREATE INDEX idx_users_phone_number ON users (phone_number);