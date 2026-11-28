CREATE TABLE IF NOT EXISTS users (
    pid           TEXT PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'user',
    plan          TEXT NOT NULL DEFAULT 'free',
    created_at    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

CREATE TABLE IF NOT EXISTS usages (
    user_pid TEXT NOT NULL,
    period   TEXT NOT NULL,
    sent     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_pid, period)
);
