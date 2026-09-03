"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { login } from "@/lib/gateway-api";
import { setToken, setRole } from "@/lib/auth-client";
import { Icon } from "@/components/Icon";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const res = await login({ email, password });
      setToken(res.token);
      setRole(res.role);
      const next = new URLSearchParams(window.location.search).get("next") || "/dashboard";
      router.replace(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
      setLoading(false);
    }
  }

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
            <p className="text-xs text-muted-foreground">Sign in to your account</p>
          </div>
        </div>

        <div className="mb-5 rounded-xl border border-accent/40 bg-accent/10 px-4 py-3">
          <p className="flex items-center gap-1.5 text-sm font-medium text-accent">
            <Icon name="key" className="h-4 w-4" /> Live demo unlocked
          </p>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            Sign in with <span className="text-foreground">demo@merchant.local</span> /{" "}
            <span className="font-mono text-foreground">Demo@1234</span> to explore a store
            already running a live buyer agent, mandates and audit trail.
          </p>
        </div>

        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Email</span>
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="rounded-lg border border-border bg-background px-3 py-2.5 text-foreground outline-none transition-colors focus-visible:border-accent"
            />
          </label>
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">Password</span>
            <input
              type="password"
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="rounded-lg border border-border bg-background px-3 py-2.5 text-foreground outline-none transition-colors focus-visible:border-accent"
            />
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
            {loading ? "Signing in…" : "Sign in"}
          </button>
        </form>

        <p className="mt-4 text-center text-sm text-muted-foreground">
          No account?{" "}
          <a href="/signup" className="text-accent transition-colors hover:text-accent/80 hover:underline">
            Create one
          </a>
        </p>
      </div>
    </div>
  );
}
