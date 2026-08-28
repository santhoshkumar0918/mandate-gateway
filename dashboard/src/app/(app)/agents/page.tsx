"use client";

import { useCallback, useEffect, useState } from "react";
import {
  createKey,
  listKeys,
  revokeKey,
  type ApiKeyInfo,
} from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";

export default function AgentsPage() {
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [newKey, setNewKey] = useState<string | null>(null);
  const [label, setLabel] = useState("buyer-agent");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setKeys(await listKeys(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load keys");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  async function onCreate() {
    setError(null);
    const token = getToken();
    if (!token) return;
    try {
      const res = await createKey(token, label || "buyer-agent", [
        "mandate:issue",
        "purchase:exec",
      ]);
      setNewKey(res.key);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create key");
    }
  }

  async function onRevoke(id: string) {
    const token = getToken();
    if (!token) return;
    try {
      await revokeKey(token, id);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to revoke");
    }
  }

  return (
    <div className="mx-auto max-w-3xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Agent keys</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Scoped API keys let buyer agents authenticate. Keys show once.
        </p>
      </div>

      <div className="mb-6 rounded-xl border border-border bg-card p-5">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-end">
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
        {newKey && (
          <div className="mt-4 rounded-lg border border-accent/40 bg-accent/10 px-3 py-2">
            <p className="text-xs text-muted-foreground">New key (copy now):</p>
            <code className="block break-all text-sm text-accent">{newKey}</code>
          </div>
        )}
      </div>

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : keys.length === 0 ? (
        <p className="text-sm text-muted-foreground">No keys issued yet.</p>
      ) : (
        <ul className="flex flex-col gap-3">
          {keys.map((k) => (
            <li
              key={k.key_id}
              className="mg-fade-in flex items-center justify-between rounded-xl border border-border bg-card p-4"
            >
              <div>
                <p className="font-medium text-foreground">{k.label}</p>
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
      )}
    </div>
  );
}
