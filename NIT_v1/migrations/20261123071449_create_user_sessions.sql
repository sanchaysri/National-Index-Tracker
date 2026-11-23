CREATE TABLE user_sessions (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    access_token_hash BYTEA NOT NULL UNIQUE,
    refresh_token_hash BYTEA NOT NULL UNIQUE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    access_expires_at TIMESTAMPTZ NOT NULL,
    refresh_expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,

    CHECK (octet_length(access_token_hash) = 32),
    CHECK (octet_length(refresh_token_hash) = 32),
    CHECK (access_expires_at > created_at),
    CHECK (access_expires_at <= refresh_expires_at)
);

CREATE UNIQUE INDEX user_sessions_one_unrevoked_per_user
    ON user_sessions (user_id)
    WHERE revoked_at IS NULL;
