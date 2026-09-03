"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  createKey,
  getAuditFeed,
  listKeys,
  revokeKey,
  type ApiKeyInfo,
  type AuditFeedEntry,
} from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";
import { Badge, Card, PageHeader, Skeleton, StatCard } from "@/components/ui";
import { Icon } from "@/components/Icon";

const AGENT_EVENTS = new Set([
  "mandate_issued",
  "purchase_attempt",
  "order_created",
  "budget_debited",
  "order_reconciled",
]);

export default function AgentConsolePage() {
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [activity, setActivity] = useState<AuditFeedEntry[]>([]);
  const [newKeys, setNewKeys] = useState<Set<string>>(new Set());
  const [label, setLabel] = useState("buyer-agent");
  const [rawKey, setRawKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loadingKeys, setLoadingKeys] = useState(true);
  const seen = useRef<Set<string>>(new Set());
  const [page, setPage] = useState(1);
  const PAGE = 15;

  const loadKeys = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setKeys(await listKeys(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load keys");
    } finally {
      setLoadingKeys(false);
    }
  }, []);

  const loadActivity = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      const feed = await getAuditFeed(token, { limit: 100 });
      const agent = feed.filter(
        (e) => AGENT_EVENTS.has(e.event_type) || (e.actor && e.actor.startsWith("agent")),
      );
      const fresh = agent.filter(
        (e) => !seen.current.has(`${e.created_at}|${e.event_type}|${e.actor}`),
      );
      if (fresh.length) {
        const ids = new Set(fresh.map((e) => `${e.created_at}|${e.event_type}|${e.actor}`));
        setNewKeys(ids);
        setTimeout(() => setNewKeys(new Set()), 1200);
        fresh.forEach((e) => seen.current.add(`${e.created_at}|${e.event_type}|${e.actor}`));
      }
      setActivity(agent);
    } catch {
      /* non-fatal */
    }
  }, []);

  useEffect(() => {
    loadKeys();
    loadActivity();
    const t = setInterval(loadActivity, 3000);
    return () => clearInterval(t);
  }, [loadKeys, loadActivity]);

  async function onCreate() {
    setError(null);
    const token = getToken();
    if (!token) return;
    try {
      const res = await createKey(token, label || "buyer-agent", [
        "mandate:issue",
        "purchase:exec",
      ]);
      setRawKey(res.key);
      await loadKeys();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create key");
    }
  }

  async function onRevoke(id: string) {
    const token = getToken();
    if (!token) return;
    try {
      await revokeKey(token, id);
      await loadKeys();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to revoke");
    }
  }

  const stats = {
    issued: activity.filter((e) => e.event_type === "mandate_issued").length,
    purchases: activity.filter((e) => e.event_type === "order_created" || e.event_type === "budget_debited").length,
    blocked: activity.filter((e) => e.decision === "blocked").length,
  };

  return (
    <div className="mx-auto max-w-5xl">
      <PageHeader
        title="Agent console"
        description="Provision scoped keys and watch your buyer agents act live."
        icon={<Icon name="bot" className="h-5 w-5" />}
      />

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      <div className="mb-6 grid grid-cols-3 gap-4">
        <StatCard index={0} label="Mandates issued" value={String(stats.issued)} icon={<Icon name="shield" className="h-4 w-4" />} />
        <StatCard index={1} label="Purchases" value={String(stats.purchases)} accent icon={<Icon name="package" className="h-4 w-4" />} />
        <StatCard index={2} label="Blocked" value={String(stats.blocked)} icon={<Icon name="lock" className="h-4 w-4" />} />
      </div>

      <Card className="mb-8 p-6 mg-stagger-3">
        <h2 className="mb-4 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          API keys
        </h2>
        <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-end">
          <label className="flex flex-1 flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Key label</span>
            <input
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              className="rounded-lg border border-border bg-background px-3 py-2 text-foreground outline-none transition-colors focus-visible:border-accent"
            />
          </label>
          <button
            onClick={onCreate}
            className="rounded-lg bg-accent px-4 py-2.5 font-medium text-on-accent transition-all hover:opacity-90 active:scale-[0.98]"
          >
            Issue key
          </button>
        </div>
        {rawKey && (
          <div className="mg-fade-in mb-4 rounded-lg border border-accent/40 bg-accent/10 px-3 py-2.5">
            <p className="text-xs text-muted-foreground">New key (copy now):</p>
            <code className="block break-all text-sm font-medium text-accent">{rawKey}</code>
          </div>
        )}
        {loadingKeys ? (
          <div className="flex flex-col gap-2">
            {Array.from({ length: 2 }).map((_, i) => (
              <Skeleton key={i} className="h-12 rounded-lg" />
            ))}
          </div>
        ) : (
          <ul className="flex flex-col gap-2">
            {keys.length === 0 && (
              <li className="text-sm text-muted-foreground">No keys issued yet.</li>
            )}
            {keys.map((k) => (
              <li
                key={k.key_id}
                className="flex items-center justify-between gap-3 rounded-lg border border-border bg-background px-3.5 py-2.5"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium text-foreground">{k.label}</p>
                  <p className="truncate text-xs text-muted-foreground">
                    {k.scopes.join(", ")} · {k.revoked ? "revoked" : "active"}
                  </p>
                </div>
                {!k.revoked ? (
                  <button
                    onClick={() => onRevoke(k.key_id)}
                    className="rounded-lg border border-border px-3 py-1.5 text-sm text-destructive transition-colors hover:bg-destructive/10"
                  >
                    Revoke
                  </button>
                ) : (
                  <Badge tone="muted">revoked</Badge>
                )}
              </li>
            ))}
          </ul>
        )}
      </Card>

      <div className="mg-stagger-4">
        <div className="mb-3 flex items-center gap-2.5">
          <span className="live-dot h-2.5 w-2.5 rounded-full bg-accent" />
          <h2 className="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
            Live agent activity
          </h2>
        </div>
        <ul className="divide-y divide-border overflow-hidden rounded-xl border border-border">
          {activity.length === 0 && (
            <li className="px-4 py-8 text-sm text-muted-foreground">
              No agent activity yet. Start the worker or issue a mandate.
            </li>
          )}
          {activity.slice((page - 1) * PAGE, page * PAGE).map((e) => {
            const k = `${e.created_at}|${e.event_type}|${e.actor}`;
            return (
              <li
                key={k}
                className={`flex items-center justify-between gap-3 px-4 py-3 text-sm ${
                  newKeys.has(k) ? "mg-fade-in bg-accent/5" : ""
                }`}
              >
                <div className="min-w-0">
                  <p className="font-medium text-foreground">{e.event_type}</p>
                  <p className="truncate text-xs text-muted-foreground">
                    {e.actor}
                    {e.reason ? ` · ${e.reason}` : ""}
                  </p>
                </div>
                <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                  {new Date(e.created_at).toLocaleTimeString("en-IN")}
                </span>
              </li>
            );
          })}
        </ul>
        {activity.length > PAGE && (
          <Pagination page={page} pageSize={PAGE} total={activity.length} onPage={setPage} />
        )}
      </div>
    </div>
  );
}
