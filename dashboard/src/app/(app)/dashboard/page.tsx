import Link from "next/link";
import { cookies } from "next/headers";
import { fetchCatalog, TOKEN_COOKIE } from "@/lib/gateway-api";

export const dynamic = "force-dynamic";

function StatCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="mg-fade-in rounded-xl border border-border bg-card p-5">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">{label}</p>
      <p className="mt-2 text-2xl font-semibold text-foreground">{value}</p>
      {hint && <p className="mt-1 text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

export default async function DashboardHome() {
  const token = (await cookies()).get(TOKEN_COOKIE)?.value;
  let productCount = 0;
  let catalogValue = 0;
  let currency = "INR";

  try {
    const catalog = await fetchCatalog(token);
    productCount = catalog.length;
    catalogValue = catalog.reduce((sum, p) => sum + p.price, 0);
    if (catalog[0]) currency = catalog[0].currency;
  } catch {
    // gateway unreachable — render shell with zeros
  }

  const fmt = new Intl.NumberFormat("en-IN", {
    style: "currency",
    currency,
    maximumFractionDigits: 0,
  });

  return (
    <div className="mx-auto max-w-5xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Merchant dashboard</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Your store is reachable by AI buyer agents through signed, scoped mandates.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard label="Live products" value={String(productCount)} hint="agent-discoverable" />
        <StatCard label="Catalog value" value={fmt.format(catalogValue / 100)} hint="total listed" />
        <StatCard label="Agent access" value="Scoped" hint="per-mandate gating" />
      </div>

      <div className="mt-8 grid grid-cols-1 gap-4 sm:grid-cols-2">
        <Link
          href="/catalog"
          className="mg-fade-in rounded-xl border border-border bg-card p-5 transition-colors hover:border-accent"
        >
          <p className="font-medium text-foreground">Manage catalog</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Review what buyer agents can discover and purchase.
          </p>
        </Link>
        <Link
          href="/agents"
          className="mg-fade-in rounded-xl border border-border bg-card p-5 transition-colors hover:border-accent"
        >
          <p className="font-medium text-foreground">Issue agent keys</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Provision scoped API keys for your buyer agents.
          </p>
        </Link>
      </div>
    </div>
  );
}
