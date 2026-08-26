-- Initial schema for the Merchant Agent Gateway audit log.
--
-- All tables are append-only in practice (UPDATE is forbidden on mandates
-- and audit_log by convention — enforced by application code, not DB triggers
-- at this stage).

-- Mandates: signed, scoped permission slips that gate every payment.
CREATE TABLE mandates (
    mandate_id          UUID PRIMARY KEY,
    issued_at           TIMESTAMPTZ NOT NULL,
    expires_at          TIMESTAMPTZ NOT NULL,
    user_id             TEXT NOT NULL,
    merchant_id         TEXT NOT NULL,
    buyer_agent_id      TEXT NOT NULL,
    max_amount          BIGINT NOT NULL,
    currency            TEXT NOT NULL DEFAULT 'INR',
    scope               JSONB NOT NULL DEFAULT '[]',
    frequency           TEXT NOT NULL CHECK (frequency IN ('one_time', 'recurring')),
    spent_amount        BIGINT NOT NULL DEFAULT 0,
    status              TEXT NOT NULL CHECK (status IN ('active', 'revoked', 'expired', 'exhausted')),
    nonce               TEXT NOT NULL UNIQUE,
    signature           BYTEA NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mandates_merchant ON mandates (merchant_id);
CREATE INDEX idx_mandates_status ON mandates (status);
CREATE INDEX idx_mandates_user ON mandates (user_id);

-- Audit log: append-only record of every money-moving action and decision.
CREATE TABLE audit_log (
    id                  BIGSERIAL PRIMARY KEY,
    event_type          TEXT NOT NULL,
    mandate_id          UUID REFERENCES mandates(mandate_id),
    entity_id           TEXT NOT NULL,
    decision            TEXT NOT NULL CHECK (decision IN ('allow', 'block', 'escalate')),
    reason              TEXT,
    detail              JSONB,
    actor               TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_mandate ON audit_log (mandate_id);
CREATE INDEX idx_audit_log_event_type ON audit_log (event_type);
CREATE INDEX idx_audit_log_created ON audit_log (created_at);

-- Orders: tracks Razorpay orders created by the gateway.
CREATE TABLE orders (
    order_id            TEXT PRIMARY KEY,
    mandate_id          UUID NOT NULL REFERENCES mandates(mandate_id),
    amount              BIGINT NOT NULL,
    currency            TEXT NOT NULL DEFAULT 'INR',
    status              TEXT NOT NULL,
    receipt             TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Payments: tracks Razorpay payments captured against orders.
CREATE TABLE payments (
    payment_id          TEXT PRIMARY KEY,
    order_id            TEXT NOT NULL REFERENCES orders(order_id),
    mandate_id          UUID NOT NULL REFERENCES mandates(mandate_id),
    amount              BIGINT NOT NULL,
    currency            TEXT NOT NULL DEFAULT 'INR',
    status              TEXT NOT NULL,
    method              TEXT,
    captured            BOOLEAN DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payments_order ON payments (order_id);
CREATE INDEX idx_payments_mandate ON payments (mandate_id);

-- Refunds: tracks Razorpay refunds issued for mismatch recovery.
CREATE TABLE refunds (
    refund_id           TEXT PRIMARY KEY,
    payment_id          TEXT NOT NULL REFERENCES payments(payment_id),
    mandate_id          UUID NOT NULL REFERENCES mandates(mandate_id),
    amount              BIGINT NOT NULL,
    currency            TEXT NOT NULL DEFAULT 'INR',
    status              TEXT NOT NULL,
    idempotency_key     TEXT NOT NULL UNIQUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_refunds_payment ON refunds (payment_id);
CREATE INDEX idx_refunds_mandate ON refunds (mandate_id);

-- Intents: captures the buyer agent's original purchase intent.
CREATE TABLE intents (
    intent_id           UUID PRIMARY KEY,
    mandate_id          UUID NOT NULL REFERENCES mandates(mandate_id),
    product_id          TEXT NOT NULL,
    category            TEXT NOT NULL,
    expected_price      BIGINT NOT NULL,
    currency            TEXT NOT NULL DEFAULT 'INR',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Mismatches: detected intent-vs-outcome divergences.
CREATE TABLE mismatches (
    mismatch_id         UUID PRIMARY KEY,
    intent_id           UUID NOT NULL REFERENCES intents(intent_id),
    mandate_id          UUID NOT NULL REFERENCES mandates(mandate_id),
    kind                JSONB NOT NULL,
    status              TEXT NOT NULL CHECK (status IN ('detected', 'refund_initiated', 'refund_completed')),
    refund_id           TEXT REFERENCES refunds(refund_id),
    detected_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    refund_initiated_at TIMESTAMPTZ,
    refund_completed_at TIMESTAMPTZ
);

CREATE INDEX idx_mismatches_mandate ON mismatches (mandate_id);
CREATE INDEX idx_mismatches_status ON mismatches (status);
