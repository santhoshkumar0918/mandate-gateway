"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import {
  fetchCatalog,
  getAuditFeed,
  getMandates,
  type AuditFeedEntry,
  type MandateSummary,
  type Product,
} from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Icon } from "@/components/Icon";

const PURCHASE_EVENTS = new Set(["order_created", "purchase_attempt", "budget_debited"]);
const AGENT_EVENTS = new Set([
  "mandate_issued",
  "purchase_attempt",
  "order_created",
  "budget_debited",
  "order_reconciled",
]);

function Kpi({
  label,
  value,
  sub,
  accent,
}: {
  label: string;
  value: string;
  sub?: string;
  accent?: boolean;
}) {
  return (
    <div className="rounded-xl border border-border bg-card p-5">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
      <p className={`mt-2 text-2xl font-semibold ${accent ? "text-accent" : "text-foreground"}`}>
        {value}
      </p>
      {sub && <p className="mt-1 text-xs text-muted-foreground">{sub}</p>}
    </div>
  );
}

function ActivityRow({ e }: { e: AuditFeedEntry }) {
  return (
    <li className="flex items-center justify-between border-b border-border py-3 last:border-0">
      <div className="flex items-center gap-3">
        <span className="grid h-8 w-8 place-items-center rounded-lg bg-accent/10 text-accent">
          <Icon name="bolt" className="h-4 w-4" />
        </span>
        <div>
          <p className="font-mono text-sm text-foreground">{e.event_type}</p>
          <p className="text-xs text-muted-foreground">{e.actor}{e.reason ? ` · ${e.reason}` : ""}</p>
        </div>
      </div>
      <span className="text-xs text-muted-foreground">
        {new Date(e.created_at).toLocaleTimeString("en-IN")}
      </span>
    </li>
  );
}

export default function DashboardHome() {
  const [catalog, setCatalog] = useState<Product[]>([]);
  const [mandates, setMandates] = useState<MandateSummary[]>([]);
  const [activity, setActivity] = useState<AuditFeedEntry[]>([]);
  const [currency, setCurrency] = useState("INR");
  const seen = useRef<Set<string>>(new Set());
  const [hasData, setHasData] = useState(true);

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      const cat = await fetchCatalog(token);
      setCatalog(cat);
      if (cat[0]) setCurrency(cat[0].currency);
    } catch {
      /* ignore */
    }
    try {
      setMandates(await getMandates(token));
    } catch {
      /* ignore */
    }
    try {
      const feed = await getAuditFeed(token, { limit: 100 });
      feed.forEach((e) => seen.current.add(`${e.created_at}|${e.event_type}|${e.actor}`));
      const agent = feed.filter((e) => AGENT_EVENTS.has(e.event_type) || (e.actor && e.actor.startsWith("agent")));
      setActivity(agent.slice(0, 12));
      setHasData(agent.some((e) => PURCHASE_EVENTS.has(e.event_type)));
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    load();
    const t = setInterval(load, 4000);
    return () => clearInterval(t);
  }, [load]);

  const catalogValue = catalog.reduce((s, p) => s + p.price, 0);
  const activeMandates = mandates.filter((m) => m.status === "active").length;
  const purchasesToday = activity.filter(
    (e) => PURCHASE_EVENTS.has(e.event_type) && new Date(e.created_at).toDateString() === new Date().toDateString(),
  ).length;
  const blocked = activity.filter((e) => e.decision === "blocked").length;

  const fmt = (p: number) =>
    (p / 100).toLocaleString("en-IN", { style: "currency", currency, maximumFractionDigits: 0 });

  return (
    <div className="mx-auto max-w-5xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Overview</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Your store, as AI buyer agents see it — and everything they&apos;ve done.
        </p>
      </div>

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
        <Kpi label="Live products" value={String(catalog.length)} sub="agent-discoverable" />
        <Kpi label="Active mandates" value={String(activeMandates)} accent sub="granted to agents" />
        <Kpi label="Purchases (24h)" value={String(purchasesToday)} sub="captured via gateway" />
        <Kpi label="Blocked" value={String(blocked)} sub="by policy engine" />
        <Kpi label="Catalog value" value={fmt(catalogValue)} sub="total listed" />
      </div>

      <div className="mt-8 grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
              Live agent activity
            </h2>
            <Link href="/audit" className="text-xs text-accent hover:underline">
              Full audit trail →
            </Link>
          </div>
          <div className="rounded-xl border border-border bg-card px-4">
            {activity.length === 0 ? (
              <p className="py-8 text-sm text-muted-foreground">No agent activity yet.</p>
            ) : (
              <ul>
                {activity.map((e) => (
                  <ActivityRow key={`${e.created_at}|${e.event_type}|${e.actor}`} e={e} />
                ))}
              </ul>
            )}
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <div className="rounded-xl border border-border bg-card p-5">
            <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
              Quick actions
            </h2>
            <div className="mt-3 flex flex-col gap-2">
              <Link
                href="/catalog"
                className="flex items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm transition-colors hover:border-accent hover:text-accent"
              >
                <Icon name="tag" className="h-4 w-4" /> Manage catalog
              </Link>
              <Link
                href="/agents"
                className="flex items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm transition-colors hover:border-accent hover:text-accent"
              >
                <Icon name="bot" className="h-4 w-4" /> Issue agent key
              </Link>
              <Link
                href="/reconciliation"
                className="flex items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm transition-colors hover:border-accent hover:text-accent"
              >
                <Icon name="alert" className="h-4 w-4" /> Reconciliation
              </Link>
            </div>
          </div>

          {!hasData && (
            <div className="rounded-xl border border-accent/40 bg-accent/10 p-5">
              <p className="font-medium text-accent">Your store is live</p>
              <p className="mt-1 text-sm text-muted-foreground">
                No agent has acted yet. The bundled demo merchant already runs a buyer agent on
                a loop — sign in with the demo credentials on the login page to watch it, or
                issue a scoped key to connect your own agent.
              </p>
              <Link
                href="/agents"
                className="mt-3 inline-flex items-center gap-1 text-sm text-accent hover:underline"
              >
                Connect an agent <Icon name="arrow" className="h-4 w-4" />
              </Link>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
