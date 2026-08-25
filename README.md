# Merchant Agent Gateway

> **Razorpay AI Buildathon 2026 — Track 01: Agentic Commerce**
> Making Razorpay merchants safely transactable by autonomous AI buyer agents.

---

## The Problem

AI agents can now shop, compare, and transact on a user's behalf — but merchants have no standard way to trust them. The options today are:

- **Refuse agent traffic entirely** — lose revenue from an entire channel.
- **Trust the agent blindly** — no spend limits, no audit trail, no proof of authorization.

Razorpay built the consumer half: **Reserve Pay** lets users pre-authorize a spending limit via NPCI's SBMD mechanism. What's missing is the **merchant half** — a standard way for a merchant to say *here's my catalog, here's what an agent is allowed to do, here's proof you were authorized, and here's the audit trail if something goes wrong.*

That's what this project builds.

## What It Does

**Merchant Agent Gateway** — a trust layer that sits between an AI buyer agent and a Razorpay merchant's payment infrastructure. Every money-moving action is gated by a cryptographically signed mandate and a deterministic policy engine. Never an LLM.

```
Reference Buyer Agent (thin — LLM for intent parsing only)
        │
        │  1. discover catalog + manifest
        ▼
┌─────────────────────────────────────────┐
│         Merchant Agent Gateway           │
│                                          │
│  ┌──────────────────────────────────┐   │
│  │  Catalog / Manifest API          │   │  Machine-readable products,
│  │  GET /manifest  GET /catalog     │   │  prices, policies
│  └──────────────────────────────────┘   │
│  ┌──────────────────────────────────┐   │
│  │  Mandate Service (Ed25519)       │   │  Signed permission slip:
│  │  issue → sign → verify           │   │  max_amount, scope, expiry
│  └──────────────────────────────────┘   │
│  ┌──────────────────────────────────┐   │
│  │  Policy Engine (deterministic)   │   │  allow / block / escalate
│  │  Rust — no ML/LLM dependency    │   │  Auditable, line by line
│  └──────────────────────────────────┘   │
│  ┌──────────────────────────────────┐   │
│  │  Execution Orchestrator          │   │  Idempotent order + payment
│  │  Razorpay test-mode APIs         │   │  calls via Razorpay
│  └──────────────────────────────────┘   │
│  ┌──────────────────────────────────┐   │
│  │  Audit Log (append-only)         │   │  Every decision, timestamped,
│  │  Postgres + row-level locking    │   │  queryable, complete
│  └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
        │
        ▼
  Razorpay Test Mode
  ─ Orders API (mandate params: max_amount, expire_at)
  ─ Payments API (test UPI: success@razorpay / failure@razorpay)
  ─ Webhooks (async confirmation)
        │
        ▼
  Merchant Dashboard (Next.js)
  ─ View mandates, audit trail, failure recovery
```

## Key Concepts

| Concept | What it is |
|---|---|
| **Mandate** | A signed, scoped permission slip — max amount, category/SKU whitelist, expiry, merchant ID. Nothing moves money without one. Modeled on Razorpay Reserve Pay's SBMD shape. |
| **Policy Engine** | Deterministic Rust code that sits between "agent requests a purchase" and "payment fires." Checks mandate limits, allows/blocks/escalates. No LLM involved — ever. |
| **Audit Log** | Append-only Postgres record of every mandate issued, every check performed, every decision and why. Makes "explainable" a provable fact, not a pitch claim. |
| **Engineered Failure** | Catalog drift between mandate issuance and execution — intent honored, outcome wrong. Detected post-purchase, recovered via Razorpay Refund API. Full chain in the audit trail. |

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Core services | **Rust** (axum, ed25519-dalek, sqlx, tokio) | Trust-critical signing, deterministic policy, compile-time checked queries |
| Frontend | **Next.js 14** (App Router) + TypeScript + Tailwind | Fast dashboard build, server components by default |
| Database | **PostgreSQL** (Docker or Supabase) | Row-level locking for reconciliation, append-only audit schema |
| Payments | **Razorpay Test Mode** | Real API surface, no live money, test UPI IDs |
| Infra | **Docker Compose** | One-command local spin-up for demos |
| Buyer Agent | **Python or TypeScript** (thin client) | Deliberately the least-engineered piece — exists to prove the gateway works |

### What's deliberately not in the stack

No message queue (Kafka/Redis Streams) — Postgres row locks handle concurrency at this scale. No Kubernetes — Docker Compose is the right size. No crypto/stablecoin settlement — x402 stays a documented extension point, not a dependency. Naming these explicitly is a scope signal: they were chosen deliberately, not avoided because they were too hard.

## Build Phases

Sequenced so there's a demoable slice at every stage:

| Phase | Deliverable |
|---|---|
| 0 | Repo scaffold, Docker Compose, Postgres schema, Razorpay test keys |
| 1 | Catalog + merchant manifest (1–2 seeded merchants) |
| 2 | Mandate service — issue, sign, verify (Rust core) + consent UI |
| 3 | Policy engine — deterministic allow/block/escalate + decision log |
| 4 | Execution orchestrator — Razorpay Orders API, test UPI payment, idempotency |
| 5 | Reference buyer agent — discover → pick → request mandate → execute |
| 6 | Audit trail + merchant dashboard (Next.js) |
| 7 | Engineered failure scenario + recovery flow |
| 8 | Metrics, README polish, pitch video |

**If time runs short, cut from the bottom.** Phases 0–5 alone satisfy "merchant transactable by an AI buyer end to end." Phases 6–8 make it *legible* to judges.

## Project Structure

```
/
├── README.md
├── AGENTS.md                       (AI agent routing + coding standards)
├── agent-mandate-gateway-proposal.md
├── gateway/
│   ├── crates/
│   │   ├── mandate-engine/         (Ed25519 signing, mandate struct, verification)
│   │   ├── policy-engine/          (deterministic allow/block/escalate)
│   │   └── razorpay-client/        (Orders/Payments/Refunds API, webhooks)
│   └── services/
│       ├── catalog-service/        (product feed + merchant manifest)
│       └── reconciliation/         (intent-vs-outcome mismatch detection)
├── audit/
│   └── migrations/                 (Postgres audit log schema)
├── dashboard/                      (Next.js merchant dashboard + consent UI)
├── buyer-agent/                    (reference buyer agent)
├── tests/
│   └── integration/                (end-to-end + concurrency tests)
├── docker-compose.yml
└── .env.example
```

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 18+
- [Docker](https://docs.docker.com/get-docker/) & Docker Compose
- Razorpay test-mode API keys ([dashboard](https://dashboard.razorpay.com/))

### Setup

```bash
# 1. Clone the repo
git clone https://github.com/<your-username>/merchant-agent-gateway.git
cd merchant-agent-gateway

# 2. Copy environment template and fill in your Razorpay test keys
cp .env.example .env
# Edit .env with your RAZORPAY_KEY_ID and RAZORPAY_KEY_SECRET

# 3. Start everything
docker compose up --build

# 4. Run database migrations
docker compose exec gateway sqlx migrate run --source audit/migrations

# 5. Seed sample merchant data
curl -X POST http://localhost:8000/admin/seed
```

The gateway API runs on `http://localhost:8000`. The dashboard is at `http://localhost:3000`.

### Running Tests

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --test integration

# Lint (must pass before any commit)
cargo clippy --workspace -- -D warnings
```

## The Engineered Failure

Not a network timeout — those are solved problems. The hard case:

1. Buyer agent requests a mandate for "any item under ₹500 in category X"
2. Between mandate issuance and execution, the catalog price changes (simulated drift)
3. The agent's purchase still executes *inside* the mandate bounds — nothing violated procedurally
4. Post-purchase reconciliation detects the intent-vs-outcome mismatch
5. Gateway triggers a **Razorpay Refund API call** (real, test mode)
6. Full chain visible in the audit log: intent → mandate → execution → mismatch → refund → resolution

This proves "explainable" and "audit trail" are real, not just claims.

## Mandate Schema

Modeled on Reserve Pay's SBMD shape, extended with scope and agent identity:

```rust
struct Mandate {
    mandate_id:      Uuid,
    issued_at:       DateTime<Utc>,
    expires_at:      DateTime<Utc>,
    user_id:         String,         // the human who authorized this
    merchant_id:     String,
    buyer_agent_id:  String,
    max_amount:      i64,            // currency subunits (paise)
    currency:        String,         // "INR"
    scope:           Vec<String>,    // category/SKU whitelist
    frequency:       Frequency,      // one_time | recurring
    spent_amount:    i64,            // running counter, starts at 0
    status:          MandateStatus,  // active | revoked | expired | exhausted
    nonce:           String,         // replay protection
    signature:       Vec<u8>,        // Ed25519 sig over canonical JSON
}
```

## Dashboard Metrics

| Metric | What it shows |
|---|---|
| Mandate compliance rate | % of purchase attempts within mandate bounds |
| Blocked / escalated attempts | Count, broken down by which rule triggered |
| Transaction volume vs. mandate budget | Utilization tracking |
| Mean time to detect mismatch | Execution timestamp to reconciliation flag |
| Recovery success rate | Mismatches detected vs. resolved |
| Audit trail completeness | Every money action has a matching decision record |

## Safety & Security Principles

- **No LLM makes money-movement decisions.** LLMs are used only for natural-language intent parsing in the buyer agent.
- **No payment executes without a verified mandate.** Not in test mode, not for a quick test.
- **Every money action writes an audit entry first.** Not after — before the action is considered complete.
- **Secrets never enter git.** `.env` files are gitignored. `.env.example` contains placeholders only.
- **No `.unwrap()` on money-related paths.** All mandate/payment/signature handling uses `Result` propagation.

## Extension Points (Designed For, Not Built)

These are documented architecture seams — not in scope for the buildathon, but the gateway is designed to accommodate them:

- **ACP-shaped checkout negotiation** — the catalog schema is grounded in the real ACP Product Feed Spec (OpenAI + Stripe, Apache 2.0)
- **UAP agent registration** — once NPCI's Unified Agent Protocol is live, the mandate service maps directly to its authorization model
- **x402 machine-native settlement** — a documented adapter point for Coinbase's protocol, not a dependency

## License

MIT

---

Built for the [Razorpay AI Buildathon 2026](https://razorpay.com/buildathon/) — Track 01: Agentic Commerce.
