"use client";

import { useCallback, useEffect, useState } from "react";
import {
  fetchCatalog,
  updateCatalogPrice,
  type Product,
} from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";

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
    <div className="mx-auto max-w-5xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Catalog</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          What buyer agents can discover and request mandates for. Edit prices
          to drive live inventory.
        </p>
      </div>

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : (
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {products.slice((page - 1) * PAGE, page * PAGE).map((p) => (
            <div key={p.product_id} className="mg-fade-in rounded-xl border border-border bg-card p-4">
              <div className="flex items-start justify-between">
                <p className="font-medium text-foreground">{p.title}</p>
                <StockBadge availability={p.availability} />
              </div>
              <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">{p.description}</p>
              <div className="mt-3 flex items-end justify-between gap-2">
                <label className="flex flex-col gap-1 text-xs text-muted-foreground">
                  Price ({p.currency})
                  <input
                    type="number"
                    step="0.01"
                    defaultValue={(p.price / 100).toFixed(2)}
                    onChange={(e) =>
                      setDrafts((d) => ({ ...d, [p.product_id]: e.target.value }))
                    }
                    className="w-28 rounded-lg border border-border bg-background px-2 py-1.5 text-sm text-foreground outline-none focus-visible:border-accent"
                  />
                </label>
                <button
                  onClick={() => onSave(p)}
                  disabled={saving === p.product_id || drafts[p.product_id] === undefined}
                  className="rounded-lg bg-accent px-3 py-1.5 text-sm font-medium text-on-accent transition-opacity hover:opacity-90 disabled:opacity-50"
                >
                  {saving === p.product_id ? "Saving…" : "Save"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
      <Pagination page={page} pageSize={PAGE} total={products.length} onPage={setPage} />
    </div>
  );
}
