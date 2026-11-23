ALTER TABLE index_values RENAME COLUMN value TO payload;

ALTER TABLE index_values
    ALTER COLUMN payload TYPE JSONB USING to_jsonb(payload);
