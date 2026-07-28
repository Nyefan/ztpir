-- Add migration script here
CREATE TABLE subscriptions_confirmation_tokens
(
  id               UUID PRIMARY KEY     DEFAULT uuidv7(),
  subscriptions_id UUID REFERENCES subscriptions (id),
  token            TEXT        NOT NULL,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at       TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '7 days',
  exhausted        BOOLEAN     NOT NULL DEFAULT FALSE,
  exhausted_at     TIMESTAMPTZ NULL
)
