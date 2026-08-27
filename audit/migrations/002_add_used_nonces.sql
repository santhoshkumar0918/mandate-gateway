-- Per-purchase nonce replay protection.
--
-- Every money-moving purchase must be authorized by a freshly signed
-- PurchaseAuth carrying its own nonce. That nonce is recorded here and
-- consumed atomically with the budget debit, so a replayed purchase
-- authorization can never trigger a second payment — even under
-- concurrency, the UNIQUE constraint on nonce guarantees at most one
-- successful consumption.

CREATE TABLE used_nonces (
    nonce       TEXT PRIMARY KEY,
    mandate_id  UUID NOT NULL REFERENCES mandates(mandate_id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_used_nonces_mandate ON used_nonces (mandate_id);
