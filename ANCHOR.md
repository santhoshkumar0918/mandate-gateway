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

0.5. **Ticket 01 — signing-key persistence at rest** (`4ab68fb`):
    - Signing key is generated once, encrypted with AES-256-GCM under an env
      master secret (`MANDATE_MASTER_KEY`), and stored in a new `keychain`
      table (`audit/migrations/003_keychain.sql`). Every boot loads the same
      key — a restart no longer invalidates previously issued mandates.
    - New `db::keychain_repo::KeychainRepo::load_or_create` (returns stored
      key, or persists a fresh one), `DbError::Crypto`, and
      `MandateSigner::from_key_bytes`.
    - Tests: unit (`from_key_bytes_reuses_same_key`), DB-backed integration
      (`load_or_create_with_restart`), and a live kill+restart boot verified a
      single unchanged key_id. Gateway port now env-configurable (`PORT`).
    - Also repaired pre-existing missing `reconciliation` imports in the
      integration harness so its tests run.
    - **Done — ticket file `docs/product-backlog/issues/01-*.md` deleted.**

0.6. **Ticket 02 — catalog + merchants to Postgres source of truth** (`bd3e997`,
    `fc85d70`, `309de50`, `712c9d5`):
    - Migration `004_catalog.sql`: `merchants` + `catalog` tables, seeded with
      merchant-001 + 8 products (idempotent).
    - `db::catalog_repo` + `db::merchant_repo` (Postgres-backed list/get/set_price,
      FromRow structs).
    - Gateway handlers (`get_catalog`, `simulate_drift`, `execute_purchase`,
      `get_manifest`) now read/write Postgres; removed the in-memory `CatalogStore`.
      Catalog is per-tenant (merchant_id); `MERCHANT_ID` env overrides default.
    - Test `db_catalog_tests::catalog_is_sourced_from_postgres` passes; live boot
      served 8 products + persisted manifest from Postgres.
    - **Done — ticket file `docs/product-backlog/issues/02-*.md` deleted.**

0.7. **Ticket 03 — auth service (roles + sessions + RBAC gate)** (`ebf0fda`,
    `2b8efb6`, `900d17f`, `e5f66f6`):
    - Migration `005_accounts.sql`: `accounts` table (role merchant/agent/admin,
      argon2id password_hash, tenant_id). JWT_SECRET + MANDATE_MASTER_KEY in
      `.env.example` + compose.
    - `db::auth_repo`: create_account (argon2id) + verify_password.
    - `auth.rs`: HS256 JWT issue/verify + `AuthUser` axum extractor (401 on
      missing/invalid token). Handlers: `POST /auth/signup`, `/auth/login`,
      `GET /auth/me` (protected).
    - Test `db_auth_tests::account_signup_and_password_verify` passes; live flow
      verified (signup→token→/me 200, no-token→401, login works).
    - NOTE: money endpoints not yet gated — that lands with ticket 04 (agent
      API keys) + ticket 06/09 (merchant/agent console scoping).
    - **Done — ticket file `docs/product-backlog/issues/03-*.md` deleted.**

0.8. **Ticket 04 — agent API keys (scoped credentials, rotation, revoke)** (`54ee66b`,
    `cf2c9f0`, `955d680`, `17f5d6d`, `f52dd83`):
    - Migration `006_api_keys.sql` (key_id, account_id FK, sha256 key_hash,
      scopes JSONB, tenant_id, revoked, last_used_at).
    - `db::api_key_repo`: create_key (generates `magw_`+32B, stores hash only),
      verify_key (hashes, updates last_used_at, rejects revoked), revoke, list.
    - `auth.rs`/`main.rs`: `ApiKey` axum extractor (Bearer `magw_…`); handlers
      `POST/GET /agents/keys`, `DELETE /agents/keys/{id}`; `issue_mandate` +
      `execute_purchase` now require `ApiKey` and scope-check (`mandate:issue`,
      `purchase:exec`). `buyer_agent_id` derived from key tenant_id.
    - Buyer-agent rewritten to sign up → issue scoped key → key-auth every
      money call.
    - Live verified: full keyed purchase (Razorpay order created); no-key→401,
      revoked key→401, wrong-scope→403. Test `api_key_create_verify_revoke` passes.
    - **Done — ticket file `docs/product-backlog/issues/04-*.md` deleted.**

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

10 remaining tracer-bullet vertical slices. Work the frontier (no unblocked
peers): **05 (web-app shell + design system)** is now unblocked (03 DONE; 04
also DONE). Full remaining chain: 06←{02,03,05}; 07←{03,05}; 08←04; 09←{04,07};
10←{06,07}; 11←04; 12←{03,11}; 13←all; 14←{05,06,07}.

1. ~~`01-signing-key-persistence`~~ — **DONE** (`4ab68fb`), file deleted.
2. ~~`02-catalog-merchants-postgres`~~ — **DONE** (`bd3e997`+follow-ups), file deleted.
3. ~~`03-auth-service`~~ — **DONE** (`ebf0fda`+follow-ups), file deleted.
4. ~~`04-agent-api-keys`~~ — **DONE** (`54ee66b`+follow-ups), file deleted.
5. `05-web-app-shell-design-system` — product web app shell + design system. No blockers. (NEXT.)
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

Ticket **05: web-app shell + design system (auth → dashboard)** — unblocked now
that 01, 02, 03, 04 are done.

## Blocked / waiting on

Nothing.

## Do not touch

Nothing is off-limits.

## Next task

Work the backlog frontier. **Ticket 02 `02-catalog-merchants-postgres`
(move catalog + merchants out of the in-memory `CatalogStore` into
Postgres) is next and unblocked.** Work the frontier top-down per the
blocking chain in the "Product tickets" section.

Stack/completion notes for whoever resumes:
- Test guidance: `cargo clippy -- -D warnings` in `gateway/` (workspace),
  `bun run lint` + `bun run build` in `dashboard/`.
- Design system: read `docs/design-system/mandate-gateway/MASTER.md` before any
  frontend slice; check `pages/<page>.md` overrides first.
- Models: use opencode/OpenRouter free models for any LLM intent-work, never
  OpenAI; for Rust LLM work evaluate the `rig` crate first.
- Skills: load `to-tickets` when the user says "tickets"/`/to-tickets`;
  load `ui-ux-pro-max` for every frontend slice.
