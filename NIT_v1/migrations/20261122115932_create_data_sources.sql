CREATE TABLE data_sources (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    url TEXT NOT NULL,
    api_endpoint TEXT
);
