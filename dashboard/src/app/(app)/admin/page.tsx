"use client";

import { useCallback, useEffect, useState } from "react";
import { getAdminMetrics, type AdminMetrics } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { PageHeader, Skeleton, StatCard } from "@/components/ui";
import { Icon } from "@/components/Icon";

function rupee(paise: number): string {
  return `₹${(paise / 100).toLocaleString("en-IN", { maximumFractionDigits: 0 })}`;
}

export default function AdminPage() {
  const [m, setM] = useState<AdminMetrics | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setM(await getAdminMetrics(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load metrics");
    }
  }, []);

  useEffect(() => {
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, [load]);

  return (
    <div className="mx-auto max-w-5xl">
      <PageHeader
        title="Admin console"
        description="Platform-wide health and trust metrics."
        icon={<Icon name="cog" className="h-5 w-5" />}
      />

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {!m ? (
        <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
          {Array.from({ length: 8 }).map((_, i) => (
            <div key={i} className="card p-5">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="mt-3 h-8 w-16" />
            </div>
          ))}
        </div>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
            <StatCard index={0} label="Total mandates" value={String(m.total_mandates)} icon={<Icon name="shield" className="h-4 w-4" />} />
            <StatCard index={1} label="Active" value={String(m.active_mandates)} accent icon={<Icon name="check" className="h-4 w-4" />} />
            <StatCard index={2} label="Orders" value={String(m.total_orders)} icon={<Icon name="package" className="h-4 w-4" />} />
            <StatCard index={3} label="Captured volume" value={rupee(m.captured_volume_paise)} accent icon={<Icon name="chart" className="h-4 w-4" />} />
            <StatCard index={4} label="Blocked decisions" value={String(m.blocked_decisions)} icon={<Icon name="lock" className="h-4 w-4" />} />
            <StatCard index={5} label="Mismatches" value={String(m.mismatches)} sub={m.unresolved_mismatches > 0 ? `${m.unresolved_mismatches} unresolved` : "all resolved"} accent={m.unresolved_mismatches > 0} icon={<Icon name="alert" className="h-4 w-4" />} />
            <StatCard index={6} label="Unfulfilled" value={String(m.unfulfilled_orders)} accent={m.unfulfilled_orders > 0} icon={<Icon name="refresh" className="h-4 w-4" />} />
            <StatCard index={7} label="Active keys" value={String(m.active_api_keys)} icon={<Icon name="key" className="h-4 w-4" />} />
            <StatCard index={8} label="Merchants" value={String(m.merchants)} icon={<Icon name="globe" className="h-4 w-4" />} />
            <StatCard index={9} label="Agents" value={String(m.agents)} icon={<Icon name="bot" className="h-4 w-4" />} />
            <StatCard index={10} label="Admins" value={String(m.admins)} icon={<Icon name="eye" className="h-4 w-4" />} />
            <StatCard index={11} label="Unresolved" value={String(m.unresolved_mismatches)} icon={<Icon name="alert" className="h-4 w-4" />} />
          </div>

          <div className="mg-stagger-3 card mt-6 flex items-center gap-4 p-5">
            <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-accent/15 text-accent">
              <Icon name="shield" className="h-5 w-5" />
            </span>
            <div>
              <p className="font-medium text-foreground">Gateway trust status</p>
              <p className="mt-0.5 text-sm text-muted-foreground">
                {m.unresolved_mismatches > 0 || m.unfulfilled_orders > 0
                  ? "Attention needed — open items detected."
                  : "All systems nominal. Every intent matched its outcome."}
              </p>
            </div>
            <span className="ml-auto flex items-center gap-2 text-sm">
              <span
                className={`live-dot h-2.5 w-2.5 rounded-full ${
                  m.unresolved_mismatches > 0 ? "bg-destructive" : "bg-accent"
                }`}
              />
              <span className="hidden text-muted-foreground sm:inline">
                {m.unresolved_mismatches > 0 ? "Degraded" : "Healthy"}
              </span>
            </span>
          </div>
        </>
      )}
    </div>
  );
}
