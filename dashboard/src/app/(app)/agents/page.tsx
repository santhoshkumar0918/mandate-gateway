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

const AGENT_EVENTS = new Set([
  "mandate_issued",
  "mandate.revoke",
  "policy.evaluate",
  "payment.attempt",
  "payment.captured",
  "reconcile.check",
  "reconcile.mismatch",
]);

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-xl border border-border bg-card p-4">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
      <p className="mt-1 text-2xl font-semibold">{value}</p>
    </div>
  );
}

export default function AgentConsolePage() {
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [activity, setActivity] = useState<AuditFeedEntry[]>([]);
  const [newKeys, setNewKeys] = useState<Set<string>>(new Set());
  const [label, setLabel] = useState("buyer-agent");
  const [rawKey, setRawKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const seen = useRef<Set<string>>(new Set());

  const loadKeys = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setKeys(await listKeys(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load keys");
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
      fresh.forEach((e) => seen.current.add(`${e.created_at}|${e.event_type}|${e.actor}`));
      if (fresh.length) {
        setNewKeys(new Set(fresh.map((e) => `${e.created_at}|${e.event_type}|${e.actor}`)));
        setTimeout(() => setNewKeys(new Set()), 1200);
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
    purchases: activity.filter((e) => e.event_type === "payment.captured").length,
    blocked: activity.filter((e) => e.decision === "blocked").length,
  };

  return (
    <div className="mx-auto max-w-4xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Agent console</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Provision scoped keys and watch your buyer agents act live.
        </p>
      </div>

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      <div className="mb-8 grid grid-cols-3 gap-4">
        <Stat label="Mandates issued" value={stats.issued} />
        <Stat label="Purchases" value={stats.purchases} />
        <Stat label="Blocked" value={stats.blocked} />
      </div>

      <section className="mb-8 rounded-xl border border-border bg-card p-5">
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          API keys
        </h2>
        <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-end">
          <label className="flex flex-1 flex-col gap-1 text-sm">
            <span className="text-muted-foreground">Key label</span>
            <input
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              className="rounded-lg border border-border bg-background px-3 py-2 text-foreground outline-none focus-visible:border-accent"
            />
          </label>
          <button
            onClick={onCreate}
            className="rounded-lg bg-accent px-4 py-2.5 font-medium text-on-accent transition-opacity hover:opacity-90"
          >
            Issue key
          </button>
        </div>
        {rawKey && (
          <div className="mb-4 rounded-lg border border-accent/40 bg-accent/10 px-3 py-2">
            <p className="text-xs text-muted-foreground">New key (copy now):</p>
            <code className="block break-all text-sm text-accent">{rawKey}</code>
          </div>
        )}
        <ul className="flex flex-col gap-2">
          {keys.length === 0 && (
            <li className="text-sm text-muted-foreground">No keys issued yet.</li>
          )}
          {keys.map((k) => (
            <li
              key={k.key_id}
              className="flex items-center justify-between rounded-lg border border-border bg-background px-3 py-2"
            >
              <div>
                <p className="text-sm font-medium text-foreground">{k.label}</p>
                <p className="text-xs text-muted-foreground">
                  {k.scopes.join(", ")} · {k.revoked ? "revoked" : "active"}
                </p>
              </div>
              {!k.revoked && (
                <button
                  onClick={() => onRevoke(k.key_id)}
                  className="rounded-lg border border-border px-3 py-1.5 text-sm text-destructive transition-colors hover:bg-destructive/10"
                >
                  Revoke
                </button>
              )}
            </li>
          ))}
        </ul>
      </section>

      <section>
        <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          Live agent activity
        </h2>
        <ul className="divide-y divide-border overflow-hidden rounded-xl border border-border">
          {activity.length === 0 && (
            <li className="px-4 py-6 text-sm text-muted-foreground">
              No agent activity yet. Start the worker or issue a mandate.
            </li>
          )}
          {activity.map((e) => {
            const k = `${e.created_at}|${e.event_type}|${e.actor}`;
            return (
              <li
                key={k}
                className={`flex items-center justify-between px-4 py-3 text-sm ${
                  newKeys.has(k) ? "mg-fade-in bg-accent/5" : ""
                }`}
              >
                <div>
                  <p className="font-medium text-foreground">{e.event_type}</p>
                  <p className="text-xs text-muted-foreground">
                    {e.actor}
                    {e.reason ? ` · ${e.reason}` : ""}
                  </p>
                </div>
                <span className="text-xs text-muted-foreground">
                  {new Date(e.created_at).toLocaleTimeString("en-IN")}
                </span>
              </li>
            );
          })}
        </ul>
      </section>
    </div>
  );
}
