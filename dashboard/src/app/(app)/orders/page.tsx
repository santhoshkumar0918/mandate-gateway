"use client";

import { useCallback, useEffect, useState } from "react";
import { getOrders, type Order } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";

function FulfillmentBadge({ o }: { o: Order }) {
  const fulfilled = o.fulfillment === "fulfilled";
  return (
    <span
      className={`rounded-full px-2 py-0.5 text-xs ${
        fulfilled ? "bg-accent/15 text-accent" : "bg-destructive/15 text-destructive"
      }`}
    >
      {fulfilled ? "fulfilled" : "unfulfilled"}
    </span>
  );
}

function PaymentBadge({ paymentId }: { paymentId: string | null }) {
  return paymentId ? (
    <span
      title={paymentId}
      className="rounded-full bg-accent/15 px-2 py-0.5 text-xs text-accent"
    >
      captured
    </span>
  ) : null;
}

export default function OrdersPage() {
  const [items, setItems] = useState<Order[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const PAGE = 10;

  const load = useCallback(async () => {
    const token = getToken();
    if (!token) return;
    try {
      setItems(await getOrders(token));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load orders");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
    const t = setInterval(load, 5000);
    return () => clearInterval(t);
  }, [load]);

  const unfulfilled = items.filter((o) => o.fulfillment !== "fulfilled").length;

  return (
    <div className="mx-auto max-w-5xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Orders</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Every purchase with its fulfillment status and the buyer agent&#39;s intent.
        </p>
      </div>

      {unfulfilled > 0 && (
        <div
          role="alert"
          className="mb-6 rounded-xl border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        >
          {unfulfilled} order{unfulfilled > 1 ? "s" : ""} awaiting fulfillment.
        </div>
      )}

      {error && (
        <p role="alert" className="mb-4 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : items.length === 0 ? (
        <p className="text-sm text-muted-foreground">No orders yet.</p>
      ) : (
        <ul className="flex flex-col gap-3">
          {items.slice((page - 1) * PAGE, page * PAGE).map((o) => (
            <li
              key={o.order_id}
              className={`mg-fade-in rounded-xl border bg-card p-4 ${
                o.fulfillment === "fulfilled" ? "border-border" : "border-destructive/50"
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate font-medium text-foreground">
                    {o.product_id ?? o.order_id}
                  </p>
                  <p className="mt-0.5 truncate text-xs text-muted-foreground">
                    {o.order_id} · {new Date(o.created_at).toLocaleString("en-IN")}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <span className="text-sm font-medium">
                    ₹{(o.amount / 100).toLocaleString("en-IN")}
                  </span>
                  <PaymentBadge paymentId={o.payment_id} />
                  <FulfillmentBadge o={o} />
                </div>
              </div>
              <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
                {o.category && <span>category {o.category}</span>}
                {o.selection_method && (
                  <span className="rounded-full bg-muted px-2 py-0.5">
                    {o.selection_method}
                  </span>
                )}
              </div>
              {o.reasoning && (
                <p className="mt-2 rounded-lg bg-muted/60 px-3 py-2 text-xs text-foreground/80 italic">
                  “{o.reasoning}”
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
      <Pagination page={page} pageSize={PAGE} total={items.length} onPage={setPage} />
    </div>
  );
}
