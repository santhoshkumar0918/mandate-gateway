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
  { href: "/orders", label: "Orders", icon: "package", group: "operations" },
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

  const items = NAV.filter((n) => n.href !== "/admin" || user.role === "admin");

  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <aside className="sticky top-0 hidden h-screen w-64 shrink-0 flex-col border-r border-border bg-primary/50 p-4 backdrop-blur md:flex">
        <Link href="/dashboard" className="mb-8 flex items-center gap-2.5 px-2">
          <span className="grid h-9 w-9 place-items-center rounded-xl bg-gradient-to-br from-[#4ade80] to-[#16a34a] text-[#06281a] shadow-[0_0_24px_-6px_rgba(34,197,94,0.5)]">
            <Icon name="shield" className="h-5 w-5" />
          </span>
          <div className="leading-tight">
            <p className="text-sm font-semibold tracking-tight text-foreground">Mandate Gateway</p>
            <p className="text-[11px] text-muted-foreground">Agentic trust layer</p>
          </div>
        </Link>

        <nav className="flex flex-1 flex-col gap-6 overflow-y-auto">
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
                        className={`group relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-all duration-200 ${
                          active
                            ? "bg-accent/15 font-medium text-accent"
                            : "text-muted-foreground hover:bg-secondary/70 hover:text-foreground"
                        }`}
                      >
                        <span
                          className={`absolute left-0 top-1/2 h-5 w-[3px] -translate-y-1/2 rounded-full bg-accent transition-opacity duration-200 ${
                            active ? "opacity-100" : "opacity-0"
                          }`}
                        />
                        <Icon
                          name={item.icon}
                          className={`h-[18px] w-[18px] transition-transform duration-200 ${
                            active ? "text-accent" : "group-hover:scale-110"
                          }`}
                        />
                        {item.label}
                      </Link>
                    );
                  })}
              </div>
            </div>
          ))}
        </nav>

        <div className="mt-4 border-t border-border pt-4">
          <div className="flex items-center gap-2.5 rounded-lg bg-secondary/40 px-3 py-2.5">
            <span className="live-dot h-2.5 w-2.5 rounded-full bg-accent" />
            <div className="min-w-0 text-xs">
              <p className="text-muted-foreground">Signed in as</p>
              <p className="truncate font-medium capitalize text-foreground">{user.role}</p>
            </div>
          </div>
          <button
            onClick={logout}
            className="mt-3 w-full rounded-lg border border-border px-3 py-2 text-left text-sm text-foreground transition-colors hover:border-destructive/40 hover:bg-destructive/10 hover:text-destructive"
          >
            Sign out
          </button>
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="sticky top-0 z-20 flex items-center justify-between border-b border-border bg-background/80 px-4 py-3 backdrop-blur-md md:px-8">
          <div className="flex items-center gap-3">
            <Link href="/dashboard" className="flex items-center gap-2 md:hidden">
              <span className="grid h-8 w-8 place-items-center rounded-lg bg-gradient-to-br from-[#4ade80] to-[#16a34a] text-[#06281a]">
                <Icon name="shield" className="h-4 w-4" />
              </span>
              <span className="text-sm font-semibold">Mandate Gateway</span>
            </Link>
            <div className="hidden items-center gap-2 md:flex">
              <span className="live-dot h-2 w-2 rounded-full bg-accent" />
              <p className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                Verify &amp; pay · live
              </p>
            </div>
          </div>
          <button
            onClick={logout}
            className="rounded-lg border border-border px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:border-accent hover:text-accent"
          >
            Sign out
          </button>
        </header>
        <main className="mx-auto w-full max-w-7xl flex-1 px-4 py-8 md:px-8">{children}</main>
      </div>
    </div>
  );
}
