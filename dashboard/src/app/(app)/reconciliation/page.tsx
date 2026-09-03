"use client";

import { useCallback, useEffect, useState } from "react";
import { getMismatches, type Mismatch } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";
import { Badge, Card, PageHeader, Skeleton } from "@/components/ui";
import { Icon } from "@/components/Icon";

const STATUS_TONE: Record<string, "danger" | "warning" | "accent" | "muted"> = {
  detected: "danger",
  refund_initiated: "warning",
  refund_completed: "accent",
  released: "muted",
};

function StatusBadge({ status }: { status: string }) {
  return (
    <Badge tone={STATUS_TONE[status] ?? "neutral"}>
      {status.replace("_", " ")}
    </Badge>
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
      <PageHeader
        title="Reconciliation"
        description="Intent-vs-outcome mismatches the engine detected and is recovering."
        icon={<Icon name="alert" className="h-5 w-5" />}
      />

      {open.length > 0 && (
        <div
          role="alert"
          className="mg-fade-in mb-6 flex items-center gap-3 rounded-xl border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          <span className="live-dot h-2.5 w-2.5 shrink-0 rounded-full bg-destructive" />
          {open.length} unreconciled mismatch{open.length > 1 ? "es" : ""} detected — refunds in progress.
        </div>
      )}

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <div className="flex flex-col gap-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="card p-5">
              <div className="flex items-center justify-between">
                <Skeleton className="h-4 w-40" />
                <Skeleton className="h-6 w-20 rounded-full" />
              </div>
              <Skeleton className="mt-3 h-3 w-64" />
            </div>
          ))}
        </div>
      ) : items.length === 0 ? (
        <Card className="p-10 text-center">
          <Icon name="check" className="mx-auto h-8 w-8 text-accent/70" />
          <p className="mt-3 text-sm text-muted-foreground">
            No mismatches. Every intent matched its outcome.
          </p>
        </Card>
      ) : (
        <ul className="flex flex-col gap-3">
          {items.slice((page - 1) * PAGE, page * PAGE).map((m, i) => (
            <li
              key={m.mismatch_id}
              className={`card mg-stagger p-5 mg-stagger-${(i % 4) + 1} ${
                m.status === "detected" ? "border-destructive/50" : "card-hover"
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-3">
                  <span
                    className={`grid h-9 w-9 shrink-0 place-items-center rounded-lg ${
                      m.status === "detected"
                        ? "bg-destructive/15 text-destructive"
                        : m.status === "refund_completed"
                          ? "bg-accent/15 text-accent"
                          : "bg-muted text-muted-foreground"
                    }`}
                  >
                    <Icon name="alert" className="h-4 w-4" />
                  </span>
                  <p className="font-medium text-foreground">
                    {String((m.kind && (m.kind as Record<string, unknown>).kind) ?? "mismatch")}
                  </p>
                </div>
                <StatusBadge status={m.status} />
              </div>
              <p className="mt-3 text-xs text-muted-foreground">
                mandate {m.mandate_id} · detected {new Date(m.detected_at).toLocaleString("en-IN")}
              </p>
              {m.refund_id && (
                <p className="mt-1.5 inline-flex items-center gap-1.5 rounded-md bg-accent/10 px-2 py-1 font-mono text-[11px] text-accent">
                  <Icon name="refresh" className="h-3 w-3" /> refund {m.refund_id}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
      {items.length > PAGE && (
        <Pagination page={page} pageSize={PAGE} total={items.length} onPage={setPage} />
      )}
    </div>
  );
}
