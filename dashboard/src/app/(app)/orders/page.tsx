"use client";

import { useCallback, useEffect, useState } from "react";
import { getOrders, type Order } from "@/lib/gateway-api";
import { getToken } from "@/lib/auth-client";
import { Pagination } from "@/components/Pagination";
import { Badge, Card, PageHeader, Skeleton } from "@/components/ui";
import { Icon } from "@/components/Icon";

function FulfillmentBadge({ o }: { o: Order }) {
  const fulfilled = o.fulfillment === "fulfilled";
  return <Badge tone={fulfilled ? "accent" : "danger"}>{fulfilled ? "fulfilled" : "unfulfilled"}</Badge>;
}

function PaymentBadge({ paymentId }: { paymentId: string | null }) {
  return paymentId ? (
    <Badge tone="accent" className="cursor-help">
      <Icon name="check" className="h-3 w-3" /> captured
    </Badge>
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
    <div className="mx-auto max-w-6xl">
      <PageHeader
        title="Orders"
        description="Every purchase with its fulfillment status and the buyer agent's intent."
        icon={<Icon name="package" className="h-5 w-5" />}
      />

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
        <div className="flex flex-col gap-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="card p-4">
              <div className="flex items-center justify-between">
                <div className="space-y-2">
                  <Skeleton className="h-4 w-40" />
                  <Skeleton className="h-3 w-64" />
                </div>
                <Skeleton className="h-6 w-24 rounded-full" />
              </div>
              <Skeleton className="mt-3 h-3 w-48" />
            </div>
          ))}
        </div>
      ) : items.length === 0 ? (
        <Card className="p-10 text-center">
          <Icon name="package" className="mx-auto h-8 w-8 text-muted-foreground/50" />
          <p className="mt-3 text-sm text-muted-foreground">No orders yet.</p>
        </Card>
      ) : (
        <ul className="flex flex-col gap-3">
          {items.slice((page - 1) * PAGE, page * PAGE).map((o, i) => (
            <li
              key={o.order_id}
              className={`card card-hover mg-stagger p-5 ${
                o.fulfillment === "fulfilled" ? "" : "border-destructive/40"
              } mg-stagger-${(i % 5) + 1}`}
            >
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate font-medium text-foreground">{o.product_id ?? o.order_id}</p>
                  <p className="mt-0.5 truncate text-xs text-muted-foreground">
                    {o.order_id} · {new Date(o.created_at).toLocaleString("en-IN")}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <span className="text-base font-semibold tabular-nums">
                    ₹{(o.amount / 100).toLocaleString("en-IN")}
                  </span>
                  <PaymentBadge paymentId={o.payment_id} />
                  <FulfillmentBadge o={o} />
                </div>
              </div>
              <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-muted-foreground">
                {o.category && <span>category {o.category}</span>}
                {o.selection_method && (
                  <Badge tone="muted">{o.selection_method}</Badge>
                )}
              </div>
              {o.reasoning && (
                <p className="mt-3 rounded-lg bg-muted/50 px-3.5 py-2.5 text-xs leading-relaxed text-foreground/80 italic">
                  “{o.reasoning}”
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
      {items.length > PAGE && (
        <Pagination page={page} pageSize={PAGE} total={items.length} onPage={setPage} />
      )}
    </div>
  );
}
