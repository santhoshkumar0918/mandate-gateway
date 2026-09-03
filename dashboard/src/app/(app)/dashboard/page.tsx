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
import { Badge, PageHeader, Skeleton, StatCard } from "@/components/ui";

const PURCHASE_EVENTS = new Set(["order_created", "purchase_attempt", "budget_debited"]);
const AGENT_EVENTS = new Set([
  "mandate_issued",
  "purchase_attempt",
  "order_created",
  "budget_debited",
  "order_reconciled",
]);

function ActivityRow({ e, fresh }: { e: AuditFeedEntry; fresh?: boolean }) {
  return (
    <li
      className={`flex items-center justify-between gap-3 border-b border-border py-3 last:border-0 ${
        fresh ? "mg-fade-in" : ""
      }`}
    >
      <div className="flex min-w-0 items-center gap-3">
        <span className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-accent/10 text-accent">
          <Icon name="bolt" className="h-4 w-4" />
        </span>
        <div className="min-w-0">
          <p className="font-mono text-sm text-foreground">{e.event_type}</p>
          <p className="truncate text-xs text-muted-foreground">
            {e.actor}
            {e.reason ? ` · ${e.reason}` : ""}
          </p>
        </div>
      </div>
      <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
        {new Date(e.created_at).toLocaleTimeString("en-IN")}
      </span>
    </li>
  );
}

function ActivitySkeleton() {
  return (
    <div className="flex flex-col gap-3 px-4 py-4">
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className="flex items-center gap-3">
          <Skeleton className="h-9 w-9 rounded-lg" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-3 w-40" />
            <Skeleton className="h-2.5 w-56" />
          </div>
        </div>
      ))}
    </div>
  );
}

export default function DashboardHome() {
  const [catalog, setCatalog] = useState<Product[]>([]);
  const [mandates, setMandates] = useState<MandateSummary[]>([]);
  const [activity, setActivity] = useState<AuditFeedEntry[]>([]);
  const [fresh, setFresh] = useState<Set<string>>(new Set());
  const [currency, setCurrency] = useState("INR");
  const seen = useRef<Set<string>>(new Set());
  const [hasData, setHasData] = useState(true);
  const [loading, setLoading] = useState(true);

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
      const newOnes = feed.filter((e) => !seen.current.has(`${e.created_at}|${e.event_type}|${e.actor}`));
      if (newOnes.length) {
        const keys = new Set(newOnes.map((e) => `${e.created_at}|${e.event_type}|${e.actor}`));
        setFresh(keys);
        setTimeout(() => setFresh(new Set()), 1400);
        newOnes.forEach((e) => seen.current.add(`${e.created_at}|${e.event_type}|${e.actor}`));
      }
      const agent = feed.filter((e) => AGENT_EVENTS.has(e.event_type) || (e.actor && e.actor.startsWith("agent")));
      setActivity(agent.slice(0, 12));
      setHasData(agent.some((e) => PURCHASE_EVENTS.has(e.event_type)));
    } catch {
      /* ignore */
    } finally {
      setLoading(false);
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
    <div className="mx-auto max-w-6xl">
      <PageHeader
        title="Overview"
        description="Your store, as AI buyer agents see it — and everything they've done, in real time."
        icon={<Icon name="grid" className="h-5 w-5" />}
      />

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
        {loading ? (
          Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="card p-5">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="mt-3 h-8 w-16" />
            </div>
          ))
        ) : (
          <>
            <StatCard index={0} label="Live products" value={String(catalog.length)} sub="agent-discoverable" icon={<Icon name="tag" className="h-4 w-4" />} />
            <StatCard index={1} label="Active mandates" value={String(activeMandates)} accent sub="granted to agents" icon={<Icon name="shield" className="h-4 w-4" />} />
            <StatCard index={2} label="Purchases (24h)" value={String(purchasesToday)} sub="captured via gateway" icon={<Icon name="package" className="h-4 w-4" />} />
            <StatCard index={3} label="Blocked" value={String(blocked)} sub="by policy engine" icon={<Icon name="lock" className="h-4 w-4" />} />
            <StatCard index={4} label="Catalog value" value={fmt(catalogValue)} sub="total listed" icon={<Icon name="chart" className="h-4 w-4" />} />
          </>
        )}
      </div>

      <div className="mt-10 grid grid-cols-1 gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2.5">
              <span className="live-dot h-2.5 w-2.5 rounded-full bg-accent" />
              <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                Live agent activity
              </h2>
            </div>
            <Link href="/audit" className="text-xs font-medium text-accent transition-colors hover:text-accent/80">
              Full audit trail →
            </Link>
          </div>
          <div className="card mg-stagger-3 overflow-hidden px-4">
            {loading ? (
              <ActivitySkeleton />
            ) : activity.length === 0 ? (
              <p className="py-10 text-center text-sm text-muted-foreground">No agent activity yet.</p>
            ) : (
              <ul>
                {activity.map((e) => (
                  <ActivityRow
                    key={`${e.created_at}|${e.event_type}|${e.actor}`}
                    e={e}
                    fresh={fresh.has(`${e.created_at}|${e.event_type}|${e.actor}`)}
                  />
                ))}
              </ul>
            )}
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <div className="card mg-stagger-4 p-5">
            <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
              Quick actions
            </h2>
            <div className="mt-4 flex flex-col gap-2.5">
              {[
                { href: "/catalog", icon: "tag", label: "Manage catalog" },
                { href: "/agents", icon: "bot", label: "Issue agent key" },
                { href: "/reconciliation", icon: "alert", label: "Reconciliation" },
              ].map((a) => (
                <Link
                  key={a.href}
                  href={a.href}
                  className="card-hover group flex items-center gap-2.5 rounded-xl border border-border px-3.5 py-2.5 text-sm text-foreground/90 transition-colors hover:border-accent/40"
                >
                  <Icon name={a.icon} className="h-4 w-4 text-accent" />
                  {a.label}
                  <Icon name="arrow" className="ml-auto h-4 w-4 text-muted-foreground/50 transition-transform group-hover:translate-x-0.5" />
                </Link>
              ))}
            </div>
          </div>

          {!hasData && !loading && (
            <div className="card mg-stagger-5 border-accent/30 bg-accent/[0.07] p-5">
              <div className="flex items-center gap-2">
                <Icon name="sparkles" className="h-4 w-4 text-accent" />
                <p className="font-medium text-accent">Your store is live</p>
              </div>
              <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                No agent has acted yet. The bundled demo merchant already runs a buyer agent on a
                loop — sign in with the demo credentials to watch it, or issue a scoped key to
                connect your own agent.
              </p>
              <Link href="/agents" className="mt-3 inline-flex items-center gap-1 text-sm font-medium text-accent hover:underline">
                Connect an agent <Icon name="arrow" className="h-4 w-4" />
              </Link>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
