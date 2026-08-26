import Link from "next/link";

export default function Home() {
  return (
    <div className="flex flex-1 items-center justify-center min-h-screen">
      <div className="text-center">
        <h1 className="text-3xl font-semibold text-foreground mb-3">
          Mandate Gateway
        </h1>
        <p className="text-muted mb-8">
          Merchant consent dashboard for agent mandate approvals.
        </p>
        <Link
          href="/consent?mandate_id=demo"
          className="inline-flex items-center rounded-lg bg-foreground px-5 py-2.5 text-sm font-medium text-background transition-colors hover:opacity-90"
        >
          View Consent Page
        </Link>
      </div>
    </div>
  );
}
