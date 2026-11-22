-- Add migration script here

CREATE TABLE IF NOT EXISTS indices (
    id SERIAL PRIMARY KEY,
    interest_id INTEGER NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    unit VARCHAR,
    CONSTRAINT fk_indices_interest
        FOREIGN KEY (interest_id)
        REFERENCES interests(id)
        ON DELETE CASCADE
);
