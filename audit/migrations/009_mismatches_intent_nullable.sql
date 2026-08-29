-- Fulfillment-timeout mismatches have no buyer intent, so intent_id must be nullable.
ALTER TABLE mismatches ALTER COLUMN intent_id DROP NOT NULL;
