const GATEWAY_URL = process.env.NEXT_PUBLIC_GATEWAY_URL || "http://localhost:8000";

export const TOKEN_COOKIE = "mg_token";

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

export interface Claims {
  sub: string;
  role: string;
  tenant_id: string | null;
  exp: number;
}

export interface Product {
  product_id: string;
  title: string;
  description: string;
  category: string;
  price: number;
  currency: string;
  availability: string;
}

function authHeader(token?: string): Record<string, string> {
  return token ? { Authorization: `Bearer ${token}` } : {};
}

// ─── Auth ────────────────────────────────────────────────────────────────

export async function signup(input: {
  role: string;
  email: string;
  name: string;
  password: string;
  tenant_id?: string;
}): Promise<{ token: string }> {
  const res = await fetch(`${GATEWAY_URL}/auth/signup`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || `Signup failed: ${res.status}`);
  }
  return res.json();
}

export async function login(input: {
  email: string;
  password: string;
}): Promise<{ token: string; role: string; tenant_id: string | null }> {
  const res = await fetch(`${GATEWAY_URL}/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || `Login failed: ${res.status}`);
  }
  return res.json();
}

export async function fetchMe(token: string): Promise<Claims> {
  const res = await fetch(`${GATEWAY_URL}/auth/me`, {
    headers: authHeader(token),
  });
  if (!res.ok) throw new Error(`Failed to load identity: ${res.status}`);
  return res.json();
}

export interface ApiKeyInfo {
  key_id: string;
  label: string;
  scopes: string[];
  tenant_id: string | null;
  revoked: boolean;
  created_at: string;
  last_used_at: string | null;
}

export async function listKeys(token: string): Promise<ApiKeyInfo[]> {
  const res = await fetch(`${GATEWAY_URL}/agents/keys`, {
    headers: authHeader(token),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Failed to load keys: ${res.status}`);
  const data = await res.json();
  return data.map((k: Record<string, unknown>) => ({
    key_id: k.key_id as string,
    label: k.label as string,
    scopes: (k.scopes as string[]) || [],
    tenant_id: (k.tenant_id as string) ?? null,
    revoked: Boolean(k.revoked),
    created_at: k.created_at as string,
    last_used_at: (k.last_used_at as string) ?? null,
  }));
}

export async function createKey(
  token: string,
  label: string,
  scopes: string[],
): Promise<{ key: string }> {
  const res = await fetch(`${GATEWAY_URL}/agents/keys`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeader(token) },
    body: JSON.stringify({ label, scopes }),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || `Key creation failed: ${res.status}`);
  }
  return res.json();
}

export async function revokeKey(token: string, keyId: string): Promise<void> {
  const res = await fetch(`${GATEWAY_URL}/agents/keys/${keyId}`, {
    method: "DELETE",
    headers: authHeader(token),
  });
  if (!res.ok) throw new Error(`Revoke failed: ${res.status}`);
}

// ─── Catalog ─────────────────────────────────────────────────────────────

export async function fetchMandate(
  mandateId: string,
  token?: string,
): Promise<Mandate> {
  const res = await fetch(`${GATEWAY_URL}/mandate/${mandateId}`, {
    headers: authHeader(token),
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch mandate: ${res.status}`);
  }
  return res.json();
}

export async function fetchCatalog(token?: string): Promise<Product[]> {
  const res = await fetch(`${GATEWAY_URL}/catalog`, {
    headers: authHeader(token),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Failed to load catalog: ${res.status}`);
  return res.json();
}

export interface MandateSummary {
  mandate_id: string;
  merchant_id: string;
  buyer_agent_id: string;
  max_amount: number;
  currency: string;
  scope: string[];
  frequency: string;
  spent_amount: number;
  status: string;
  expires_at: string;
}

export async function getMandates(
  token: string,
  merchantId?: string,
): Promise<MandateSummary[]> {
  const url = new URL(`${GATEWAY_URL}/mandates`);
  if (merchantId) url.searchParams.set("merchant_id", merchantId);
  const res = await fetch(url, { headers: authHeader(token), cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to load mandates: ${res.status}`);
  return res.json();
}

export async function updateCatalogPrice(
  token: string,
  productId: string,
  price: number,
): Promise<{ product_id: string; price: number }> {
  const res = await fetch(`${GATEWAY_URL}/catalog/${productId}/price`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeader(token) },
    body: JSON.stringify({ price }),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || `Price update failed: ${res.status}`);
  }
  return res.json();
}

export interface AuditFeedEntry {
  mandate_id: string;
  event_type: string;
  entity_id: string;
  decision: string;
  reason: string | null;
  detail: Record<string, unknown> | null;
  actor: string;
  created_at: string;
}

export async function getAuditFeed(
  token: string,
  params: { mandate_id?: string; event_type?: string; decision?: string; limit?: number } = {},
): Promise<AuditFeedEntry[]> {
  const url = new URL(`${GATEWAY_URL}/audit`);
  if (params.mandate_id) url.searchParams.set("mandate_id", params.mandate_id);
  if (params.event_type) url.searchParams.set("event_type", params.event_type);
  if (params.decision) url.searchParams.set("decision", params.decision);
  if (params.limit) url.searchParams.set("limit", String(params.limit));
  const res = await fetch(url, { headers: authHeader(token), cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to load audit feed: ${res.status}`);
  return res.json();
}

// ─── Audit ───────────────────────────────────────────────────────────────

export async function fetchAuditTrail(
  mandateId: string,
  token?: string,
): Promise<AuditEntry[]> {
  const res = await fetch(`${GATEWAY_URL}/audit/${mandateId}`, {
    headers: authHeader(token),
    cache: "no-store",
  });
  if (!res.ok) throw new Error(`Failed to fetch audit trail: ${res.status}`);
  const data = await res.json();
  return data.entries || [];
}

export async function rejectMandate(
  mandateId: string,
  token?: string,
): Promise<{ mandate_id: string; status: string }> {
  const res = await fetch(`${GATEWAY_URL}/mandate/revoke`, {
    method: "POST",
    headers: { "Content-Type": "application/json", ...authHeader(token) },
    body: JSON.stringify({ mandate_id: mandateId, reason: "rejected by merchant via dashboard" }),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    throw new Error(body?.error || `Rejection failed: ${res.status}`);
  }
  return res.json();
}

export async function approveMandate(
  _mandateId: string,
): Promise<{ mandate_id: string; status: string }> {
  // Mandates are issued as Active — approval is implicit.
  // The consent page lets the merchant review before the agent can spend.
  return { mandate_id: _mandateId, status: "active" };
}
