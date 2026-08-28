"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { signup } from "@/lib/gateway-api";
import { setToken } from "@/lib/auth-client";

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
      router.replace("/");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Signup failed");
      setLoading(false);
    }
  }

  return (
    <div className="grid min-h-screen place-items-center bg-background px-4">
      <div className="w-full max-w-sm rounded-2xl border border-border bg-card p-7">
        <div className="mb-6 flex items-center gap-2">
          <span className="grid h-9 w-9 place-items-center rounded-lg bg-accent text-on-accent font-bold">
            M
          </span>
          <div>
            <p className="font-semibold">Mandate Gateway</p>
            <p className="text-xs text-muted-foreground">Create your account</p>
          </div>
        </div>
        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">Account type</span>
            <select
              value={role}
              onChange={(e) => setRole(e.target.value as "merchant" | "agent")}
              className="rounded-lg border border-border bg-background px-3 py-2 text-foreground outline-none focus-visible:border-accent"
            >
              <option value="merchant">Merchant</option>
              <option value="agent">Buyer agent</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">Name</span>
            <input
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="rounded-lg border border-border bg-background px-3 py-2 text-foreground outline-none focus-visible:border-accent"
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">Email</span>
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="rounded-lg border border-border bg-background px-3 py-2 text-foreground outline-none focus-visible:border-accent"
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">
              Tenant ID {role === "agent" ? "(agent id)" : "(merchant id)"}
            </span>
            <input
              value={tenantId}
              onChange={(e) => setTenantId(e.target.value)}
              placeholder={role === "agent" ? "agent-001" : "merchant-001"}
              className="rounded-lg border border-border bg-background px-3 py-2 text-foreground outline-none focus-visible:border-accent"
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">Password</span>
            <input
              type="password"
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="rounded-lg border border-border bg-background px-3 py-2 text-foreground outline-none focus-visible:border-accent"
            />
          </label>
          {error && (
            <p role="alert" className="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </p>
          )}
          <button
            type="submit"
            disabled={loading}
            className="rounded-lg bg-accent px-4 py-2.5 font-medium text-on-accent transition-opacity hover:opacity-90 disabled:opacity-60"
          >
            {loading ? "Creating…" : "Create account"}
          </button>
        </form>
        <p className="mt-4 text-center text-sm text-muted-foreground">
          Already have an account?{" "}
          <a href="/login" className="text-accent hover:underline">
            Sign in
          </a>
        </p>
      </div>
    </div>
  );
}
