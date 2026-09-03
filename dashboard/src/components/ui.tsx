import type { ReactNode } from "react";

/* Shared premium UI atoms for the Mandate Gateway dashboard.
   These are pure presentational helpers — they hold no data and call no
   backend, so they can never drift from the gateway contract. */

export function PageHeader({
  title,
  description,
  icon,
}: {
  title: string;
  description?: string;
  icon?: ReactNode;
}) {
  return (
    <header className="mb-8 flex items-start justify-between gap-4">
      <div>
        <h1 className="flex items-center gap-3 text-2xl font-semibold tracking-tight text-foreground">
          {icon && (
            <span className="grid h-10 w-10 place-items-center rounded-xl border border-border bg-card text-accent shadow-card">
              {icon}
            </span>
          )}
          {title}
        </h1>
        {description && (
          <p className="mt-2 max-w-2xl text-sm leading-relaxed text-muted-foreground">{description}</p>
        )}
      </div>
    </header>
  );
}

export function Card({
  children,
  className = "",
  hover = false,
}: {
  children: ReactNode;
  className?: string;
  hover?: boolean;
}) {
  return (
    <div className={`card ${hover ? "card-hover" : ""} ${className}`}>{children}</div>
  );
}

export function StatCard({
  label,
  value,
  sub,
  accent = false,
  icon,
  index = 0,
}: {
  label: string;
  value: string;
  sub?: string;
  accent?: boolean;
  icon?: ReactNode;
  index?: number;
}) {
  const stagger = index <= 6 ? ` mg-stagger-${index}` : "";
  return (
    <Card hover className={`mg-stagger${stagger} p-5`}>
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">{label}</p>
        {icon && <span className="text-accent/80">{icon}</span>}
      </div>
      <p
        className={`mt-3 text-3xl font-semibold tracking-tight tabular-nums ${
          accent ? "text-gradient" : "text-foreground"
        }`}
      >
        {value}
      </p>
      {sub && <p className="mt-1.5 text-xs text-muted-foreground">{sub}</p>}
    </Card>
  );
}

export function Skeleton({ className = "", lines = 1 }: { className?: string; lines?: number }) {
  return (
    <div role="status" aria-label="Loading">
      {Array.from({ length: lines }).map((_, i) => (
        <div key={i} className={`skeleton ${className}`} style={{ height: i === 0 ? undefined : "0.75rem" }} />
      ))}
      <span className="sr-only">Loading…</span>
    </div>
  );
}

export function Badge({
  children,
  tone = "neutral",
  className = "",
}: {
  children: ReactNode;
  tone?: "neutral" | "accent" | "danger" | "warning" | "muted";
  className?: string;
}) {
  const tones: Record<string, string> = {
    neutral: "bg-secondary/70 text-foreground/90 border-border",
    accent: "bg-accent/15 text-accent border-accent/30",
    danger: "bg-destructive/15 text-destructive border-destructive/30",
    warning: "bg-amber-500/15 text-amber-400 border-amber-500/30",
    muted: "bg-muted text-muted-foreground border-border",
  };
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium ${tones[tone]} ${className}`}
    >
      {children}
    </span>
  );
}
