"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { clearToken } from "@/lib/auth-client";

interface NavItem {
  href: string;
  label: string;
  icon: string;
}

const NAV: NavItem[] = [
  { href: "/", label: "Dashboard", icon: "▦" },
  { href: "/catalog", label: "Catalog", icon: "▤" },
  { href: "/mandates", label: "Mandates", icon: "⬡" },
  { href: "/audit", label: "Audit Trail", icon: "≣" },
  { href: "/reconciliation", label: "Reconciliation", icon: "⚠" },
  { href: "/agents", label: "Agent Console", icon: "◉" },
];

const ADMIN_NAV: NavItem[] = [
  ...NAV,
  { href: "/admin", label: "Admin", icon: "⚙" },
];

export function AppShell({
  user,
  children,
}: {
  user: { role: string; tenant_id: string | null };
  children: React.ReactNode;
}) {
  const pathname = usePathname();
  const router = useRouter();

  function logout() {
    clearToken();
    router.replace("/login");
  }

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <aside className="hidden w-64 shrink-0 flex-col border-r border-border bg-primary/40 p-4 md:flex">
        <div className="mb-8 flex items-center gap-2 px-2">
          <span className="grid h-8 w-8 place-items-center rounded-lg bg-accent text-on-accent font-bold">
            M
          </span>
          <div className="leading-tight">
            <p className="text-sm font-semibold">Mandate Gateway</p>
            <p className="text-[11px] text-muted-foreground">Agentic commerce trust</p>
          </div>
        </div>
        <nav className="flex flex-1 flex-col gap-1">
          {(user.role === "admin" ? ADMIN_NAV : NAV).map((item) => {
            const active =
              item.href === "/"
                ? pathname === "/"
                : pathname.startsWith(item.href);
            return (
              <Link
                key={item.href}
                href={item.href}
                aria-current={active ? "page" : undefined}
                className={`flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors ${
                  active
                    ? "bg-accent/15 text-accent"
                    : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                }`}
              >
                <span aria-hidden className="text-base">
                  {item.icon}
                </span>
                {item.label}
              </Link>
            );
          })}
        </nav>
        <div className="mt-4 border-t border-border pt-4 text-xs text-muted-foreground">
          <p>
            Signed in as <span className="text-foreground">{user.role}</span>
          </p>
          {user.tenant_id && <p>tenant: {user.tenant_id}</p>}
          <button
            onClick={logout}
            className="mt-2 w-full rounded-lg border border-border px-3 py-2 text-left text-foreground transition-colors hover:bg-secondary"
          >
            Sign out
          </button>
        </div>
      </aside>

      <div className="flex flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-border px-4 py-3 md:px-6">
          <p className="text-sm text-muted-foreground">
            Razorpay merchant trust layer
          </p>
          <div className="flex items-center gap-2 md:hidden">
            <span className="text-sm font-semibold">Mandate Gateway</span>
          </div>
        </header>
        <main className="flex-1 px-4 py-6 md:px-8 md:py-8">{children}</main>
      </div>
    </div>
  );
}
