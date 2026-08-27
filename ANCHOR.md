# ANCHOR — Current Project State

> This file is context, not instructions. Rules and routing live in
> AGENTS.md. This file answers "where are we right now" — it should be
> updated at the end of every work session, by whichever agent did the
> work, so the next session (or a fresh/stuck agent) doesn't have to
> re-derive state from scratch.

**Last updated:** 2026-08-27 by the reconciliation work session

## Current phase

Phase 7 — Reconciliation (engineered failure: catalog drift → mismatch
detection → refund recovery). Phases 0–6 complete.

## Last completed

Wired the reconciliation flow end-to-end and committed it on `main`:

- `ReconcileService` orchestrating detect → persist → refund with a
  `RefundProvider` seam (Razorpay-backed real impl, in-memory fake in
  tests).
- `db::reconcile` with `lock_mandate_and_check_refund_exists` (row-lock
  serialization + idempotency short-circuit) and `record_recovery`
  (refund insert + mismatch update + `refund_triggered` audit in one
  transaction).
- Catalog-service keeps a mutable `CatalogStore`, records the buyer's
  intended price on purchase, reconciles the live charge against it, and
  exposes `POST /admin/simulate-drift`.
- Fixed audit `decision` values to `allow`/`block`/`escalate` (were
  `allowed`/`blocked`/`escalated`), which were violating the DB CHECK
  constraint.
- Added 2 DB-backed integration tests (drift→refund, idempotency/no
  double refund); all 5 integration tests and full workspace pass; clippy
  `-D warnings` clean.
- Fixed git hygiene: `tests/integration/target/` artifacts untracked +
  gitignored.

Commits: `70f8b4d`, `41aba55`, `4b12085` (local).

Also hardened the state-discipline workflow: added **section 0 "The state
contract"** to AGENTS.md (read ANCHOR.md before any code, update it as part
of every builder agent's closing sequence, and make it a required step in
the definition of done). Committed as `6d818ad`.

## In progress right now

Nothing mid-flight. The reconciliation flow is complete and committed.
All integration + workspace tests green.

## Blocked / waiting on

Nothing. Two documented, non-blocking reconciliation limitations (for a
future hardening pass, not blockers):

1. The mandate row lock is released before the external refund call; a
   crash between a successful Razorpay refund and the DB insert could
   double-refund at Razorpay (DB unique `refunds.idempotency_key` only
   guards the recorded double-refund, not the live API call — no
   idempotency key is sent to Razorpay's create_refund).
2. The live purchase path sets `payment_id: ""`, so real drift would
   audit `refund_failed` (graceful) rather than issue a live refund; the
   full refund path is proven by the fake-provider test.

## Do not touch

Nothing is off-limits right now.

## Next task

Commit + push the AGENTS.md state-contract hardening (section 0, closing
sequence, definition-of-done step) along with the ANCHOR.md update. After
that, the natural next step is an end-to-end run of the drift scenario
against the running gateway + dashboard, or a hardening pass on the two
limitations above.
