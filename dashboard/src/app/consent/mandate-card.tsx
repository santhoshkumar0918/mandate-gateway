import type { Mandate } from "@/lib/gateway-api";

function formatAmount(paise: number): string {
  return `₹${(paise / 100).toLocaleString("en-IN")}`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("en-IN", {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    active: "bg-emerald-50 text-emerald-700 border-emerald-200",
    revoked: "bg-red-50 text-red-700 border-red-200",
    expired: "bg-zinc-100 text-zinc-500 border-zinc-200",
  };
  const color = colors[status] || colors.expired;

  return (
    <span
      className={`inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium ${color}`}
    >
      {status}
    </span>
  );
}

export function MandateCard({ mandate }: { mandate: Mandate }) {
  return (
    <div className="rounded-xl border border-card-border bg-card p-6 shadow-sm">
      <div className="flex items-center justify-between mb-5">
        <div>
          <p className="text-sm text-muted">Agent</p>
          <p className="font-mono text-sm font-medium text-foreground">
            {mandate.buyer_agent_id}
          </p>
        </div>
        <StatusBadge status={mandate.status} />
      </div>

      <div className="space-y-4">
        <Row label="Merchant" value={mandate.merchant_id} />
        <Row label="User" value={mandate.user_id} />
        <Row
          label="Scope"
          value={mandate.scope.join(", ")}
          highlight
        />
        <Row
          label="Budget"
          value={formatAmount(mandate.max_amount)}
          highlight
        />
        <Row
          label="Per-purchase max"
          value={formatAmount(mandate.max_amount)}
        />
        <Row label="Spent so far" value={formatAmount(mandate.spent_amount)} />
        <Row label="Frequency" value={mandate.frequency} />
        <Row label="Expires" value={formatDate(mandate.expires_at)} />
        <Row
          label="Nonce"
          value={mandate.nonce.slice(0, 16) + "..."}
          mono
        />
      </div>
    </div>
  );
}

function Row({
  label,
  value,
  highlight,
  mono,
}: {
  label: string;
  value: string;
  highlight?: boolean;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-muted">{label}</span>
      <span
        className={`text-sm text-right ${
          highlight
            ? "font-medium text-foreground"
            : mono
              ? "font-mono text-muted"
              : "text-foreground"
        }`}
      >
        {value}
      </span>
    </div>
  );
}
