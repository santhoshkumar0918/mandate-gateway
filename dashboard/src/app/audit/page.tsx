import Link from "next/link";
import { fetchAuditTrail, fetchMandate } from "@/lib/gateway-api";

function formatAmount(paise: number): string {
  return `\u20b9${(paise / 100).toLocaleString("en-IN")}`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("en-IN", {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function DecisionBadge({ decision }: { decision: string }) {
  const colors: Record<string, string> = {
    allow: "bg-emerald-50 text-emerald-700 border-emerald-200",
    block: "bg-red-50 text-red-700 border-red-200",
    escalate: "bg-amber-50 text-amber-700 border-amber-200",
  };
  const color = colors[decision] || "bg-zinc-100 text-zinc-500 border-zinc-200";

  return (
    <span
      className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium ${color}`}
    >
      {decision}
    </span>
  );
}

export default async function AuditPage({
  searchParams,
}: {
  searchParams: Promise<{ mandate_id?: string }>;
}) {
  const { mandate_id } = await searchParams;

  if (!mandate_id) {
    return (
      <div className="flex flex-1 items-center justify-center min-h-screen">
        <div className="text-center">
          <h1 className="text-2xl font-semibold text-foreground mb-2">
            No mandate specified
          </h1>
          <p className="text-muted">
            Provide a mandate_id query parameter to view the audit trail.
          </p>
        </div>
      </div>
    );
  }

  let entries;
  let mandate;
  try {
    [entries, mandate] = await Promise.all([
      fetchAuditTrail(mandate_id),
      fetchMandate(mandate_id),
    ]);
  } catch {
    return (
      <div className="flex flex-1 items-center justify-center min-h-screen">
        <div className="text-center">
          <h1 className="text-2xl font-semibold text-foreground mb-2">
            Audit trail not found
          </h1>
          <p className="text-muted">
            Could not load audit data for mandate {mandate_id}.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col min-h-screen px-4 py-8">
      <div className="max-w-2xl mx-auto w-full">
        <div className="mb-6">
          <Link
            href={`/consent?mandate_id=${mandate_id}`}
            className="text-sm text-muted hover:text-foreground transition-colors"
          >
            &larr; Back to mandate
          </Link>
        </div>

        <div className="mb-8">
          <h1 className="text-2xl font-semibold text-foreground">
            Audit Trail
          </h1>
          <p className="text-muted mt-1">
            Complete history of actions for this mandate.
          </p>
        </div>

        <div className="rounded-xl border border-card-border bg-card p-4 mb-6">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-xs text-muted">Mandate</p>
              <p className="font-mono text-sm text-foreground mt-0.5">
                {mandate.mandate_id.slice(0, 8)}...
              </p>
            </div>
            <div>
              <p className="text-xs text-muted">Status</p>
              <p className="text-sm font-medium text-foreground mt-0.5">
                {mandate.status}
              </p>
            </div>
            <div>
              <p className="text-xs text-muted">Budget</p>
              <p className="text-sm text-foreground mt-0.5">
                {formatAmount(mandate.spent_amount)} / {formatAmount(mandate.max_amount)}
              </p>
            </div>
            <div>
              <p className="text-xs text-muted">Agent</p>
              <p className="font-mono text-sm text-foreground mt-0.5">
                {mandate.buyer_agent_id}
              </p>
            </div>
          </div>
        </div>

        <div className="space-y-3">
          {entries.map((entry, i) => (
            <div
              key={i}
              className="rounded-lg border border-card-border bg-card p-4"
            >
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm font-medium text-foreground">
                    {entry.event_type}
                  </span>
                  <DecisionBadge decision={entry.decision} />
                </div>
                <span className="text-xs text-muted">
                  {formatDate(entry.created_at)}
                </span>
              </div>
              {entry.reason && (
                <p className="text-sm text-muted mb-1">{entry.reason}</p>
              )}
              {entry.detail && (
                <pre className="text-xs text-muted bg-background rounded p-2 mt-2 overflow-x-auto">
                  {JSON.stringify(entry.detail, null, 2)}
                </pre>
              )}
              <p className="text-xs text-muted mt-2">
                Actor: <span className="font-mono">{entry.actor}</span>
              </p>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
