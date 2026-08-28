"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { clearToken } from "@/lib/auth-client";
import { Icon } from "@/components/Icon";

interface NavItem {
  href: string;
  label: string;
  icon: string;
  group: "operations" | "platform";
}

const NAV: NavItem[] = [
  { href: "/dashboard", label: "Overview", icon: "grid", group: "operations" },
  { href: "/catalog", label: "Catalog", icon: "tag", group: "operations" },
  { href: "/agents", label: "Agent console", icon: "bot", group: "operations" },
  { href: "/mandates", label: "Mandates", icon: "shield", group: "operations" },
  { href: "/audit", label: "Live audit", icon: "list", group: "operations" },
  { href: "/reconciliation", label: "Reconciliation", icon: "alert", group: "operations" },
  { href: "/admin", label: "Admin", icon: "cog", group: "platform" },
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

  function isActive(href: string) {
    return href === "/dashboard" ? pathname === "/dashboard" : pathname.startsWith(href);
  }

  const groups: { key: NavItem["group"]; title: string }[] = [
    { key: "operations", title: "Store" },
    { key: "platform", title: "Platform" },
  ];

  // Show admin only for admins; show the rest for everyone.
  const items = NAV.filter((n) => n.href !== "/admin" || user.role === "admin");

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <aside className="hidden w-64 shrink-0 flex-col border-r border-border bg-primary/40 p-4 md:flex">
        <Link href="/dashboard" className="mb-8 flex items-center gap-2 px-2">
          <span className="grid h-8 w-8 place-items-center rounded-lg bg-gradient-to-br from-[#4ade80] to-[#16a34a] text-[#06281a] font-bold">
            M
          </span>
          <div className="leading-tight">
            <p className="text-sm font-semibold">Mandate Gateway</p>
            <p className="text-[11px] text-muted-foreground">Agentic commerce trust</p>
          </div>
        </Link>

        <nav className="flex flex-1 flex-col gap-5">
          {groups.map((g) => (
            <div key={g.key}>
              <p className="px-3 pb-2 text-[10px] font-semibold uppercase tracking-widest text-muted-foreground/70">
                {g.title}
              </p>
              <div className="flex flex-col gap-1">
                {items
                  .filter((n) => n.group === g.key)
                  .map((item) => {
                    const active = isActive(item.href);
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
                        <Icon name={item.icon} className="h-[18px] w-[18px]" />
                        {item.label}
                      </Link>
                    );
                  })}
              </div>
            </div>
          ))}
        </nav>

        <div className="mt-4 border-t border-border pt-4">
          <div className="flex items-center gap-2 px-1 text-xs text-muted-foreground">
            <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
            <span>
              Signed in as <span className="text-foreground">{user.role}</span>
            </span>
          </div>
          <button
            onClick={logout}
            className="mt-3 w-full rounded-lg border border-border px-3 py-2 text-left text-sm text-foreground transition-colors hover:bg-secondary"
          >
            Sign out
          </button>
        </div>
      </aside>

      <div className="flex flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-border bg-background/70 px-4 py-3 backdrop-blur md:px-6">
          <div className="flex items-center gap-2">
            <span className="hidden h-2 w-2 animate-pulse rounded-full bg-accent sm:block" />
            <p className="text-sm text-muted-foreground">
              Razorpay merchant trust layer
            </p>
          </div>
          <div className="flex items-center gap-2 md:hidden">
            <span className="text-sm font-semibold">Mandate Gateway</span>
          </div>
          <Link
            href="/login"
            className="rounded-lg border border-border px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:border-accent hover:text-accent"
          >
            Try demo
          </Link>
        </header>
        <main className="flex-1 px-4 py-6 md:px-8 md:py-8">{children}</main>
      </div>
    </div>
  );
}
