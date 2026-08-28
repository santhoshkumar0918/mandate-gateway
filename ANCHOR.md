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

0.9. **Ticket 05 — web-app shell + design system** (`7583da7`, `67d6d2b`,
    `0aea822` + `f893695` lockfile):
    - Dark OLED fintech design system applied to `dashboard` (tokens + IBM Plex
      Sans, `#22C55E` accent) matching `docs/design-system/mandate-gateway/MASTER.md`.
    - Cookie auth (`mg_token`) + Next 16 `proxy.ts` route guard (redirect
      unauthenticated → /login). `auth-client.ts` set/get/clear; `gateway-api.ts`
      extended with signup/login/me/keys + token-aware catalog/audit.
    - App shell (`AppShell.tsx`): sidebar nav (Dashboard/Catalog/Mandates/Audit/
      Agents) + sign-out. Login + signup pages call gateway auth.
    - Pages: dashboard home (KPI cards), catalog (live products), agents (issue/
      revoke scoped keys), mandates (placeholder, ticket 06), plus existing
      consent/audit folded under the guard.
    - `bun run build` passes. Live verified: /login 200; / → 307 /login; authed
      / renders "Merchant dashboard", /catalog shows live "Wireless Mouse",
      /agents shows key manager.
    - **Done — ticket file `docs/product-backlog/issues/05-*.md` deleted.**

0.10. **Ticket 06 — merchant dashboard: catalog + mandate console** (`43ecf4c`,
    `f7f03ea`):
    - Gateway: `GET /mandates?merchant_id=` (merchant-scoped list via
      `MandateRepo::list_by_merchant`, JWT `AuthUser`) and
      `POST /catalog/{id}/price` (merchant price edit, `AuthUser`).
      `to_detail` helper shared by detail + list.
    - Dashboard: real mandate console (list with status, revoke →
      `/mandate/revoke`, links to review/audit) and editable catalog (live price
      updates via `updateCatalogPrice`). `gateway-api.ts` gained `getMandates` +
      `updateCatalogPrice`.
    - `bun run build` passes. Live verified: merchant `/mandates` lists the
      issued mandate; `/catalog/prod-001/price` updates and the catalog page
      reflects the new price.
    - **Done — ticket file `docs/product-backlog/issues/06-*.md` deleted.**

0.11. **Ticket 07 — live audit stream** (`855782e`, `1ff34de`):
    - Gateway: `audit_repo::list_recent` + `GET /audit` (JWT `AuthUser`, filters
      `mandate_id`/`event_type`/`decision`/`limit`) returning recent events.
    - Dashboard: `(app)/audit/page.tsx` live feed — polls every 2.5s, LIVE
      pulse indicator, decision/event filters, new-row fade-in. Replaces the
      old root `audit/page.tsx` (now under the shell). `getAuditFeed` added.
    - `bun run build` passes. Gateway `GET /audit` verified returning real
      events (mandate_issued, policy.evaluate, payment events).
    - **Done — ticket file `docs/product-backlog/issues/07-*.md` deleted.**

0.12. **Ticket 08 — continuous agent worker (Redis + queue)** (`72a977b`, `ea98b48`):
    - `buyer-agent/worker.py`: runs as a managed worker, not a one-shot script.
      Prod consumes tasks from a Redis list (`agent:tasks`) and posts step
      events to a Redis stream (`agent:events`); local/dev falls back to a
      timed loop (no Redis needed). Authenticates with a provisioned scoped
      API key (ticket 04); idempotent via a processed-task set; graceful
      SIGINT/SIGTERM shutdown.
    - `buyer-agent/Dockerfile` installs redis + defaults to `worker.py`;
      `docker-compose.yml` adds `redis` + wires the worker (`REDIS_URL`,
      depends_on gateway+redis).
    - Verified locally (loop mode): authenticated, performed a full
      discover→mandate→purchase run, then shut down gracefully on signal.
      (Queue mode path requires Redis to exercise — installed in the image.)
    - **Done — ticket file `docs/product-backlog/issues/08-*.md` deleted.**

0.13. **Ticket 09 — agent console** (`9ab66f2`):
    - `(app)/agents/page.tsx` expanded into a full operator console: scoped API
      keys (issue/revoke, raw key shown once), live agent activity feed (polls
      `GET /audit` every 3s, filtered to agent actors/events, new-row fade-in),
      and run stats (mandates issued / purchases / blocked). Nav label →
      "Agent Console".
    - `bun run build` passes; page renders (keys + stats + activity sections).
      Activity populates client-side from the verified audit feed.
    - **Done — ticket file `docs/product-backlog/issues/09-*.md` deleted.**

0.14. **Ticket 10 — reconciliation UI + alerting** (`9ec281e`, `1911143`):
    - Gateway: `mismatch_repo::list_recent` + `GET /reconciliation/mismatches`
      (JWT `AuthUser`) returning mismatch rows (kind/status/refund_id).
    - Dashboard: `(app)/reconciliation/page.tsx` — polls every 4s, red alert
      banner for unreconciled mismatches, per-mismatch cards with status badge
      (detected/refund_initiated/refund_completed). Nav item "Reconciliation".
      `getMismatches` added to `gateway-api.ts`.
    - `bun run build` passes; endpoint verified 200. (Population depends on the
      reconciliation engine detecting at payment capture — test-mode orders
      stay `created`, so live mismatches appear once a payment is captured; the
      engine + insert path already exist from the trust-engine phases.)
    - **Done — ticket file `docs/product-backlog/issues/10-*.md` deleted.**

0.15. **Ticket 11 — gateway cache + rate-limit** (`69336b4`):
    - `AppState` gained `catalog_cache: Arc<Mutex<Option<(Instant, rows)>>>` (10s TTL)
      and `rate_limiter: Arc<Mutex<HashMap<key_id,(Instant,u64)>>>`. `get_catalog`
      now serves via `get_cached_catalog` (TTL-bounded, cheaper under agent polling).
      `check_rate_limit` enforces a 30/60s fixed window per API key on
      `issue_mandate` + `execute_purchase` → `429 TOO_MANY_REQUESTS`.
    - `cargo clippy -D warnings` clean; verified live: 35 mandate calls on one key
      → 30 OK then 429; `/catalog` cached 200.
    - **Done — ticket file `docs/product-backlog/issues/11-*.md` deleted.**

0.16. **Ticket 12 — admin console + observability** (`fe08df2`, `0122cd3`):
    - Gateway `GET /admin/metrics` (admin role only → 403 otherwise) aggregates
      mandates/active, orders, captured volume (paise), blocked decisions,
      mismatches/unresolved, merchants/agents/admins, active API keys.
    - Dashboard `(app)/admin/page.tsx` polls every 5s; nav shows "Admin" only
      for `role==admin` (server `fetchMe` + client role cookie). `getAdminMetrics`
      added; signup/login persist `mg_role`.
    - `bun run build` passes; verified: admin 200 (real aggregates: 71 mandates,
      18 unresolved mismatches, etc.), merchant 403.
    - **Done — ticket file `docs/product-backlog/issues/12-*.md` deleted.**

0.17. **Ticket 13 — CI/CD + orchestration** (`5dba240`, `7cf31e3`, `fc93044`,
      `2149486`, `9a189f4`, `aae0c8f`):
    - `docker compose up` now brings up the WHOLE stack (postgres, gateway,
      dashboard, redis, buyer-agent) and is live-verified: gateway `/manifest`
      200, dashboard `/login` 200, authenticated `/catalog` 200 (server-side
      `fetchMe` reaches `gateway:8000` via `GATEWAY_URL`), merchant signup →
      JWT, admin metrics 403 for non-admin, and the **continuous buyer agent
      purchases live** (discover→pick→mandate→`order_id`, status `created`).
    - Dockerfiles fixed: gateway Rust 1.96 + openssl dev/runtime; dashboard
      `output: standalone` + server/client gateway URL split; buyer-agent
      unbuffered + resilient queue mode (self-drives when queue idle, tolerates
      Redis errors, `socket_timeout` above blpop interval).
    - CI: `.github/workflows/ci.yml` (gateway clippy+build, dashboard build,
      `docker compose build`).
    - **Trust-engine bug found & fixed during docker verify:** mandate
      signature covered `issued_at`/`expires_at`, which Postgres `timestamptz`
      truncates to microseconds → every freshly-issued mandate failed its own
      verification after the DB round-trip (`ReplayDetected`). Fixed by
      excluding the timestamps from the signed payload (`5dba240`). Old
      mandates already in the DB are now unverifiable (test data only).
    - **Done — ticket file `docs/product-backlog/issues/13-*.md` deleted.**

0.18. **Ticket 14 — marketing landing + demo polish** (`018092a`):
    - New public root `dashboard/src/app/page.tsx`: dark fintech landing
      (hero, 3-step flow, trust pillars, live-agent note, CTAs to /login &
      /signup). Authenticated dashboard home relocated to `/dashboard`;
      AppShell nav, login/signup redirects, and `proxy.ts` (treats `/` as
      public) updated. `bun run build` passes; verified `/` 200 (marketing),
      `/dashboard` 307 without token, `/login` 200. Dashboard image rebuilt.
    - The filmable end-to-end demo is live: `docker compose up` runs the
      continuous buyer agent (discover→mandate→purchase, streamed to the
      Agent Console + audit + reconciliation views).
    - **Done — ticket file `docs/product-backlog/issues/14-*.md` deleted.**

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

 0 remaining tracer-bullet vertical slices. **Backlog complete — all 14
 tickets shipped.** Full chain resolved: 14←{05,06,07}; 13←all; 12←{03,11};
 11←04; 10←{06,07}; 09←{04}; 08←04; 07←06; 06←02; 05←03; 04←03; 03←02; 02←01;
 01 standalone.

 1. ~~`01-signing-key-persistence`~~ — **DONE** (`4ab68fb`), file deleted.
 2. ~~`02-catalog-merchants-postgres`~~ — **DONE** (`bd3e997`+follow-ups), file deleted.
 3. ~~`03-auth-service`~~ — **DONE** (`ebf0fda`+follow-ups), file deleted.
 4. ~~`04-agent-api-keys`~~ — **DONE** (`54ee66b`+follow-ups), file deleted.
 5. ~~`05-web-app-shell-design-system`~~ — **DONE** (`7583da7`+follow-ups), file deleted.
 6. ~~`06-merchant-dashboard-catalog-mandates`~~ — **DONE** (`43ecf4c`+follow-ups), file deleted.
 7. ~~`07-live-audit-stream`~~ — **DONE** (`855782e`+follow-ups), file deleted.
 8. ~~`08-continuous-agent-worker`~~ — **DONE** (`72a977b`+follow-ups), file deleted.
 9. ~~`09-agent-console`~~ — **DONE** (`9ab66f2`), file deleted.
 10. ~~`10-reconciliation-ui-alerting`~~ — **DONE** (`9ec281e`+follow-ups), file deleted.
 11. ~~`11-gateway-cache-ratelimit`~~ — **DONE** (`69336b4`), file deleted.
 12. ~~`12-admin-console-observability`~~ — **DONE** (`fe08df2`+follow-ups), file deleted.
 13. ~~`13-cicd-orchestration`~~ — **DONE** (`5dba240`+follow-ups), file deleted.
 14. ~~`14-marketing-landing-demo-polish`~~ — **DONE** (`018092a`), file deleted.
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

Demo-readiness + UX polish pass (post-backlog, user-requested for judges):
- DONE: Added `scripts/seed_demo.py` (idempotent) that creates a demo merchant
  mapped to `merchant-001` (the live catalog owner) + a demo admin + a scoped
  agent key + one immediate purchase + a reconciliation mismatch. Run AFTER
  `docker compose up`: `python3 scripts/seed_demo.py`.
- DONE: Login page shows a "Live demo unlocked" banner with
  `demo@merchant.local` / `Demo@1234` (merchant) and `admin@merchant.local` /
  `Admin@1234` (admin).
- DONE: Landing page redesigned (premium dark fintech, green shades, hero
  mandate card, how-it-works, live-agent showcase, security section, CTA).
- DONE: AppShell nav upgraded with SVG icons (no emoji), grouped Store /
  Platform, live "signed in" indicator, "Try demo" link.
- DONE: Dashboard Overview now shows KPI cards (live products, active mandates,
  purchases 24h, blocked, catalog value) + a live agent-activity feed + a
  getting-started panel for empty stores.
- DONE: Pagination added to catalog / mandates / audit / agents-activity /
  reconciliation.
- DONE: Frontend event types aligned to the REAL audit events
  (`mandate_issued`, `purchase_attempt`, `order_created`, `budget_debited`,
  `order_reconciled`) — earlier code referenced non-existent `payment.captured`
  etc., so KPIs/feeds showed zero.
- DONE: Gateway CORS layer added so the browser dashboard can call the gateway
  cross-origin (`localhost:3000` → `localhost:8000`).
- NOTE: each real merchant gets its own isolated tenant, so a freshly created
  account starts EMPTY by design (correct product behavior). The demo accounts
  are the populated showcase.

## Blocked / waiting on

Nothing.

## Do not touch

Nothing is off-limits.

## Next task

Record a short demo video / write a one-page judge walkthrough: sign in as the
demo merchant → watch Overview KPIs + live agent activity update → open
Mandates/Audit/Reconciliation to see the gated purchases and the auto-recovered
price-drift mismatch. Optionally enrich the catalog (add `007_catalog_extra.sql`
+ a product-create API) for a fuller Catalog page.

Stack/completion notes for whoever resumes:
- Test guidance: `cargo clippy -- -D warnings` in `gateway/` (workspace),
  `npx tsc --noEmit` + `npx eslint src` in `dashboard/` (the
  `react-hooks/set-state-in-effect` rule is downgraded to a warning because the
  dashboard intentionally polls the gateway from client effects).
- Demo runbook: `docker compose up` then `python3 scripts/seed_demo.py`.
- Design system: read `docs/design-system/mandate-gateway/MASTER.md` before any
  frontend slice; check `pages/<page>.md` overrides first.
- Models: use opencode/OpenRouter free models for any LLM intent-work, never
  OpenAI; for Rust LLM work evaluate the `rig` crate first.
- Skills: load `to-tickets` when the user says "tickets"/`/to-tickets`;
  load `ui-ux-pro-max` for every frontend slice.
