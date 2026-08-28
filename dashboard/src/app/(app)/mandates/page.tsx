export default function MandatesPage() {
  return (
    <div className="mx-auto max-w-3xl">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold">Mandates</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Signed, scoped permissions granted to buyer agents.
        </p>
      </div>
      <div className="rounded-xl border border-border bg-card p-6 text-sm text-muted-foreground">
        <p>
          The mandate console (issue, review, revoke, expiry status) lands in the
          merchant dashboard ticket. Mandates are already enforced by the policy
          engine on every purchase — this view is the operator-facing surface.
        </p>
        <p className="mt-3">
          Track the work in{" "}
          <code className="text-foreground">ANCHOR.md</code> (ticket 06).
        </p>
      </div>
    </div>
  );
}
