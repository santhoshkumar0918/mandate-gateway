import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { AppShell } from "@/components/AppShell";
import { fetchMe, TOKEN_COOKIE } from "@/lib/gateway-api";

export default async function AppLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const cookieStore = await cookies();
  const token = cookieStore.get(TOKEN_COOKIE)?.value;
  if (!token) redirect("/login");

  let user = { role: "unknown", tenant_id: null as string | null };
  try {
    const me = await fetchMe(token);
    user = { role: me.role, tenant_id: me.tenant_id };
  } catch {
    redirect("/login");
  }

  return <AppShell user={user}>{children}</AppShell>;
}
