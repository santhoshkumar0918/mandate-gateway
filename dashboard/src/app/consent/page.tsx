import { MandateCard } from "./mandate-card";
import { ActionButtons } from "./action-buttons";
import { fetchMandate } from "@/lib/gateway-api";

export default async function ConsentPage({
  searchParams,
}: {
  searchParams: Promise<{ mandate_id?: string }>;
}) {
  const { mandate_id } = await searchParams;

  if (!mandate_id) {
    return (
      <div className="flex flex-1 items-center justify-center min-h-screen">
        <div className="text-center">
          <h1 className="text-2xl font-semibold text-foreground mb-2">
            No mandate specified
          </h1>
          <p className="text-muted">
            Provide a mandate_id query parameter to view consent details.
          </p>
        </div>
      </div>
    );
  }

  let mandate;
  try {
    mandate = await fetchMandate(mandate_id);
  } catch {
    return (
      <div className="flex flex-1 items-center justify-center min-h-screen">
        <div className="text-center">
          <h1 className="text-2xl font-semibold text-foreground mb-2">
            Mandate not found
          </h1>
          <p className="text-muted">
            Could not load mandate {mandate_id}. Check the ID and try again.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1 items-center justify-center min-h-screen px-4">
      <div className="w-full max-w-lg">
        <div className="mb-8">
          <h1 className="text-2xl font-semibold text-foreground">
            Mandate Approval Request
          </h1>
          <p className="text-muted mt-1">
            An agent is requesting permission to transact on your behalf.
            Review the details below before approving.
          </p>
        </div>

        <MandateCard mandate={mandate} />

        <div className="mt-6">
          <ActionButtons mandateId={mandate.mandate_id} />
        </div>
      </div>
    </div>
  );
}
