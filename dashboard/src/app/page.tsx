import Link from "next/link";
import { Icon } from "@/components/Icon";

export const dynamic = "force-dynamic";

const NAV = [
  { href: "#how", label: "How it works" },
  { href: "#live", label: "Live agent" },
  { href: "#security", label: "Security" },
  { href: "#features", label: "Product" },
];

function NavLink({ href, label }: { href: string; label: string }) {
  return (
    <a href={href} className="text-sm text-muted-foreground transition-colors hover:text-foreground">
      {label}
    </a>
  );
}

function Btn({
  href,
  children,
  primary = false,
  className = "",
}: {
  href: string;
  children: React.ReactNode;
  primary?: boolean;
  className?: string;
}) {
  return (
    <Link
      href={href}
      className={`inline-flex items-center justify-center gap-2 rounded-xl px-5 py-3 text-sm font-semibold transition-all duration-200 ${
        primary
          ? "bg-accent text-on-accent shadow-[0_0_0_1px_rgba(34,197,94,0.4),0_10px_30px_-12px_rgba(34,197,94,0.7)] hover:opacity-90 hover:shadow-[0_0_0_1px_rgba(34,197,94,0.6),0_14px_40px_-12px_rgba(34,197,94,0.9)]"
          : "border border-border bg-card text-foreground hover:border-accent hover:text-accent"
      } ${className}`}
    >
      {children}
    </Link>
  );
}

function SectionTitle({
  eyebrow,
  title,
  sub,
}: {
  eyebrow: string;
  title: React.ReactNode;
  sub?: string;
}) {
  return (
    <div className="mx-auto max-w-2xl text-center">
      <p className="text-xs font-semibold uppercase tracking-[0.2em] text-accent">{eyebrow}</p>
      <h2 className="mt-3 text-3xl font-semibold leading-tight sm:text-4xl">{title}</h2>
      {sub && <p className="mt-3 text-sm text-muted-foreground sm:text-base">{sub}</p>}
    </div>
  );
}

const STEPS = [
  {
    n: "01",
    icon: "globe",
    title: "Publish an agent-readable catalog",
    body: "Your products become discoverable through a signed merchant manifest. Nothing moves until a mandate exists — agents only ever see what you list.",
  },
  {
    n: "02",
    icon: "shield",
    title: "An agent requests a scoped mandate",
    body: "A buyer agent asks for a capped, expiring permission: max amount, category whitelist, one-time or recurring. You review and it is signed with Ed25519.",
  },
  {
    n: "03",
    icon: "bolt",
    title: "Every purchase is gated + audited",
    body: "The deterministic policy engine allows or blocks each payment. Intent is reconciled against outcome; a mismatch triggers an automatic refund.",
  },
];

const FEATURES = [
  { icon: "key", title: "Scoped API keys for agents", body: "Issue, rotate and revoke per-agent credentials with explicit scopes. Keys never carry money." },
  { icon: "lock", title: "Deterministic policy engine", body: "Budget, category, frequency and expiry are enforced in plain code before any call to Razorpay." },
  { icon: "fingerprint", title: "Replay-protected mandates", body: "Ed25519 signatures with one-time nonces — a mandate can't be reused, forged or replayed." },
  { icon: "refresh", title: "Intent-vs-outcome reconciliation", body: "Catalog drift or overcharge is detected and auto-refunded. One failure, recovered live." },
  { icon: "chart", title: "Live audit + observability", body: "Every allow/block is streamed to an append-only trail merchants and admins watch in real time." },
  { icon: "globe", title: "Razorpay test-mode ready", body: "Drops onto your existing Razorpay merchant account. No new PSP, no new banking rails." },
];

const SAMPLE_MANDATE = `{
  "mandate_id": "mand_8f3c…21a",
  "merchant_id": "merchant-001",
  "buyer_agent_id": "agent-001",
  "max_amount": 50000,
  "currency": "INR",
  "scope": ["electronics"],
  "frequency": "one_time",
  "expires_at": "2026-08-28T18:40:00Z",
  "nonce": "a91f…e3",
  "signature": "ed25519:7b2d…c4"   // verified before spend
}`;

const LIVE_STEPS = [
  { t: "discover", label: "Agent discovers catalog", ok: true },
  { t: "mandate_issued", label: "Requests ₹500 scoped mandate", ok: true },
  { t: "policy.evaluate", label: "Policy: within budget, in scope", ok: true },
  { t: "order_created", label: "Razorpay order #TV9PkL created", ok: true },
  { t: "order_reconciled", label: "Intent matched outcome ✓", ok: true },
];

export default function Landing() {
  return (
    <div className="min-h-screen bg-background text-foreground">
      {/* Nav */}
      <header className="sticky top-0 z-50 border-b border-border/60 bg-background/80 backdrop-blur-xl">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div className="flex items-center gap-2">
            <span className="grid h-9 w-9 place-items-center rounded-xl bg-gradient-to-br from-[#4ade80] to-[#16a34a] text-[#06281a] font-bold shadow-[0_8px_24px_-10px_rgba(34,197,94,0.8)]">
              M
            </span>
            <span className="text-sm font-semibold tracking-tight">Mandate Gateway</span>
          </div>
          <nav className="hidden items-center gap-8 md:flex">
            {NAV.map((n) => (
              <NavLink key={n.href} {...n} />
            ))}
          </nav>
          <Btn href="/login" primary className="px-4 py-2">
            Open dashboard
          </Btn>
        </div>
      </header>

      <main>
        {/* Hero */}
        <section className="relative overflow-hidden">
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0 -z-10 bg-[radial-gradient(60%_50%_at_50%_0%,rgba(34,197,94,0.18),transparent_70%)]"
          />
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0 -z-10 opacity-[0.12] [background-image:linear-gradient(to_right,rgba(148,163,184,0.4)_1px,transparent_1px),linear-gradient(to_bottom,rgba(148,163,184,0.4)_1px,transparent_1px)] [background-size:44px_44px] [mask-image:radial-gradient(60%_60%_at_50%_30%,black,transparent)]"
          />
          <div className="mx-auto grid max-w-6xl items-center gap-12 px-6 py-20 lg:grid-cols-2 lg:py-28">
            <div>
              <span className="inline-flex items-center gap-2 rounded-full border border-accent/40 bg-accent/10 px-3 py-1 text-xs text-accent">
                <Icon name="bolt" className="h-3.5 w-3.5" />
                Agentic commerce · Razorpay test-mode ready
              </span>
              <h1 className="mt-6 text-4xl font-semibold leading-[1.07] tracking-tight sm:text-5xl lg:text-6xl">
                Let AI agents buy from your store —{" "}
                <span className="bg-gradient-to-br from-[#86efac] via-[#22c55e] to-[#15803d] bg-clip-text text-transparent">
                  with a mandate, not the keys
                </span>
                .
              </h1>
              <p className="mt-5 max-w-xl text-base text-muted-foreground sm:text-lg">
                Mandate Gateway is the trust layer between a Razorpay merchant and an external
                AI buyer agent. Every purchase is authorized by a cryptographically scoped
                mandate, decided by a deterministic policy engine, and written to an immutable
                audit trail. One engineered failure — detected and recovered, live.
              </p>
              <div className="mt-8 flex flex-wrap gap-3">
                <Btn href="/login" primary>
                  <Icon name="play" className="h-4 w-4" />
                  Open the live demo
                </Btn>
                <Btn href="/signup">
                  Create merchant account
                  <Icon name="arrow" className="h-4 w-4" />
                </Btn>
              </div>
              <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-2 text-xs text-muted-foreground">
                <span className="inline-flex items-center gap-2"><Icon name="check" className="h-4 w-4 text-accent" /> No LLM makes a money decision</span>
                <span className="inline-flex items-center gap-2"><Icon name="check" className="h-4 w-4 text-accent" /> Ed25519-signed mandates</span>
                <span className="inline-flex items-center gap-2"><Icon name="check" className="h-4 w-4 text-accent" /> Append-only audit</span>
              </div>
            </div>

            {/* Mandate card mock */}
            <div className="relative">
              <div className="absolute -inset-4 -z-10 rounded-3xl bg-[radial-gradient(60%_60%_at_50%_50%,rgba(34,197,94,0.22),transparent)] blur-2xl" aria-hidden />
              <div className="rounded-2xl border border-border bg-card/80 p-5 shadow-2xl backdrop-blur">
                <div className="flex items-center justify-between">
                  <p className="text-sm font-semibold">Active mandate</p>
                  <span className="inline-flex items-center gap-1.5 rounded-full bg-accent/15 px-2.5 py-1 text-xs text-accent">
                    <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-accent" /> signed
                  </span>
                </div>
                <pre className="mt-4 overflow-x-auto rounded-xl bg-background/70 p-4 font-mono text-[11px] leading-relaxed text-muted-foreground">
                  {SAMPLE_MANDATE}
                </pre>
                <div className="mt-4 grid grid-cols-3 gap-3 text-center">
                  <div className="rounded-lg bg-background/60 p-2">
                    <p className="text-xs text-muted-foreground">Cap</p>
                    <p className="text-sm font-semibold text-foreground">₹500</p>
                  </div>
                  <div className="rounded-lg bg-background/60 p-2">
                    <p className="text-xs text-muted-foreground">Scope</p>
                    <p className="text-sm font-semibold text-foreground">electronics</p>
                  </div>
                  <div className="rounded-lg bg-background/60 p-2">
                    <p className="text-xs text-muted-foreground">Policy</p>
                    <p className="text-sm font-semibold text-accent">allowed</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Trust strip */}
        <section className="border-y border-border/60 bg-primary/30">
          <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-center gap-x-10 gap-y-4 px-6 py-6 text-xs text-muted-foreground">
            <span className="font-semibold uppercase tracking-widest text-foreground/70">Built on</span>
            <span className="inline-flex items-center gap-2"><Icon name="fingerprint" className="h-4 w-4 text-accent" /> Ed25519 signatures</span>
            <span className="inline-flex items-center gap-2"><Icon name="lock" className="h-4 w-4 text-accent" /> Deterministic policy</span>
            <span className="inline-flex items-center gap-2"><Icon name="list" className="h-4 w-4 text-accent" /> Append-only audit</span>
            <span className="inline-flex items-center gap-2"><Icon name="refresh" className="h-4 w-4 text-accent" /> Auto-reconcile</span>
          </div>
        </section>

        {/* How it works */}
        <section id="how" className="mx-auto max-w-6xl scroll-mt-20 px-6 py-20">
          <SectionTitle
            eyebrow="How it works"
            title={<>Three steps. Zero trust placed in the model.</>}
            sub="The agent picks what to buy. The mandate, the policy engine and the reconciliation layer decide — never the LLM."
          />
          <div className="relative mt-12 grid gap-6 lg:grid-cols-3">
            {STEPS.map((s, i) => (
              <div key={s.n} className="relative rounded-2xl border border-border bg-card p-6">
                <div className="flex items-center justify-between">
                  <span className="grid h-11 w-11 place-items-center rounded-xl bg-accent/10 text-accent">
                    <Icon name={s.icon} className="h-5 w-5" />
                  </span>
                  <span className="text-sm font-semibold text-accent">{s.n}</span>
                </div>
                <h3 className="mt-4 text-lg font-semibold">{s.title}</h3>
                <p className="mt-2 text-sm text-muted-foreground">{s.body}</p>
                {i < STEPS.length - 1 && (
                  <Icon name="arrow" className="absolute -right-6 top-1/2 hidden h-5 w-5 -translate-y-1/2 text-border lg:block" />
                )}
              </div>
            ))}
          </div>
        </section>

        {/* Live agent showcase */}
        <section id="live" className="scroll-mt-20 border-y border-border/60 bg-primary/30">
          <div className="mx-auto grid max-w-6xl items-center gap-12 px-6 py-20 lg:grid-cols-2">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-accent">Live agent</p>
              <h2 className="mt-3 text-3xl font-semibold leading-tight sm:text-4xl">
                Watch a buyer agent spend — and stay inside its mandate
              </h2>
              <p className="mt-4 text-sm text-muted-foreground sm:text-base">
                In this deployment a buyer agent discovers the catalog, requests a mandate and
                purchases on a loop. Each step is signed, policy-checked and streamed to the
                audit trail. Open the demo to watch it happen in real time.
              </p>
              <div className="mt-6 flex flex-wrap gap-3">
                <Btn href="/login" primary>
                  See the live dashboard
                  <Icon name="arrow" className="h-4 w-4" />
                </Btn>
                <Btn href="/reconciliation" className="hidden lg:inline-flex">
                  View reconciliation
                </Btn>
              </div>
            </div>

            <div className="rounded-2xl border border-border bg-card/80 p-5 shadow-2xl backdrop-blur">
              <div className="flex items-center justify-between">
                <p className="text-sm font-semibold">Buyer agent · run #4821</p>
                <span className="inline-flex items-center gap-1.5 rounded-full bg-accent/15 px-2.5 py-1 text-xs text-accent">
                  <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-accent" /> LIVE
                </span>
              </div>
              <ul className="mt-4 divide-y divide-border">
                {LIVE_STEPS.map((s) => (
                  <li key={s.t} className="flex items-center justify-between py-3">
                    <div className="flex items-center gap-3">
                      <span className="grid h-7 w-7 place-items-center rounded-lg bg-accent/10 text-accent">
                        <Icon name="check" className="h-4 w-4" />
                      </span>
                      <div>
                        <p className="font-mono text-sm text-foreground">{s.t}</p>
                        <p className="text-xs text-muted-foreground">{s.label}</p>
                      </div>
                    </div>
                    <span className="text-xs text-accent">ok</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </section>

        {/* Features */}
        <section id="features" className="mx-auto max-w-6xl scroll-mt-20 px-6 py-20">
          <SectionTitle
            eyebrow="Product"
            title={<>Everything money-adjacent, engineered to be explainable</>}
            sub="Plain deterministic code you can audit line by line — not a black box with your balance sheet inside."
          />
          <div className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {FEATURES.map((f) => (
              <div key={f.title} className="group rounded-2xl border border-border bg-card p-6 transition-colors hover:border-accent/60">
                <span className="grid h-11 w-11 place-items-center rounded-xl bg-accent/10 text-accent transition-transform group-hover:scale-105">
                  <Icon name={f.icon} className="h-5 w-5" />
                </span>
                <h3 className="mt-4 font-semibold">{f.title}</h3>
                <p className="mt-2 text-sm text-muted-foreground">{f.body}</p>
              </div>
            ))}
          </div>
        </section>

        {/* Security / sample mandate */}
        <section id="security" className="scroll-mt-20 border-y border-border/60 bg-primary/30">
          <div className="mx-auto grid max-w-6xl items-center gap-12 px-6 py-20 lg:grid-cols-2">
            <div className="rounded-2xl border border-border bg-card/80 p-5 shadow-2xl backdrop-blur">
              <p className="text-sm font-semibold">What a mandate actually carries</p>
              <pre className="mt-4 overflow-x-auto rounded-xl bg-background/70 p-4 font-mono text-[11px] leading-relaxed text-muted-foreground">
                {SAMPLE_MANDATE}
              </pre>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-accent">Security by design</p>
              <h2 className="mt-3 text-3xl font-semibold leading-tight sm:text-4xl">
                No model ever touches the final decision
              </h2>
              <ul className="mt-6 space-y-4">
                {[
                  ["lock", "LLMs parse intent only", "Natural-language selection is the agent's job. The allow/block call is plain code, gated by the signed mandate."],
                  ["list", "Every action is audited first", "An append-only trail records who, what, under which mandate, allowed or blocked, and why — before completion."],
                  ["key", "Keys never carry money", "Agents authenticate with scoped API keys. Revoking a key stops a buyer agent instantly, mid-loop."],
                ].map(([icon, title, body]) => (
                  <li key={title} className="flex gap-3">
                    <span className="mt-0.5 grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-accent/10 text-accent">
                      <Icon name={icon} className="h-4 w-4" />
                    </span>
                    <div>
                      <p className="font-medium">{title}</p>
                      <p className="text-sm text-muted-foreground">{body}</p>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </section>

        {/* Final CTA */}
        <section className="mx-auto max-w-6xl px-6 py-20">
          <div className="relative overflow-hidden rounded-3xl border border-accent/40 bg-gradient-to-br from-[#0b3d24] via-[#08311d] to-[#062013] p-10 text-center sm:p-14">
            <div aria-hidden className="pointer-events-none absolute inset-0 bg-[radial-gradient(50%_60%_at_50%_0%,rgba(34,197,94,0.35),transparent)]" />
            <h2 className="relative text-3xl font-semibold leading-tight sm:text-4xl">
              Make your store safe for AI buyers — today
            </h2>
            <p className="relative mx-auto mt-3 max-w-xl text-sm text-muted-foreground sm:text-base">
              Open the live demo to watch a buyer agent spend inside a mandate, see the audit
              trail, and watch a drifted price get refunded automatically.
            </p>
            <div className="relative mt-8 flex flex-wrap justify-center gap-3">
              <Btn href="/login" primary>
                <Icon name="play" className="h-4 w-4" />
                Open the live demo
              </Btn>
              <Btn href="/signup" className="border-white/20 bg-white/5 text-foreground hover:border-accent hover:text-accent">
                Create merchant account
              </Btn>
            </div>
          </div>
        </section>
      </main>

      <footer className="border-t border-border">
        <div className="mx-auto flex max-w-6xl flex-col items-center justify-between gap-2 px-6 py-8 text-xs text-muted-foreground sm:flex-row">
          <p className="flex items-center gap-2">
            <span className="grid h-6 w-6 place-items-center rounded-md bg-accent text-on-accent text-[11px] font-bold">M</span>
            Mandate Gateway — agentic-commerce trust layer for Razorpay merchants.
          </p>
          <p>Deterministic. Auditable. Recoverable.</p>
        </div>
      </footer>
    </div>
  );
}
