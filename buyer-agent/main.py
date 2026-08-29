"""
Reference Buyer Agent — thin demo harness.

Discovers a merchant, picks something within budget, requests a mandate, and
completes a purchase. LLM is used ONLY for intent parsing ("pick something
under ₹500 in category X") — never for the purchase decision itself.

Auth flow (product-grade): the agent first signs up as an `agent` account,
issues a scoped API key (mandate:issue + purchase:exec), then authenticates
every money-moving call with that key. No money endpoint is called
unauthenticated.
"""

import json
import os
import http.client

import intent

GATEWAY_URL = os.getenv("GATEWAY_URL", "http://localhost:8000")
BUDGET = int(os.getenv("BUDGET", "50000"))  # paise
CATEGORY = os.getenv("CATEGORY", "electronics")
MERCHANT_ID = os.getenv("MERCHANT_ID", "merchant-001")
SHIPPING_ADDRESS = os.getenv("SHIPPING_ADDRESS", "123 Demo Street, Bangalore")
AGENT_EMAIL = os.getenv("AGENT_EMAIL", "agent-001@buyer.local")
AGENT_TENANT = os.getenv("AGENT_TENANT", "agent-001")
AGENT_PASSWORD = os.getenv("AGENT_PASSWORD", "buyer-agent-secret")
AGENT_GOAL = os.getenv(
    "AGENT_GOAL", f"Pick a good {CATEGORY} product that fits the shopper's needs and budget"
)


def _request(method, path, body=None, token=None):
    """Helper to make HTTP requests to the gateway."""
    parsed = GATEWAY_URL.replace("http://", "")
    host, _, port = parsed.partition(":")
    port = int(port) if port else 80

    conn = http.client.HTTPConnection(host, port)
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    conn.request(method, path, body=json.dumps(body) if body else None, headers=headers)
    resp = conn.getresponse()
    data = resp.read()
    conn.close()
    return json.loads(data)


def discover_merchant():
    """Fetch merchant manifest and catalog (public endpoints)."""
    manifest = _request("GET", "/manifest")
    catalog = _request("GET", "/catalog")
    return manifest, catalog


def signup_agent():
    """Create an agent account (no-op if it already exists), then log in."""
    resp = _request(
        "POST",
        "/auth/signup",
        {
            "role": "agent",
            "email": AGENT_EMAIL,
            "name": "Demo Buyer Agent",
            "password": AGENT_PASSWORD,
            "tenant_id": AGENT_TENANT,
        },
    )
    if "token" in resp:
        return resp["token"]
    # Account exists (or already provisioned) — log in instead.
    login = _request(
        "POST", "/auth/login", {"email": AGENT_EMAIL, "password": AGENT_PASSWORD}
    )
    if "token" not in login:
        raise RuntimeError(f"agent auth failed: {login}")
    return login["token"]


def issue_api_key(jwt):
    """Request a scoped API key (mandate:issue + purchase:exec)."""
    resp = _request(
        "POST",
        "/agents/keys",
        {"label": "demo-buyer-key", "scopes": ["mandate:issue", "purchase:exec"]},
        token=jwt,
    )
    if "key" not in resp:
        raise RuntimeError(f"key issuance failed: {resp}")
    return resp["key"]


def request_mandate(api_key, product):
    """Request a mandate for the selected product (key-authenticated)."""
    body = {
        "merchant_id": MERCHANT_ID,
        "max_amount": BUDGET,
        "currency": "INR",
        "scope": [product["category"]],
        "frequency": "one_time",
        "expires_in_hours": 1,
    }
    return _request("POST", "/mandate", body, token=api_key)


def execute_purchase(api_key, mandate, product):
    """Execute the purchase via the gateway (key-authenticated)."""
    body = {
        "mandate_id": mandate["mandate_id"],
        "product_id": product["product_id"],
        "quantity": 1,
        "shipping_address": SHIPPING_ADDRESS,
        "expected_price": product["price"],
    }
    return _request("POST", "/purchase", body, token=api_key)


def pick_product(catalog, budget, category):
    """Agentic selection: LLM intent parsing with rule-based fallback."""
    product, how = intent.select_product(
        catalog, budget=budget, category=category, brief=AGENT_GOAL
    )
    print(f"[buyer-agent] Selection mode: {how}")
    return product


def main():
    print(f"[buyer-agent] Discovering merchant at {GATEWAY_URL}...")
    manifest, catalog = discover_merchant()
    print(f"[buyer-agent] Merchant: {manifest['merchant_name']}")
    print(f"[buyer-agent] Catalog: {len(catalog)} products")

    print(f"[buyer-agent] Choosing a product for goal: {AGENT_GOAL}")
    product = pick_product(catalog, BUDGET, CATEGORY)
    if not product:
        print("[buyer-agent] No matching product found. Exiting.")
        return

    print(f"[buyer-agent] Selected: {product['title']} @ ₹{product['price'] // 100}")

    print("[buyer-agent] Authenticating as agent and issuing API key...")
    jwt = signup_agent()
    api_key = issue_api_key(jwt)
    print(f"[buyer-agent] Got scoped API key ({api_key[:8]}...)")

    print("[buyer-agent] Requesting mandate...")
    mandate = request_mandate(api_key, product)
    if "error" in mandate:
        print(f"[buyer-agent] Mandate failed: {mandate['error']}")
        return
    print(f"[buyer-agent] Mandate issued: {mandate['mandate_id']}")

    print("[buyer-agent] Executing purchase...")
    result = execute_purchase(api_key, mandate, product)
    if "error" in result:
        print(f"[buyer-agent] Purchase failed: {result['error']}")
        return
    print(f"[buyer-agent] Purchase complete: {result['status']} (order: {result['order_id']})")


if __name__ == "__main__":
    main()
