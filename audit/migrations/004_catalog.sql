-- Catalog + merchants as Postgres source of truth.
--
-- Replaces the in-memory `HashMap` catalog (catalog_store.rs) so products and
-- merchants survive restarts and become per-tenant. A product is owned by a
-- merchant; the manifest endpoint reads from `merchants`.

CREATE TABLE merchants (
    merchant_id       TEXT PRIMARY KEY,
    name              TEXT NOT NULL,
    supported_scopes  JSONB NOT NULL,
    capability_version TEXT NOT NULL,
    catalog_endpoint  TEXT NOT NULL,
    mandate_endpoint  TEXT NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE catalog (
    product_id    TEXT NOT NULL,
    merchant_id   TEXT NOT NULL REFERENCES merchants(merchant_id),
    offer_id      TEXT NOT NULL,
    title         TEXT NOT NULL,
    description   TEXT NOT NULL,
    category      TEXT NOT NULL,
    price         BIGINT NOT NULL,
    currency      TEXT NOT NULL,
    availability  TEXT NOT NULL,
    inventory_count BIGINT NOT NULL DEFAULT 0,
    seller_name   TEXT NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (product_id, merchant_id)
);

CREATE INDEX idx_catalog_merchant ON catalog (merchant_id);

-- Seed demo merchant + products (idempotent: only inserted if absent).
INSERT INTO merchants (merchant_id, name, supported_scopes, capability_version, catalog_endpoint, mandate_endpoint)
SELECT 'merchant-001', 'TechStore Demo',
       '["electronics","accessories","home"]'::jsonb, '1.0',
       '/catalog', '/mandate'
WHERE NOT EXISTS (SELECT 1 FROM merchants WHERE merchant_id = 'merchant-001');

INSERT INTO catalog (product_id, merchant_id, offer_id, title, description, category, price, currency, availability, inventory_count, seller_name)
SELECT v.product_id, 'merchant-001', v.offer_id, v.title, v.description, v.category, v.price, 'INR', v.availability, v.inventory_count, 'TechStore Demo'
FROM (VALUES
    ('prod-001', 'off-001', 'Wireless Mouse', 'Ergonomic wireless mouse with USB-C receiver', 'electronics', 129900, 'in_stock', 50),
    ('prod-002', 'off-002', 'USB-C Hub 7-in-1', 'HDMI, USB-A x3, SD, microSD, USB-C PD', 'accessories', 249900, 'in_stock', 30),
    ('prod-003', 'off-003', 'Mechanical Keyboard', 'Hot-swappable 75% mechanical keyboard', 'electronics', 399900, 'in_stock', 25),
    ('prod-004', 'off-004', '4K Webcam', 'USB webcam with auto-framing', 'electronics', 549900, 'in_stock', 15),
    ('prod-005', 'off-005', 'Desk Lamp', 'LED desk lamp with warm/cool presets', 'home', 89900, 'in_stock', 60),
    ('prod-006', 'off-006', 'Laptop Stand', 'Aluminium foldable laptop stand', 'accessories', 149900, 'in_stock', 40),
    ('prod-007', 'off-007', 'Noise-Cancel Headphones', 'Over-ear ANC headphones', 'electronics', 799900, 'preorder', 10),
    ('prod-008', 'off-008', 'USB-C Cable 2m', 'Braided 100W USB-C cable', 'accessories', 49900, 'in_stock', 100)
) AS v(product_id, offer_id, title, description, category, price, availability, inventory_count)
WHERE NOT EXISTS (SELECT 1 FROM catalog WHERE merchant_id = 'merchant-001');
