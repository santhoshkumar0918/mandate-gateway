import { cookies } from "next/headers";
import { fetchCatalog, TOKEN_COOKIE } from "@/lib/gateway-api";

export const dynamic = "force-dynamic";

function StockBadge({ availability }: { availability: string }) {
  const inStock = availability === "in_stock";
  return (
    <span
      className={`rounded-full px-2 py-0.5 text-xs ${
        inStock ? "bg-accent/15 text-accent" : "bg-destructive/15 text-destructive"
      }`}
    >
      {inStock ? "In stock" : availability}
    </span>
  );
}

export default async function CatalogPage() {
  const token = (await cookies()).get(TOKEN_COOKIE)?.value;
  let products: Awaited<ReturnType<typeof fetchCatalog>> = [];
  let failed = false;
  try {
    products = await fetchCatalog(token);
  } catch {
    failed = true;
  }

  return (
    <div className="mx-auto max-w-5xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Catalog</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          What buyer agents can discover and request mandates for.
        </p>
      </div>

      {failed ? (
        <p className="rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          Could not reach the gateway. Is the backend running?
        </p>
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {products.map((p) => (
            <div key={p.product_id} className="mg-fade-in rounded-xl border border-border bg-card p-4">
              <div className="flex items-start justify-between">
                <p className="font-medium text-foreground">{p.title}</p>
                <StockBadge availability={p.availability} />
              </div>
              <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">{p.description}</p>
              <div className="mt-3 flex items-center justify-between">
                <span className="text-sm font-semibold">
                  {(p.price / 100).toLocaleString("en-IN", {
                    style: "currency",
                    currency: p.currency,
                  })}
                </span>
                <span className="text-xs uppercase tracking-wide text-muted-foreground">
                  {p.category}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
