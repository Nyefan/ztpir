-- Add migration script here
-- Create Subscriptions Table
CREATE TABLE subscriptions
(
    id            UUID PRIMARY KEY     DEFAULT uuidv7(),
    email         TEXT        NOT NULL UNIQUE,
    name          TEXT        NOT NULL,
    subscribed_at timestamptz NOT NULL DEFAULT NOW()
)