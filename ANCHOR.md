# ANCHOR — Current Project State

> This file is context, not instructions. Rules and routing live in
> AGENTS.md. This file answers "where are we right now" — it should be
> updated at the end of every work session, by whichever agent did the
> work, so the next session (or a fresh/stuck agent) doesn't have to
> re-derive state from scratch.

**Last updated:** 2026-08-27 — product up-leveling: honest gap assessment, super-app stack, ticket backlog written, AGENTS.md updated.

## Current phase

Phases 0–7 (trust-engine slices) complete and verified — the money core
is real. **A strict product review scored us ~28% of a sellable product**:
the trust engine is the strong 30%, but identity/auth, product UI,
continuous agents, key persistence, and deployment (the other ~70%) were
prototype-grade. We are now building the product layer toward a Tier-3
"super-app / protocol network" target, sequenced through demoable tickets.

**Target tier (super-app):** multi-tenant identity (merchant/agent/admin),
persisted Postgres source of truth, Redis for cache/jobs/rate-limit,
continuous agent worker, signing-key persistence at rest, full product
web app with live audit stream, CI/CD + `docker compose up` = whole stack.

## Last completed

0. **Product up-leveling decision + ticket backlog + workflow (this session):**
   - Honest gap assessment written (trust engine strong; identity/UX/deployment
     were prototype-grade; overall ~28% of a sellable product).
   - **Ticket backlog published** — 14 vertical tracer-bullet slices under
     `docs/product-backlog/issues/01..14-*.md` (see "Product tickets" below).
     Working/reference files live under `docs/` (gitignored); only the state
     summary in this file is ever committed. Tickets are cleaned from `docs/`
     once their work is done.
   - **AGENTS.md updated**: added `product-ux-agent` + `deployment-agent` to the
     roster, `ui-ux-pro-max` + `to-tickets` to the skill roster; added
     non-negotiable rules 7 (runs as a product, not a script) and 8 (no OpenAI —
     use opencode/OpenRouter free models; evaluate `rig` for Rust).
   - **Design system generated** into `docs/design-system/mandate-gateway/MASTER.md`
     (dark-OLED fintech, IBM Plex Sans, `#22C55E` money-accent) via ui-ux-pro-max.

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

## Product tickets (backlog — see `docs/product-backlog/issues/`)

14 tracer-bullet vertical slices, blockers declared. Start at the frontier
(no unblocked peers): **01 (signing-key persistence)** and **02 (catalog →
Postgres)** are unblocked and are the foundation everything else depends on.
Full dependency chain: 03←{01,02}; 04←03; 05←03; 06←{02,03,05};
07←{03,05}; 08←04; 09←{04,07}; 10←{06,07}; 11←04; 12←{03,11};
13←all; 14←{05,06,07}.

1. `01-signing-key-persistence` — keypair survives restart (fixes the FATAL
   in-memory regeneration). No blockers.
2. `02-catalog-merchants-postgres` — catalog/merchants out of memory → Postgres.
   No blockers.
3. `03-auth-service` — merchant/agent/admin signup+login, JWT sessions, RBAC.
4. `04-agent-api-keys` — scoped agent credentials, rotation, revoke.
5. `05-web-app-shell-design-system` — real app shell + locked design system.
6. `06-merchant-dashboard-catalog-mandates` — catalog CRUD + mandate approve/reject from UI.
7. `07-live-audit-stream` — real-time (SSE/websocket) audit stream.
8. `08-continuous-agent-worker` — agents run on a Redis-backed queue, not a script.
9. `09-agent-console` — agent's self-service console.
10. `10-reconciliation-ui-alerting` — make mismatch/refund visible + demoable.
11. `11-gateway-cache-ratelimit` — Redis cache + per-key rate limiting.
12. `12-admin-console-observability` — admin console + health/tracing.
13. `13-cicd-orchestration` — GitHub Actions + `docker compose up` = whole stack.
14. `14-marketing-landing-demo-polish` — landing page + filmable demo (Phase 8).

## In progress right now

Nothing mid-flight. Backlog published; awaiting user go-ahead on which
ticket the next session starts (recommended: **01 + 02** — the foundation).

## Blocked / waiting on

Nothing.

## Do not touch

Nothing is off-limits.

## Next task

Start the product backlog at the frontier. **Recommended first tickets:**
`01-signing-key-persistence` (fixes the fatal in-memory key regeneration)
and `02-catalog-merchants-postgres` (move catalog/merchants into Postgres).
Both are unblocked and everything else depends on them. Work the frontier
top-down per the blocking chain in the "Product tickets" section.

Stack/completion notes for whoever resumes:
- Test guidance: `cargo clippy -- -D warnings` in `gateway/` (workspace),
  `bun run lint` + `bun run build` in `dashboard/`.
- Design system: read `docs/design-system/mandate-gateway/MASTER.md` before any
  frontend slice; check `pages/<page>.md` overrides first.
- Models: use opencode/OpenRouter free models for any LLM intent-work, never
  OpenAI; for Rust LLM work evaluate the `rig` crate first.
- Skills: load `to-tickets` when the user says "tickets"/`/to-tickets`;
  load `ui-ux-pro-max` for every frontend slice.
