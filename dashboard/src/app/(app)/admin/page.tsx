"use client";

import { useCallback, useEffect, useState } from "react";
import { getAdminMetrics, type AdminMetrics } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";

function rupee(paise: number): string {
  return `₹${(paise / 100).toLocaleString("en-IN", { maximumFractionDigits: 2 })}`;
}

function Kpi({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="rounded-xl border border-border bg-card p-4">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
      <p className={`mt-1 text-2xl font-semibold ${accent ? "text-accent" : "text-foreground"}`}>
        {value}
      </p>
    </div>
  );
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
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Admin Console</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Platform-wide health and trust metrics.
        </p>
      </div>

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {!m ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : (
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
          <Kpi label="Mandates" value={String(m.total_mandates)} />
          <Kpi label="Active" value={String(m.active_mandates)} accent />
          <Kpi label="Orders" value={String(m.total_orders)} />
          <Kpi label="Captured" value={rupee(m.captured_volume_paise)} />
          <Kpi label="Blocked" value={String(m.blocked_decisions)} />
          <Kpi
            label="Mismatches"
            value={String(m.mismatches)}
            accent={m.unresolved_mismatches > 0}
          />
          <Kpi label="Unresolved" value={String(m.unresolved_mismatches)} />
          <Kpi
            label="Unfulfilled"
            value={String(m.unfulfilled_orders)}
            accent={m.unfulfilled_orders > 0}
          />
          <Kpi label="Merchants" value={String(m.merchants)} />
          <Kpi label="Agents" value={String(m.agents)} />
          <Kpi label="Admins" value={String(m.admins)} />
          <Kpi label="Active keys" value={String(m.active_api_keys)} />
        </div>
      )}
    </div>
  );
}
