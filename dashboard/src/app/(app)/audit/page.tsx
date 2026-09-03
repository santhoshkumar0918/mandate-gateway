"use client";

import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "next/navigation";
import { getAuditFeed, type AuditFeedEntry } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";
import { Badge, PageHeader, Skeleton } from "@/components/ui";
import { Icon } from "@/components/Icon";

const EVENT_TYPES = [
  "mandate_issued",
  "purchase_attempt",
  "order_created",
  "budget_debited",
  "order_reconciled",
];

function DecisionBadge({ decision }: { decision: string }) {
  const allowed = decision === "allowed";
  const blocked = decision === "blocked";
  const tone = allowed ? "accent" : blocked ? "danger" : "muted";
  return <Badge tone={tone}>{decision}</Badge>;
}

function keyOf(e: AuditFeedEntry) {
  return `${e.created_at}|${e.event_type}|${e.actor}|${e.entity_id}`;
}

function Feed({ scopeId }: { scopeId: string | null }) {
  const [entries, setEntries] = useState<AuditFeedEntry[]>([]);
  const [decision, setDecision] = useState("all");
  const [eventType, setEventType] = useState("all");
  const [live, setLive] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const seen = useRef<Set<string>>(new Set());
  const [newKeys, setNewKeys] = useState<Set<string>>(new Set());
  const [page, setPage] = useState(1);
  const PAGE = 15;

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      const feed = await getAuditFeed(token, {
        mandate_id: scopeId ?? undefined,
        decision: decision === "all" ? undefined : decision,
        event_type: eventType === "all" ? undefined : eventType,
        limit: 100,
      });
      const fresh = feed.filter((e) => !seen.current.has(keyOf(e)));
      if (fresh.length) {
        const added = new Set(fresh.map(keyOf));
        setNewKeys(added);
        setTimeout(() => setNewKeys(new Set()), 1200);
        fresh.forEach((e) => seen.current.add(keyOf(e)));
      }
      setEntries(feed);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Feed error");
    } finally {
      setLoading(false);
    }
  }, [scopeId, decision, eventType]);

  useEffect(() => {
    if (!live) return;
    load();
    const t = setInterval(load, 2500);
    return () => clearInterval(t);
  }, [load, live]);

  const selectCls =
    "rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground outline-none transition-colors focus-visible:border-accent";

  return (
    <div className="mx-auto max-w-4xl">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <PageHeader
          title="Live audit trail"
          description="Every money-moving action, allowed or blocked, with why."
          icon={<Icon name="list" className="h-5 w-5" />}
        />
        <div className="mb-8 flex items-center gap-2.5">
          <span className="flex items-center gap-2 rounded-full border border-border bg-card px-3 py-1.5 text-xs font-medium text-muted-foreground">
            <span className={`live-dot h-2 w-2 rounded-full ${live ? "bg-accent" : "bg-muted-foreground"}`} />
            {live ? "LIVE" : "paused"}
          </span>
          <button
            onClick={() => setLive((v) => !v)}
            className="rounded-lg border border-border px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-secondary"
          >
            {live ? "Pause" : "Resume"}
          </button>
        </div>
      </div>

      <div className="mb-4 flex flex-wrap gap-3">
        <label className="flex flex-col gap-1.5 text-xs text-muted-foreground">
          Decision
          <select value={decision} onChange={(e) => setDecision(e.target.value)} className={selectCls}>
            <option value="all">All</option>
            <option value="allowed">Allowed</option>
            <option value="blocked">Blocked</option>
          </select>
        </label>
        <label className="flex flex-col gap-1.5 text-xs text-muted-foreground">
          Event
          <select value={eventType} onChange={(e) => setEventType(e.target.value)} className={selectCls}>
            <option value="all">All</option>
            {EVENT_TYPES.map((et) => (
              <option key={et} value={et}>
                {et}
              </option>
            ))}
          </select>
        </label>
      </div>

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <div className="flex flex-col gap-2.5 overflow-hidden rounded-xl border border-border p-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <Skeleton key={i} className="h-10 rounded-lg" />
          ))}
        </div>
      ) : (
        <div className="card overflow-hidden">
          <div className="grid grid-cols-[7rem_1fr_6rem] gap-3 border-b border-border bg-secondary/40 px-4 py-2.5 text-xs uppercase tracking-wide text-muted-foreground">
            <span>Time</span>
            <span>Event</span>
            <span className="text-right">Decision</span>
          </div>
          <ul className="divide-y divide-border">
            {entries.length === 0 && (
              <li className="px-4 py-10 text-center text-sm text-muted-foreground">No events yet.</li>
            )}
            {entries.slice((page - 1) * PAGE, page * PAGE).map((e) => {
              const k = keyOf(e);
              return (
                <li
                  key={k}
                  className={`grid grid-cols-[7rem_1fr_6rem] items-center gap-3 px-4 py-3 text-sm ${
                    newKeys.has(k) ? "mg-fade-in bg-accent/5" : ""
                  }`}
                >
                  <span className="text-xs tabular-nums text-muted-foreground">
                    {new Date(e.created_at).toLocaleTimeString("en-IN")}
                  </span>
                  <div className="min-w-0">
                    <p className="font-medium text-foreground">{e.event_type}</p>
                    <p className="truncate text-xs text-muted-foreground">
                      {e.actor}
                      {e.reason ? ` · ${e.reason}` : ""}
                    </p>
                  </div>
                  <div className="text-right">
                    <DecisionBadge decision={e.decision} />
                  </div>
                </li>
              );
            })}
          </ul>
        </div>
      )}
      {entries.length > PAGE && (
        <Pagination page={page} pageSize={PAGE} total={entries.length} onPage={setPage} />
      )}
    </div>
  );
}

export default function AuditPage() {
  return (
    <Suspense fallback={<div className="text-sm text-muted-foreground">Loading…</div>}>
      <FeedInner />
    </Suspense>
  );
}

function FeedInner() {
  const params = useSearchParams();
  const scopeId = params.get("mandate_id");
  return <Feed scopeId={scopeId} />;
}
