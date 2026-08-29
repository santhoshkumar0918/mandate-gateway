-- Verify-then-pay: fulfillment proof for orders.
-- An order is "settled" only after a merchant posts delivery proof; absent
-- proof within the SLA, the reconciliation sweep recovers the spend.
CREATE TABLE IF NOT EXISTS fulfillments (
    order_id     TEXT PRIMARY KEY,
    mandate_id   UUID NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending',
    proof        TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    fulfilled_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_fulfillments_status ON fulfillments (status);
