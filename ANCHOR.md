# ANCHOR — Current Project State

> This file is context, not instructions. Rules and routing live in
> AGENTS.md. This file answers "where are we right now" — it should be
> updated at the end of every work session, by whichever agent did the
> work, so the next session (or a fresh/stuck agent) doesn't have to
> re-derive state from scratch.

**Last updated:** 2026-08-27 by the per-purchase nonce implementation session

## Current phase

Phase 7 complete (reconciliation). Per-purchase nonce replay protection
implemented and verified live. Phases 0–7 + nonce hardening done.

## Last completed

Implemented per-purchase nonce replay protection — the core fix that makes
the live gateway purchasable:

- **`PurchaseAuth` type** (`mandate-engine/src/purchase_auth.rs`): signed,
  per-purchase authorization carrying `auth_id`, `mandate_id`, fresh
  `nonce`, `amount`, `currency`, `product_id`, `category`, `created_at`.
  Signing payload covers immutable fields only; nonce is the replay token.
- **`MandateSigner::sign_auth` / `verify_auth`** (`signing.rs`): sign and
  verify purchase authorizations; `signing.rs` now has 6 tests (3 auth).
- **Fixed mandate `signing_payload()`** (`mandate.rs`): removed mutable
  fields (`spent_amount`, `status`, `nonce`) from the signing payload so
  the mandate signature remains valid across its lifecycle after spending.
- **Policy evaluator** (`evaluator.rs`): `evaluate(&mandate, &auth)` now
  verifies mandate signature + auth signature + expiry/budget/scope against
  the auth's amounts. Nonce check moved OUT of evaluator (consumed at DB
  layer).
- **`used_nonces` table** (`002_add_used_nonces.sql`): UNIQUE constraint
  is the trust-critical replay guard.
- **`db::used_nonce_repo`**: `mark_used` (atomic insert ON CONFLICT DO
  NOTHING) + `is_used` helper.
- **`db::mandate_repo::consume_and_increment_spent`**: single-transaction
  function that inserts nonce into `used_nonces` + increments `spent_amount`
  atomically; returns `DbError::Replay` on conflict.
- **`DbError::Replay`** variant added for clean replay detection.
- **Catalog-service `execute_purchase`**: builds + signs `PurchaseAuth`,
  passes to `PolicyEvaluator::evaluate`, uses `consume_and_increment_spent`
  atomically.
- **Removed `PgNonceChecker`** and `NonceChecker` trait (dead now);
  `nonce_exists` / `find_by_nonce` removed from `mandate_repo.rs`.
- **Buyer-agent not yet updated** — still sends old request schema; needs
  `max_amount`/`scope` for mandate request, `quantity`/`shipping_address`/
  `expected_price` for purchase.

### Verified live

Full end-to-end on `localhost:8000` with real Razorpay test keys:
1. Mandate issued → `e45ef150-...` (signed, 1h expiry)
2. Purchase prod-001 → order created, mismatch detected (expected 1299 vs
   actual 129900 paise — `expected_price` is in paise, catalog price is
   in paise; drift detected correctly)
3. Purchase prod-002 → order created, mismatch detected
4. Third purchase → blocked: OverBudget
5. Audit trail: mandate_issued → purchase_attempt → budget_debited →
   order_created → mismatch_detected → refund_failed (all correct)

Tests: 32 unit tests pass, clippy `-D warnings` clean.

## In progress right now

Nothing mid-flight.

## Blocked / waiting on

1. **Buyer-agent update** (`buyer-agent/main.py`): request schemas are
   outdated — needs wiring to the real gateway API so the demo buyer agent
   works end-to-end.

## Do not touch

Nothing is off-limits right now.

## Next task

Update `buyer-agent/main.py` to match the real gateway API so the full
buyer-agent → mandate → purchase flow works with the live gateway. After
that: dashboard/consent UI walkthrough, then Phase 8 (video/polish).
