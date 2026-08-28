"use client";

import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "next/navigation";
import { getAuditFeed, type AuditFeedEntry } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";

const EVENT_TYPES = [
  "mandate.issue",
  "mandate.revoke",
  "policy.evaluate",
  "payment.attempt",
  "payment.captured",
  "reconcile.check",
  "reconcile.mismatch",
];

function DecisionBadge({ decision }: { decision: string }) {
  const allowed = decision === "allowed";
  const blocked = decision === "blocked";
  const cls = allowed
    ? "bg-accent/15 text-accent"
    : blocked
      ? "bg-destructive/15 text-destructive"
      : "bg-muted-foreground/15 text-muted-foreground";
  return <span className={`rounded-full px-2 py-0.5 text-xs ${cls}`}>{decision}</span>;
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
  const seen = useRef<Set<string>>(new Set());
  const [newKeys, setNewKeys] = useState<Set<string>>(new Set());

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
    }
  }, [scopeId, decision, eventType]);

  useEffect(() => {
    if (!live) return;
    load();
    const t = setInterval(load, 2500);
    return () => clearInterval(t);
  }, [load, live]);

  return (
    <div className="mx-auto max-w-4xl">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">Live audit trail</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Every money-moving action, allowed or blocked, with why.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="flex items-center gap-2 text-xs text-muted-foreground">
            <span
              className={`h-2 w-2 rounded-full ${live ? "animate-pulse bg-accent" : "bg-muted-foreground"}`}
            />
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
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Decision
          <select
            value={decision}
            onChange={(e) => setDecision(e.target.value)}
            className="rounded-lg border border-border bg-background px-2 py-1.5 text-sm text-foreground outline-none focus-visible:border-accent"
          >
            <option value="all">All</option>
            <option value="allowed">Allowed</option>
            <option value="blocked">Blocked</option>
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          Event
          <select
            value={eventType}
            onChange={(e) => setEventType(e.target.value)}
            className="rounded-lg border border-border bg-background px-2 py-1.5 text-sm text-foreground outline-none focus-visible:border-accent"
          >
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

      <div className="overflow-hidden rounded-xl border border-border">
        <div className="grid grid-cols-[7rem_1fr_5rem] gap-2 border-b border-border bg-secondary/40 px-4 py-2 text-xs uppercase tracking-wide text-muted-foreground">
          <span>Time</span>
          <span>Event</span>
          <span>Decision</span>
        </div>
        <ul className="divide-y divide-border">
          {entries.length === 0 && (
            <li className="px-4 py-6 text-sm text-muted-foreground">No events yet.</li>
          )}
          {entries.map((e) => {
            const k = keyOf(e);
            return (
              <li
                key={k}
                className={`grid grid-cols-[7rem_1fr_5rem] gap-2 px-4 py-3 text-sm ${
                  newKeys.has(k) ? "mg-fade-in bg-accent/5" : ""
                }`}
              >
                <span className="text-muted-foreground">
                  {new Date(e.created_at).toLocaleTimeString("en-IN")}
                </span>
                <div>
                  <p className="font-medium text-foreground">{e.event_type}</p>
                  <p className="text-xs text-muted-foreground">
                    {e.actor}
                    {e.reason ? ` · ${e.reason}` : ""}
                  </p>
                </div>
                <DecisionBadge decision={e.decision} />
              </li>
            );
          })}
        </ul>
      </div>
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
