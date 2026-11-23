ALTER TABLE indices
    ADD CONSTRAINT indices_interest_id_name_key UNIQUE (interest_id, name);
