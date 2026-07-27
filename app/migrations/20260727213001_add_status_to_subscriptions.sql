-- Add migration script here
CREATE TYPE subscription_status AS ENUM ('pending_confirmation', 'confirmed', 'unsubscribed_from_all');
ALTER TABLE subscriptions ADD COLUMN status subscription_status NULL DEFAULT 'pending_confirmation';
