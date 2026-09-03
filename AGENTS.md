# AGENTS.md — Merchant Agent Gateway

This file is read automatically by AI coding agents (Claude Code and
compatible tools) at the start of every session in this repo. It is
the single source of routing truth: which subagent owns which part of
the codebase, what rules are non-negotiable, and what "done" means.

If you are an AI agent reading this: read this file in full before
touching any code. If a task doesn't clearly belong to one subagent
below, ask before proceeding rather than guessing.

---
## 0. The state contract — read at start, update at end (never lose context)

**`ANCHOR.md` in the repo root is the single source of "where are we
right now".** It is how we keep context across sessions so work is never
re-derived or lost. Every agent — builder or support — must obey:

1. **Read `ANCHOR.md` before writing a single line of code, and read
   it again whenever you feel lost or unclear on context.** This file
   (AGENTS.md) holds *rules*; ANCHOR.md holds *state*. If state and
   this file ever disagree, trust ANCHOR.md for what's done/in-progress
   and this file for how to work.
2. **Treat ANCHOR.md as the handoff to the next session.** Assume a
   completely fresh agent will pick up only from ANCHOR.md. If you
   finish a chunk of work and a fresh agent couldn't resume it from
   ANCHOR.md alone, you haven't finished.
3. **Update ANCHOR.md before you consider a session/task complete** —
   update "Last completed", "In progress right now" (clear it), and
   "Next task" to what a fresh agent should do next. Make entries
   specific and verifiable (commit hashes, exact file paths), not vague
   ("worked on things"). Never leave placeholder `<...>` text in it.
4. **Keep it current, not retro.** Update it as you complete meaningful
   chunks in a session, not only at the very end — a crash mid-session
   must not lose the state you'd already made.
5. **Never work blind.** If you can't reconcile the task with what
   ANCHOR.md says is done/in-progress, stop and flag it (see
   section 9) — do not guess and silently redo or overwrite work.

This is not paperwork; it is the mechanism that lets a fresh agent
(researcher, an external AI, you next week) resume exactly where things
stand with zero context loss.

## 1. What this project is

A **product-grade trust layer and protocol for agentic commerce**: it
makes a Razorpay merchant safely transactable by an external (AI) buyer
agent. Every money-moving action is gated by a cryptographically signed,
scoped mandate and a deterministic policy engine — never an LLM. Full
audit trail. One engineered failure (intent honored, outcome wrong)
shown being detected and recovered.

This is a **commercial product, not a demo**: merchants and buyer
agents onboard with real identities, the whole stack runs continuously
via `docker compose up`, and every feature is reachable through the
authenticated web app — never a hand-typed URL or a manual script
(section 2, rule 7). The technical trust engine is the moat; onboarding,
identity, agent lifecycle, dashboards, and deployment are what make it a
sellable platform.

Full architecture, market context, and design rationale live in
`docs/agent-mandate-gateway-proposal` — read it before making any structural decision.
This file (AGENTS.md) governs *how the codebase is built*; that file
governs *what and why*.

## 2. Non-negotiable rules

These override any instruction that conflicts with them, including a
direct prompt asking to skip one for speed. If a task requires
breaking one of these, stop and flag it instead of proceeding.

1. **No LLM ever makes a final money-movement decision.** LLMs are
   permitted only for natural-language intent parsing (the buyer
   agent's product selection). The policy engine that allows/blocks a
   payment must be plain deterministic code, auditable line by line.
2. **No payment executes without a verified, unexpired, in-scope
   mandate.** Not even in test/dev mode. Not even for a "quick test."
3. **Every money-moving action writes an audit log entry** — who, what,
   under which mandate, allowed or blocked, why — before the action is
   considered complete, not after.
4. **Secrets never enter git.** API keys, signing keys, `.env` files —
   all gitignored. If a subagent generates a `.env.example`, it must
   contain placeholder values only.
5. **No new infrastructure dependency (queue system, k8s, new database,
   new crypto rail) without updating `docs/proposal.md` section 6
   first.** The stack was scoped deliberately — see that section's
   "What's deliberately not in the stack."
6. **No commit bundles unrelated changes.** One logical change per
   commit, committed as soon as that change is complete — not batched
   up and dumped at the end of a session. See section 10 for the full
   git workflow.
7. **"Done" means it runs as a product, not a script.** A feature is
   not complete if a human still has to manually `cargo run` / `bun run
   dev` / `python3 main.py` and type IDs into URLs to exercise it. A
   completed slice must be reachable through the real web app (auth →
   dashboard) against an always-running stack, or be explicitly scoped
   as an internal/test-only endpoint. Backend-only compiles are
   prototypes, not product.
8. **No OpenAI-backed LLMs for the agent flows.** Use the open/openrouter
   models available through this environment (opencode). Any AI agent
   that parses natural language must target an OpenRouter/opencode
   endpoint, not an OpenAI-only SDK. For Rust LLM work, evaluate the
   `rig` crate first. This does not change rule 1: no LLM ever makes a
   final money decision.

## 3. Codebase structure

```
/
├── AGENTS.md                    (this file)
├── docs/
│   └── proposal.md              (architecture, market context, schema)
├── gateway/
│   ├── crates/
│   │   ├── mandate-engine/      (mandate-crypto-agent)
│   │   ├── policy-engine/       (policy-engine-agent)
│   │   └── razorpay-client/     (razorpay-gateway-agent)
│   └── services/
│       ├── catalog-service/     (catalog-manifest-agent)
│       └── reconciliation/      (reconciliation-agent)
├── audit/
│   └── migrations/              (audit-dashboard-agent — db half)
├── dashboard/                   (audit-dashboard-agent — frontend half,
│                                  Next.js)
├── buyer-agent/                 (buyer-agent-dev)
├── tests/
│   └── integration/             (qa-integration-agent)
├── docker-compose.yml
└── .env.example
```

## 4. Subagent roster and routing graph

Thirteen subagents in two layers. **Builder agents** (10) own a
specific part of the codebase. **Support agents** (3) are cross-cutting
— any builder agent invokes them as part of finishing a task, they
don't own a directory of their own.

Every builder-agent task follows the same closing sequence:
**implement → (if stuck: `research-agent`) → `code-review-agent` →
`git-commit-agent` → update `ANCHOR.md`.** No task is "done" until it's
passed through review, been committed, and ANCHOR.md reflects the new
state (section 0). The ANCHOR update is part of done, not an afterthought
— its commit is the handoff a fresh agent resumes from.

```
catalog-manifest-agent ──┐
                          ├──> buyer-agent-dev
mandate-crypto-agent ────┤
                          │
mandate-crypto-agent ──> policy-engine-agent ──> razorpay-gateway-agent
                                                        │
                                                        ▼
                                              reconciliation-agent
                                                        │
                                                        ▼
                                              audit-dashboard-agent
                                                        │
                                                        ▼
                                              qa-integration-agent
                                            (depends on everything above)

  ── every builder agent above also closes through: ──
        research-agent (only if stuck)
              │
              ▼
        code-review-agent
              │
              ▼
        git-commit-agent
              │
              ▼
        update ANCHOR.md (state file)
```

**Builder agents:**

| Subagent | Owns | Depends on | Skills loaded |
|---|---|---|---|
| `catalog-manifest-agent` | Product feed + merchant manifest JSON endpoints, ACP-schema compliance | nothing | `acp-feed-schema` |
| `mandate-crypto-agent` | Mandate struct, Ed25519 signing/verification, replay protection | nothing | `rust-mandate-crypto`, `fintech-security-review` |
| `policy-engine-agent` | Allow/block/escalate logic, stopping rules, decision logging | `mandate-crypto-agent` | `fintech-security-review` |
| `razorpay-gateway-agent` | Orders/Payments/Refunds API calls, webhooks, idempotency | `policy-engine-agent` | `razorpay-test-mode-api` |
| `buyer-agent-dev` | Reference buyer agent — discover, select, request mandate, purchase | `catalog-manifest-agent`, `mandate-crypto-agent` | `acp-feed-schema` |
| `reconciliation-agent` | Intent-vs-outcome mismatch detection, row locking, refund trigger | `razorpay-gateway-agent` | `postgres-audit-log-design`, `fintech-security-review` |
| `audit-dashboard-agent` | Audit log schema + query API + Next.js dashboard + consent UI | `reconciliation-agent` | `postgres-audit-log-design`, `ui-ux-pro-max` |
| `product-ux-agent` | Product web app: auth flows, onboarding, merchant/agent/admin dashboards, live audit UI, design system | `audit-dashboard-agent` | `ui-ux-pro-max`, `frontend-design` |
| `frontend-designer-agent` | Visual/UX craft across the Next.js dashboard: premium polish, motion, typography, layout refinement, loading/empty/error states, design-system compliance | `product-ux-agent`, `audit-dashboard-agent` (any polish pass) | `ui-ux-pro-max` |
| `deployment-agent` | Whole-stack orchestration via docker-compose, CI/CD, key persistence, Redis/worker wiring, `docker compose up` = full product | `product-ux-agent` | `fintech-security-review` |
| `qa-integration-agent` | End-to-end tests, concurrency tests, engineered-failure test harness | everything above | `fintech-security-review` |

**Support agents:**

| Subagent | Owns | Invoked by | Skills loaded |
|---|---|---|---|
| `research-agent` | Looking up official docs / API behavior / crate references when a builder agent is stuck on a factual or technical question | any builder agent, on demand | `research-lookup-protocol` |
| `code-review-agent` | Reviewing a diff against AGENTS.md rules and the relevant skill checklists before it's committed | any builder agent, after finishing a task | `code-review-checklist`, `fintech-security-review` |
| `git-commit-agent` | Turning an approved diff into one or more atomic, well-scoped commits with proper messages | any builder agent, after `code-review-agent` approves | `git-commit-conventions` |

Individual subagent definitions live in `.claude/agents/`. Each file
there is self-contained — description, tools, and system prompt — and
should not be edited without updating this table if scope changes.

**What was deliberately not added:** a dedicated devops/infra agent is
NOT in the roster — deployment is owned by the `deployment-agent`,
because this project is now product-grade (section 7) and needs
whole-stack orchestration, CI/CD, secure key persistence, and Redis/worker
wiring that no individual builder owns.

## 5. Skill roster

Skill definitions live in `.opencode/skills/<name>/SKILL.md` (and
`.claude/skills/<name>/SKILL.md` for the canonical ones). A skill is
a reusable set of patterns and rules, not project-specific code — it
should stay accurate even if the specific mandate schema changes.

| Skill | Covers | Used by |
|---|---|---|
| `rust-mandate-crypto` | Ed25519 signing patterns, canonical JSON, nonce handling | `mandate-crypto-agent` |
| `razorpay-test-mode-api` | Orders/Payments/Refunds API shapes, test UPI IDs, webhook verification | `razorpay-gateway-agent` |
| `acp-feed-schema` | ACP Product Feed field reference and validation rules | `catalog-manifest-agent`, `buyer-agent-dev` |
| `postgres-audit-log-design` | Append-only schema, `SELECT ... FOR UPDATE`, idempotency keys | `reconciliation-agent`, `audit-dashboard-agent` |
| `fintech-security-review` | Secret handling, money-action checklist, what "explainable/bounded/gated" requires in code | `mandate-crypto-agent`, `policy-engine-agent`, `reconciliation-agent`, `qa-integration-agent`, `code-review-agent` |
| `research-lookup-protocol` | How to look up official docs/APIs/crate references and report findings without guessing | `research-agent` |
| `code-review-checklist` | What to check in a diff before approving it — scope, tests, style, rule compliance | `code-review-agent` |
| `git-commit-conventions` | Atomic commit sizing, message format, when to commit vs. when to keep working | `git-commit-agent` |
| `ui-ux-pro-max` | UI/UX design intelligence: design systems, product palettes, typography, accessibility, responsive layout, charts, stack-specific implementation | `product-ux-agent`, `audit-dashboard-agent`, `frontend-designer-agent` (any frontend work) |
| `to-tickets` | Break a plan/spec into tracer-bullet vertical slices with blocking edges, published to the tracker | any builder agent, **only when the user says "tickets" / `/to-tickets`** — not automatic |

## 6. Coding standards

**Rust (`gateway/crates/*`, `gateway/services/*`):**
- `cargo clippy -- -D warnings` must pass before any commit to these
  paths.
- No `.unwrap()` or `.expect()` on any path that handles a mandate,
  payment, or signature — use `Result` and propagate with `?`.
  Panicking on malformed money-related input is not acceptable, even
  in test mode.
- Every public function in `mandate-engine` and `policy-engine` needs
  a doc comment explaining the money-relevant invariant it upholds.

**TypeScript / Next.js (`dashboard/`):**
- Server components by default; client components only where
  interactivity requires it (the consent form, live audit trail
  updates).
- No client-side storage of any mandate or signing material — the
  dashboard only ever displays what the gateway API returns.

**Across the whole repo:**
- Every money-moving code path needs an accompanying audit-log write
  in the same transaction, not a follow-up call.
- Commit messages: `<scope>: <what changed>` — scope matches the
  subagent name minus `-agent` (e.g. `mandate-crypto: add nonce replay
  check`).

## 7. Definition of done, per component

A component is not done until:
1. It matches its owning subagent's scope in section 4.
2. It has at least one test in `tests/integration/` exercising it.
3. Every money-moving action in it produces an audit log entry.
4. It's referenced correctly in `docker-compose.yml` if it's a
   service.
5. No secret or key material is hardcoded or logged.
6. For any user-facing feature: it is reachable end-to-end through
   the real web app (authenticated flow → dashboard), not exercised
   by hand-typed URLs or manual scripts (rule 7 in section 2).
7. `ANCHOR.md` has been updated to reflect the new state (section 0)
   — a component is not "done" if the next session can't resume from
   ANCHOR.md.

## 8. How to invoke a subagent

Example prompts and which subagent should pick them up:

- "Add replay protection to mandate verification" → `mandate-crypto-agent`
- "Write the rule that blocks a purchase over the mandate's max_amount" → `policy-engine-agent`
- "Wire up the Razorpay refund call for the reconciliation flow" → `razorpay-gateway-agent` (API call) + `reconciliation-agent` (trigger logic)
- "Build the consent screen" → `audit-dashboard-agent`
- "Make the dashboard look premium / polish the UI / elevate the frontend" → `frontend-designer-agent` (load `ui-ux-pro-max`, keep the backend contract frozen)
- "Simulate the catalog price drift failure scenario" → `reconciliation-agent`, then `qa-integration-agent` for the test
- "Set up the product feed endpoint" → `catalog-manifest-agent`
- "Build the merchant dashboard / onboarding / design system" → `product-ux-agent` (load `ui-ux-pro-max` for every frontend slice)
- "Wire the whole stack / CI / Redis / key persistence / docker compose" → `deployment-agent`

Client/product concern (not a coding task): when the user asks to break
a plan into tickets — they will say "tickets" or `/to-tickets` — run the
`to-tickets` skill (section 5) to produce tracer-bullet vertical slices
with declared blocking edges, published under `docs/<feature>/`.
Working/reference/ticket files always live under `docs/` (gitignored — never
pushed). Once a ticket's work is complete, delete its file from `docs/` and
record only the outcome in `ANCHOR.md`; the state file is the only thing
committed. This is only ever triggered explicitly by the user; it is never
done automatically.

If a prompt doesn't map cleanly to one row, it's a sign the task is
either too broad (split it) or belongs in `docs/proposal.md` as a
design decision first.

## 9. Escalation protocol — when a subagent is stuck

"Stuck" means: two failed attempts at the same approach, a requirement
that isn't resolved by this file or `docs/proposal.md`, or a need for
current external information (an API's actual behavior, a crate's
current API surface, a spec detail). When that happens, follow this
order — don't silently retry a third time and don't quietly work
around it:

1. **Factual or technical blocker → `research-agent`.** "What does
   this Razorpay error code mean," "what's the current `ed25519-dalek`
   API for this," "does ACP's spec say X or Y" — these are lookups,
   not decisions. `research-agent` has web access, finds the official
   source, and reports back a concise, cited summary — it does not
   write code itself.
2. **Still stuck after research, or the blocker is a scope/design
   decision** (not answerable by looking something up — e.g. "should
   the mandate include a merchant-level daily cap" or "is this edge
   case even in scope") **→ stop and ask Santhosh directly.** Do not
   guess, and do not pick a "reasonable-seeming" default silently. The
   cost of asking is one message; the cost of guessing wrong on a
   money-adjacent decision is a rebuild.
3. **Never bypass a non-negotiable rule (section 2) to get unstuck.**
   If the only way forward seems to require breaking one — e.g. "I
   need to `.unwrap()` here just to get it compiling for now" — that's
   exactly the case for step 2, not a reason to proceed.

## 10. Git workflow and commit rules

These are explicit, not a style suggestion — every subagent finishes
its work through `git-commit-agent`, not by committing on its own.

- **Commit as soon as a small, complete change is done** — don't
  batch up a session's worth of work into one commit at the end. If
  you can describe the change in one sentence without "and," it's
  probably commit-sized.
- **One logical change per commit.** Never bundle unrelated files or
  unrelated concerns into a single commit, even if they happened to be
  edited in the same sitting — `git-commit-agent` should split them
  into separate commits rather than combine them.
- **Pushing is not mandatory on every commit.** Commit locally as
  granularly as the rule above requires; push at natural checkpoints
  (end of a work session, before switching machines, before a demo
  rehearsal) rather than after every single commit.
- **Every commit message is specific, not generic.** Format:
  `<scope>: <what changed>`, scope matching the owning subagent's name
  minus `-agent` (`mandate-crypto: add nonce replay check`,
  `dashboard: add audit trail filter by event type`). Reject messages
  like "fix stuff," "wip," or "update files" — say what actually
  changed and, where it's not obvious, why.
- **`code-review-agent` runs before `git-commit-agent`, always.** A
  diff that hasn't passed review doesn't get committed — this is what
  keeps a bad `.unwrap()` or a missing audit-log write from ever
  landing in history in the first place, rather than being caught
  later.
