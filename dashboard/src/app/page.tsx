import Link from "next/link";

export const dynamic = "force-dynamic";

function Pill({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex items-center gap-2 rounded-full border border-border bg-card px-3 py-1 text-xs text-muted-foreground">
      {children}
    </span>
  );
}

function Step({
  n,
  title,
  body,
}: {
  n: string;
  title: string;
  body: string;
}) {
  return (
    <div className="rounded-2xl border border-border bg-card p-6">
      <p className="text-sm font-semibold text-accent">{n}</p>
      <h3 className="mt-2 text-lg font-semibold text-foreground">{title}</h3>
      <p className="mt-2 text-sm text-muted-foreground">{body}</p>
    </div>
  );
}

function Feature({
  title,
  body,
}: {
  title: string;
  body: string;
}) {
  return (
    <li className="flex gap-3">
      <span className="mt-1 text-accent" aria-hidden>
        ✓
      </span>
      <div>
        <p className="font-medium text-foreground">{title}</p>
        <p className="text-sm text-muted-foreground">{body}</p>
      </div>
    </li>
  );
}

export default function Landing() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="mx-auto flex max-w-6xl items-center justify-between px-6 py-5">
        <div className="flex items-center gap-2">
          <span className="grid h-8 w-8 place-items-center rounded-lg bg-accent text-on-accent font-bold">
            M
          </span>
          <span className="text-sm font-semibold">Mandate Gateway</span>
        </div>
        <Link
          href="/login"
          className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-on-accent transition-opacity hover:opacity-90"
        >
          Open Merchant Dashboard
        </Link>
      </header>

      <main className="mx-auto max-w-6xl px-6">
        <section className="relative py-20 text-center">
          <div
            aria-hidden
            className="pointer-events-none absolute inset-x-0 top-0 -z-10 mx-auto h-64 max-w-3xl rounded-full bg-accent/10 blur-3xl"
          />
          <div className="flex justify-center gap-2">
            <Pill>Ed25519 signed mandates</Pill>
            <Pill>Razorpay test-mode ready</Pill>
          </div>
          <h1 className="mx-auto mt-6 max-w-3xl text-4xl font-semibold leading-tight sm:text-5xl">
            Let AI buyer agents shop your store —{" "}
            <span className="text-accent">without handing them the keys</span>.
          </h1>
          <p className="mx-auto mt-5 max-w-2xl text-base text-muted-foreground">
            Mandate Gateway is the trust layer between a Razorpay merchant and an
            external AI agent. Every purchase is authorized by a cryptographically
            scoped mandate, decided by a deterministic policy engine, and written
            to an immutable audit trail. One engineered failure, detected and
            recovered — live.
          </p>
          <div className="mt-8 flex justify-center gap-3">
            <Link
              href="/login"
              className="rounded-lg bg-accent px-5 py-3 text-sm font-medium text-on-accent transition-opacity hover:opacity-90"
            >
              Open Merchant Dashboard
            </Link>
            <Link
              href="/signup"
              className="rounded-lg border border-border px-5 py-3 text-sm font-medium text-foreground transition-colors hover:bg-secondary"
            >
              Create merchant account
            </Link>
          </div>
        </section>

        <section className="grid grid-cols-1 gap-4 pb-10 sm:grid-cols-3">
          <Step
            n="01"
            title="Publish your catalog"
            body="Your products become agent-discoverable through a signed merchant manifest. Nothing moves until a mandate exists."
          />
          <Step
            n="02"
            title="Agent requests a mandate"
            body="A buyer agent asks for a scoped, expiring permission — capped amount, category whitelist, single or recurring."
          />
          <Step
            n="03"
            title="Every purchase is gated + audited"
            body="The policy engine allows or blocks each payment. Intent is reconciled against outcome; mismatches trigger refunds."
          />
        </section>

        <section className="grid grid-cols-1 gap-10 py-12 md:grid-cols-2">
          <div>
            <h2 className="text-2xl font-semibold">Built so money never moves on trust</h2>
            <p className="mt-3 text-sm text-muted-foreground">
              No LLM ever makes a final money decision. The mandate signature, the
              policy check, and the reconciliation engine are plain deterministic
              code — auditable line by line.
            </p>
            <ul className="mt-6 space-y-4">
              <Feature
                title="Scoped API keys for agents"
                body="Issue, rotate, and revoke per-agent credentials with explicit scopes."
              />
              <Feature
                title="Deterministic policy engine"
                body="Budget, category, frequency, and expiry enforced before any call to Razorpay."
              />
              <Feature
                title="Replay-protected mandates"
                body="Ed25519 signatures with one-time nonces — a mandate can't be reused or forged."
              />
              <Feature
                title="Intent-vs-outcome reconciliation"
                body="Catalog drift or overcharge is detected and auto-refunded."
              />
              <Feature
                title="Live audit + observability"
                body="Every allow/block is streamed; merchants and admins watch in real time."
              />
            </ul>
          </div>

          <div className="rounded-2xl border border-border bg-card p-6">
            <p className="text-sm font-semibold text-foreground">A continuous agent is already running</p>
            <p className="mt-2 text-sm text-muted-foreground">
              In this deployment a buyer agent discovers the catalog, requests a
              mandate, and purchases on a loop — each step signed and audited. The
              merchant dashboard shows live agent activity; the reconciliation view
              shows any mismatch and its refund.
            </p>
            <dl className="mt-6 space-y-3 text-sm">
              <div className="flex justify-between border-b border-border pb-2">
                <dt className="text-muted-foreground">Auth</dt>
                <dd className="text-foreground">Ed25519 + JWT</dd>
              </div>
              <div className="flex justify-between border-b border-border pb-2">
                <dt className="text-muted-foreground">Policy</dt>
                <dd className="text-foreground">deterministic, no LLM</dd>
              </div>
              <div className="flex justify-between border-b border-border pb-2">
                <dt className="text-muted-foreground">Payments</dt>
                <dd className="text-foreground">Razorpay (test mode)</dd>
              </div>
              <div className="flex justify-between">
                <dt className="text-muted-foreground">Audit</dt>
                <dd className="text-foreground">append-only, streamed</dd>
              </div>
            </dl>
            <Link
              href="/login"
              className="mt-6 inline-block rounded-lg border border-border px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-secondary"
            >
              See the live dashboard →
            </Link>
          </div>
        </section>
      </main>

      <footer className="border-t border-border">
        <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-2 px-6 py-6 text-xs text-muted-foreground sm:flex-row">
          <p>Mandate Gateway — agentic-commerce trust layer for Razorpay merchants.</p>
          <p>Deterministic. Auditable. Recoverable.</p>
        </div>
      </footer>
    </div>
  );
}
