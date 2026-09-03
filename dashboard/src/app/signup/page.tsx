"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { signup } from "@/lib/gateway-api";
import { setToken, setRole as persistRole } from "@/lib/auth-client";
import { Icon } from "@/components/Icon";

export default function SignupPage() {
  const router = useRouter();
  const [role, setRole] = useState<"merchant" | "agent">("merchant");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [tenantId, setTenantId] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const res = await signup({
        role,
        name,
        email,
        password,
        tenant_id: tenantId || undefined,
      });
      setToken(res.token);
      persistRole(res.role);
      router.replace("/dashboard");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Signup failed");
      setLoading(false);
    }
  }

  const inputCls =
    "rounded-lg border border-border bg-background px-3 py-2.5 text-foreground outline-none transition-colors focus-visible:border-accent";

  return (
    <div className="relative grid min-h-screen place-items-center overflow-hidden bg-background px-4 py-10">
      <div
        aria-hidden
        className="pointer-events-none absolute -top-40 left-1/2 h-96 w-[42rem] -translate-x-1/2 rounded-full bg-accent/10 blur-3xl"
      />
      <div className="mg-fade-in w-full max-w-sm rounded-2xl border border-border bg-card/70 p-7 shadow-card backdrop-blur">
        <div className="mb-6 flex items-center gap-2.5">
          <span className="grid h-10 w-10 place-items-center rounded-xl bg-accent text-lg font-bold text-on-accent shadow-card">
            <Icon name="bolt" className="h-5 w-5" />
          </span>
          <div>
            <p className="text-lg font-semibold tracking-tight">Mandate Gateway</p>
            <p className="text-xs text-muted-foreground">Create your account</p>
          </div>
        </div>

        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Account type</span>
            <select value={role} onChange={(e) => setRole(e.target.value as "merchant" | "agent")} className={inputCls}>
              <option value="merchant">Merchant</option>
              <option value="agent">Buyer agent</option>
            </select>
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Name</span>
            <input required value={name} onChange={(e) => setName(e.target.value)} className={inputCls} />
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Email</span>
            <input type="email" required value={email} onChange={(e) => setEmail(e.target.value)} className={inputCls} />
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">
              Tenant ID {role === "agent" ? "(agent id)" : "(merchant id)"}
            </span>
            <input
              value={tenantId}
              onChange={(e) => setTenantId(e.target.value)}
              placeholder={role === "agent" ? "agent-001" : "merchant-001"}
              className={inputCls}
            />
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Password</span>
            <input type="password" required value={password} onChange={(e) => setPassword(e.target.value)} className={inputCls} />
          </label>
          {error && (
            <p
              role="alert"
              className="mg-fade-in rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            >
              {error}
            </p>
          )}
          <button
            type="submit"
            disabled={loading}
            className="rounded-lg bg-accent px-4 py-2.5 font-medium text-on-accent transition-all hover:opacity-90 hover:shadow-card active:scale-[0.99] disabled:opacity-60"
          >
            {loading ? "Creating…" : "Create account"}
          </button>
        </form>

        <p className="mt-4 text-center text-sm text-muted-foreground">
          Already have an account?{" "}
          <a href="/login" className="text-accent transition-colors hover:text-accent/80 hover:underline">
            Sign in
          </a>
        </p>
      </div>
    </div>
  );
}
