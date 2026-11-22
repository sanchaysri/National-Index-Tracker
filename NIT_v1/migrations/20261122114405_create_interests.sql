-- Add migration script here

CREATE TABLE IF NOT EXISTS interests (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL UNIQUE,
    description TEXT
);

