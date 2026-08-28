"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import {
  getMandates,
  rejectMandate,
  type MandateSummary,
} from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";

function StatusBadge({ status }: { status: string }) {
  const map: Record<string, string> = {
    active: "bg-accent/15 text-accent",
    revoked: "bg-destructive/15 text-destructive",
    expired: "bg-muted-foreground/15 text-muted-foreground",
    exhausted: "bg-muted-foreground/15 text-muted-foreground",
  };
  return (
    <span className={`rounded-full px-2 py-0.5 text-xs ${map[status] ?? "bg-muted text-foreground"}`}>
      {status}
    </span>
  );
}

function money(paise: number, currency: string) {
  return (paise / 100).toLocaleString("en-IN", {
    style: "currency",
    currency,
    maximumFractionDigits: 0,
  });
}

export default function MandatesPage() {
  const [mandates, setMandates] = useState<MandateSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const PAGE = 8;

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setMandates(await getMandates(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load mandates");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  async function onRevoke(id: string) {
    const token = getToken();
    if (!token) return;
    try {
      await rejectMandate(id, token);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Revoke failed");
    }
  }

  return (
    <div className="mx-auto max-w-4xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Mandates</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Signed, scoped permissions granted to buyer agents.
        </p>
      </div>

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : mandates.length === 0 ? (
        <p className="text-sm text-muted-foreground">No mandates issued yet.</p>
      ) : (
        <ul className="flex flex-col gap-3">
          {mandates.slice((page - 1) * PAGE, page * PAGE).map((m) => (
            <li key={m.mandate_id} className="mg-fade-in rounded-xl border border-border bg-card p-4">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <p className="font-medium text-foreground">
                    {money(m.max_amount, m.currency)} · {m.frequency}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    agent {m.buyer_agent_id} · scope {m.scope.join(", ")}
                  </p>
                  <p className="mt-0.5 font-mono text-[11px] text-muted-foreground">
                    {m.mandate_id}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <StatusBadge status={m.status} />
                  {m.status === "active" && (
                    <button
                      onClick={() => onRevoke(m.mandate_id)}
                      className="rounded-lg border border-border px-3 py-1.5 text-sm text-destructive transition-colors hover:bg-destructive/10"
                    >
                      Revoke
                    </button>
                  )}
                </div>
              </div>
              <div className="mt-3 flex gap-3 text-xs">
                <Link href={`/consent?mandate_id=${m.mandate_id}`} className="text-accent hover:underline">
                  Review
                </Link>
                <Link href={`/audit?mandate_id=${m.mandate_id}`} className="text-accent hover:underline">
                  Audit trail
                </Link>
              </div>
            </li>
          ))}
        </ul>
      )}
      <Pagination page={page} pageSize={PAGE} total={mandates.length} onPage={setPage} />
    </div>
  );
}
