"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { approveMandate, rejectMandate } from "@/lib/gateway-api";

export function ActionButtons({ mandateId }: { mandateId: string }) {
  const [loading, setLoading] = useState<"approve" | "reject" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const router = useRouter();

  async function handleApprove() {
    setLoading("approve");
    setError(null);
    try {
      await approveMandate(mandateId);
      router.push(`/consent/success?mandate_id=${mandateId}&action=approved`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Approval failed");
      setLoading(null);
    }
  }

  async function handleReject() {
    setLoading("reject");
    setError(null);
    try {
      await rejectMandate(mandateId);
      router.push(`/consent/success?mandate_id=${mandateId}&action=rejected`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Rejection failed");
      setLoading(null);
    }
  }

  return (
    <div>
      <div className="flex gap-3">
        <button
          onClick={handleApprove}
          disabled={loading !== null}
          className="flex-1 rounded-lg bg-approve px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-approve-hover disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {loading === "approve" ? "Approving..." : "Approve"}
        </button>
        <button
          onClick={handleReject}
          disabled={loading !== null}
          className="flex-1 rounded-lg bg-reject px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-reject-hover disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {loading === "reject" ? "Rejecting..." : "Reject"}
        </button>
      </div>
      {error && (
        <p className="mt-3 text-sm text-reject text-center">{error}</p>
      )}
    </div>
  );
}
