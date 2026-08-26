export default async function ConsentSuccessPage({
  searchParams,
}: {
  searchParams: Promise<{ mandate_id?: string; action?: string }>;
}) {
  const { mandate_id, action } = await searchParams;

  const isApproved = action === "approved";

  return (
    <div className="flex flex-1 items-center justify-center min-h-screen px-4">
      <div className="w-full max-w-md text-center">
        <div
          className={`inline-flex items-center justify-center w-16 h-16 rounded-full mb-6 ${
            isApproved ? "bg-emerald-50" : "bg-red-50"
          }`}
        >
          {isApproved ? (
            <svg
              className="w-8 h-8 text-emerald-600"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={2}
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M4.5 12.75l6 6 9-13.5"
              />
            </svg>
          ) : (
            <svg
              className="w-8 h-8 text-red-600"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={2}
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          )}
        </div>

        <h1 className="text-2xl font-semibold text-foreground mb-2">
          {isApproved ? "Mandate Approved" : "Mandate Rejected"}
        </h1>

        <p className="text-muted mb-6">
          {isApproved
            ? "The agent has been authorized to transact within the specified scope and budget."
            : "The mandate request has been denied. The agent will not be able to transact."}
        </p>

        <div className="rounded-lg border border-card-border bg-card p-4">
          <p className="text-sm text-muted">Mandate ID</p>
          <p className="font-mono text-sm text-foreground mt-1">
            {mandate_id || "unknown"}
          </p>
        </div>
      </div>
    </div>
  );
}
