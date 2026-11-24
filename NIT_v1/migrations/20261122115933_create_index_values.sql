CREATE TABLE index_values (
    id SERIAL PRIMARY KEY,
    index_id INTEGER NOT NULL REFERENCES indices(id),
    source_id INTEGER NOT NULL REFERENCES data_sources(id),
    value DECIMAL NOT NULL,
    observed_on DATE NOT NULL,
    fetched_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
