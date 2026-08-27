# ANCHOR — Current Project State

> This file is context, not instructions. Rules and routing live in
> AGENTS.md. This file answers "where are we right now" — it should be
> updated at the end of every work session, by whichever agent did the
> work, so the next session (or a fresh/stuck agent) doesn't have to
> re-derive state from scratch.

**Last updated:** 2026-08-27 by the dashboard wiring + buyer-agent session

## Current phase

Phases 0–7 complete. Dashboard consent + audit trail wired to real
gateway API. Buyer-agent updated. Ready for Phase 8 (video/polish).

## Last completed

1. **Per-purchase nonce replay protection** (`df1df22`):
   - `PurchaseAuth` type + `sign_auth`/`verify_auth` on `MandateSigner`.
   - Fixed mandate `signing_payload()` — removed mutable fields
     (`spent_amount`, `status`, `nonce`) so signature stays valid across
     the mandate lifecycle.
   - `used_nonces` table (`002_add_used_nonces.sql`), `consume_and_increment_spent`
     atomic nonce+budget transaction, `DbError::Replay` variant.
   - Evaluator now takes `&PurchaseAuth` — verifies auth signature + auth
     amounts. Nonce consumed at DB layer.
   - Removed `PgNonceChecker`, `NonceChecker` trait, `nonce_exists`.
   - Verified live: two purchases succeed, third blocked OverBudget, audit
     trail logs everything correctly.

2. **Buyer-agent updated** (`8814bcb`):
   - `request_mandate` sends `user_id`, `merchant_id`, `buyer_agent_id`,
     `max_amount`, `currency`, `scope`, `frequency`, `expires_in_hours`.
   - `execute_purchase` sends `mandate_id`, `product_id`, `quantity`,
     `shipping_address`, `expected_price`.
   - Verified live: agent discovers merchant, picks product, issues
     mandate, completes purchase end-to-end.

3. **Dashboard wired to real gateway** (`b30fae1`):
   - Added `GET /mandate/{id}` endpoint to gateway.
   - Fixed `approveMandate` (no-op — mandates are active on issuance)
     and `rejectMandate` (calls `POST /mandate/revoke`).
   - Added `/audit` page: full audit trail with decision badges, detail
     JSON, and timestamp.
   - Success page now links to audit trail.
   - Dashboard builds clean with `bun run build`.

## In progress right now

Nothing mid-flight.

## Blocked / waiting on

Nothing.

## Do not touch

Nothing is off-limits.

## Next task

Phase 8: video/polish pass. The full stack is working end-to-end:
- Gateway: mandate issuance, per-purchase auth, policy evaluation,
  Razorpay order creation, reconciliation, audit logging.
- Buyer-agent: discover → pick → mandate → purchase.
- Dashboard: consent page, approve/reject, success page, audit trail.

Test it all live: `bun run dev` in `dashboard/`, gateway running on
`:8000`, buyer-agent via `python3 buyer-agent/main.py`.
