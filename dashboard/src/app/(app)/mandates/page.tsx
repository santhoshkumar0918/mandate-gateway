"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { getMandates, rejectMandate, type MandateSummary } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";
import { Badge, Card, PageHeader, Skeleton } from "@/components/ui";
import { Icon } from "@/components/Icon";

const STATUS_TONE: Record<string, "accent" | "danger" | "muted"> = {
  active: "accent",
  revoked: "danger",
  expired: "muted",
  exhausted: "muted",
};

function StatusBadge({ status }: { status: string }) {
  return (
    <Badge tone={STATUS_TONE[status] ?? "neutral"}>{status}</Badge>
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
      <PageHeader
        title="Mandates"
        description="Signed, scoped permissions granted to buyer agents."
        icon={<Icon name="shield" className="h-5 w-5" />}
      />

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
                <Skeleton className="h-5 w-40" />
                <Skeleton className="h-6 w-20 rounded-full" />
              </div>
              <Skeleton className="mt-3 h-4 w-64" />
            </div>
          ))}
        </div>
      ) : mandates.length === 0 ? (
        <Card className="p-10 text-center">
          <Icon name="shield" className="mx-auto h-8 w-8 text-muted-foreground/50" />
          <p className="mt-3 text-sm text-muted-foreground">No mandates issued yet.</p>
        </Card>
      ) : (
        <ul className="flex flex-col gap-3">
          {mandates.slice((page - 1) * PAGE, page * PAGE).map((m, i) => (
            <li key={m.mandate_id} className={`card card-hover mg-stagger p-5 mg-stagger-${(i % 4) + 1}`}>
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <p className="font-medium text-foreground">
                    {money(m.max_amount, m.currency)}
                    <span className="mx-2 text-muted-foreground">·</span>
                    <span className="capitalize text-muted-foreground">{m.frequency}</span>
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    agent {m.buyer_agent_id} · scope {m.scope.join(", ")}
                  </p>
                  <p className="mt-1 font-mono text-[11px] text-muted-foreground/70">{m.mandate_id}</p>
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
              <div className="mt-4 flex gap-4 text-xs">
                <Link href={`/consent?mandate_id=${m.mandate_id}`} className="font-medium text-accent transition-colors hover:text-accent/80">
                  Review →
                </Link>
                <Link href={`/audit?mandate_id=${m.mandate_id}`} className="font-medium text-accent transition-colors hover:text-accent/80">
                  Audit trail →
                </Link>
              </div>
            </li>
          ))}
        </ul>
      )}
      {mandates.length > PAGE && (
        <Pagination page={page} pageSize={PAGE} total={mandates.length} onPage={setPage} />
      )}
    </div>
  );
}
