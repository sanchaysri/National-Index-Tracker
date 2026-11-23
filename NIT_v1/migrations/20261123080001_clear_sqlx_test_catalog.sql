DO $$
BEGIN
    IF current_database() LIKE '%_sqlx_test%' THEN
        DELETE FROM index_values;
        DELETE FROM user_interests;
        DELETE FROM indices;
        DELETE FROM interests;
        DELETE FROM data_sources;

        ALTER SEQUENCE index_values_id_seq RESTART WITH 1;
        ALTER SEQUENCE user_interests_id_seq RESTART WITH 1;
        ALTER SEQUENCE indices_id_seq RESTART WITH 1;
        ALTER SEQUENCE interests_id_seq RESTART WITH 1;
        ALTER SEQUENCE data_sources_id_seq RESTART WITH 1;
    END IF;
END $$;
