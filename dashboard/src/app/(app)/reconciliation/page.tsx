"use client";

import { useCallback, useEffect, useState } from "react";
import { getMismatches, type Mismatch } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";

function StatusBadge({ status }: { status: string }) {
  const map: Record<string, string> = {
    detected: "bg-destructive/15 text-destructive",
    refund_initiated: "bg-amber-400/15 text-amber-300",
    refund_completed: "bg-accent/15 text-accent",
    released: "bg-muted/40 text-muted-foreground",
  };
  return (
    <span className={`rounded-full px-2 py-0.5 text-xs ${map[status] ?? "bg-muted text-foreground"}`}>
      {status.replace("_", " ")}
    </span>
  );
}

export default function ReconciliationPage() {
  const [items, setItems] = useState<Mismatch[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const PAGE = 8;

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setItems(await getMismatches(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load mismatches");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
    const t = setInterval(load, 4000);
    return () => clearInterval(t);
  }, [load]);

  const open = items.filter(
    (m) => m.status !== "refund_completed" && m.status !== "released"
  );

  return (
    <div className="mx-auto max-w-4xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Reconciliation</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Intent-vs-outcome mismatches the engine detected and is recovering.
        </p>
      </div>

      {open.length > 0 && (
        <div
          role="alert"
          className="mb-6 rounded-xl border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          {open.length} unreconciled mismatch{open.length > 1 ? "es" : ""} detected — refunds
          in progress.
        </div>
      )}

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : items.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No mismatches. Every intent matched its outcome.
        </p>
      ) : (
        <ul className="flex flex-col gap-3">
          {items.slice((page - 1) * PAGE, page * PAGE).map((m) => (
            <li
              key={m.mismatch_id}
              className={`mg-fade-in rounded-xl border bg-card p-4 ${
                m.status === "detected" ? "border-destructive/50" : "border-border"
              }`}
            >
              <div className="flex items-center justify-between">
                <p className="font-medium text-foreground">
                  {String((m.kind && (m.kind as Record<string, unknown>).kind) ?? "mismatch")}
                </p>
                <StatusBadge status={m.status} />
              </div>
              <p className="mt-1 text-xs text-muted-foreground">
                mandate {m.mandate_id} · detected {new Date(m.detected_at).toLocaleString("en-IN")}
              </p>
              {m.refund_id && (
                <p className="mt-1 text-xs text-muted-foreground">refund {m.refund_id}</p>
              )}
            </li>
          ))}
        </ul>
      )}
      <Pagination page={page} pageSize={PAGE} total={items.length} onPage={setPage} />
    </div>
  );
}
