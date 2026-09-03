"use client";

import { useCallback, useEffect, useState } from "react";
import { fetchCatalog, updateCatalogPrice, type Product } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";
import { Badge, Card, PageHeader, Skeleton } from "@/components/ui";
import { Icon } from "@/components/Icon";

function StockBadge({ availability }: { availability: string }) {
  const inStock = availability === "in_stock";
  return (
    <Badge tone={inStock ? "accent" : "danger"}>{inStock ? "In stock" : availability}</Badge>
  );
}

export default function CatalogPage() {
  const [products, setProducts] = useState<Product[]>([]);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const PAGE = 6;

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setProducts(await fetchCatalog(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load catalog");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  async function onSave(p: Product) {
    const token = getToken();
    if (!token) return;
    const raw = drafts[p.product_id];
    if (raw === undefined) return;
    const price = Math.round(parseFloat(raw) * 100);
    if (!Number.isFinite(price) || price < 0) {
      setError("Enter a valid price in major units (e.g. 1299).");
      return;
    }
    setSaving(p.product_id);
    setError(null);
    try {
      const updated = await updateCatalogPrice(token, p.product_id, price);
      setProducts((prev) =>
        prev.map((x) => (x.product_id === updated.product_id ? { ...x, price: updated.price } : x)),
      );
      setDrafts((d) => {
        const next = { ...d };
        delete next[p.product_id];
        return next;
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Price update failed");
    } finally {
      setSaving(null);
    }
  }

  return (
    <div className="mx-auto max-w-6xl">
      <PageHeader
        title="Catalog"
        description="What buyer agents can discover and request mandates for. Edit prices to drive live inventory."
        icon={<Icon name="tag" className="h-5 w-5" />}
      />

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="card p-5">
              <div className="flex items-center justify-between gap-3">
                <Skeleton className="h-4 w-32" />
                <Skeleton className="h-5 w-16 rounded-full" />
              </div>
              <Skeleton className="mt-3 h-3 w-full" />
              <Skeleton className="mt-2 h-3 w-2/3" />
              <div className="mt-5 flex items-center justify-between">
                <Skeleton className="h-8 w-24" />
                <Skeleton className="h-8 w-16" />
              </div>
            </div>
          ))}
        </div>
      ) : products.length === 0 ? (
        <Card className="p-10 text-center">
          <Icon name="tag" className="mx-auto h-8 w-8 text-muted-foreground/50" />
          <p className="mt-3 text-sm text-muted-foreground">No products in your catalog yet.</p>
        </Card>
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {products.slice((page - 1) * PAGE, page * PAGE).map((p, i) => (
            <Card key={p.product_id} hover className={`mg-stagger mg-stagger-${(i % 4) + 1} p-5`}>
              <div className="flex items-start justify-between gap-3">
                <p className="font-medium text-foreground">{p.title}</p>
                <StockBadge availability={p.availability} />
              </div>
              <p className="mt-1.5 line-clamp-2 text-sm text-muted-foreground">{p.description}</p>
              <div className="mt-4 flex items-end justify-between gap-2">
                <label className="flex flex-col gap-1.5 text-xs text-muted-foreground">
                  Price ({p.currency})
                  <div className="relative">
                    <span className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
                      ₹
                    </span>
                    <input
                      type="number"
                      step="0.01"
                      defaultValue={(p.price / 100).toFixed(2)}
                      onChange={(e) => setDrafts((d) => ({ ...d, [p.product_id]: e.target.value }))}
                      className="w-28 rounded-lg border border-border bg-background py-1.5 pl-6 pr-2 text-sm text-foreground outline-none transition-colors focus-visible:border-accent"
                    />
                  </div>
                </label>
                <button
                  onClick={() => onSave(p)}
                  disabled={saving === p.product_id || drafts[p.product_id] === undefined}
                  className="rounded-lg bg-accent px-3.5 py-1.5 text-sm font-medium text-on-accent transition-all hover:opacity-90 disabled:opacity-40"
                >
                  {saving === p.product_id ? "Saving…" : "Save"}
                </button>
              </div>
            </Card>
          ))}
        </div>
      )}
      {products.length > PAGE && (
        <Pagination page={page} pageSize={PAGE} total={products.length} onPage={setPage} />
      )}
    </div>
  );
}
