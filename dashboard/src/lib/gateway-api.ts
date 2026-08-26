const GATEWAY_URL = process.env.NEXT_PUBLIC_GATEWAY_URL || "http://localhost:8000";

export interface Mandate {
  mandate_id: string;
  user_id: string;
  merchant_id: string;
  buyer_agent_id: string;
  max_amount: number;
  currency: string;
  scope: string[];
  frequency: string;
  expires_at: string;
  nonce: string;
  signature: string;
  status: string;
  spent_amount: number;
}

export interface AuditEntry {
  event_type: string;
  entity_id: string;
  decision: string;
  reason: string | null;
  detail: Record<string, unknown> | null;
  actor: string;
  created_at: string;
}

export async function fetchMandate(mandateId: string): Promise<Mandate> {
  const res = await fetch(`${GATEWAY_URL}/mandate/${mandateId}`, {
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch mandate: ${res.status}`);
  }
  return res.json();
}

export async function fetchAuditTrail(mandateId: string): Promise<AuditEntry[]> {
  const res = await fetch(`${GATEWAY_URL}/audit/${mandateId}`, {
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch audit trail: ${res.status}`);
  }
  const data = await res.json();
  return data.entries || [];
}

export async function approveMandate(
  mandateId: string,
): Promise<{ mandate_id: string; status: string }> {
  const res = await fetch(`${GATEWAY_URL}/consent/approve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mandate_id: mandateId, approved: true }),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || `Approval failed: ${res.status}`);
  }
  return res.json();
}

export async function rejectMandate(
  mandateId: string,
): Promise<{ mandate_id: string; status: string }> {
  const res = await fetch(`${GATEWAY_URL}/consent/approve`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ mandate_id: mandateId, approved: false }),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || `Rejection failed: ${res.status}`);
  }
  return res.json();
}
