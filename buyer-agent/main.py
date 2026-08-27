"""
Reference Buyer Agent — thin demo harness.

Discovers a merchant, picks something within budget, requests a mandate,
and completes a purchase. LLM is used ONLY for intent parsing ("pick something
under ₹500 in category X") — never for the purchase decision itself.
"""

import json
import os
import http.client

GATEWAY_URL = os.getenv("GATEWAY_URL", "http://localhost:8000")
BUDGET = int(os.getenv("BUDGET", "50000"))  # paise
CATEGORY = os.getenv("CATEGORY", "electronics")
USER_ID = os.getenv("USER_ID", "buyer-agent-001")
MERCHANT_ID = os.getenv("MERCHANT_ID", "merchant-001")
SHIPPING_ADDRESS = os.getenv("SHIPPING_ADDRESS", "123 Demo Street, Bangalore")


def _request(method, path, body=None):
    """Helper to make HTTP requests to the gateway."""
    parsed = GATEWAY_URL.replace("http://", "")
    host, _, port = parsed.partition(":")
    port = int(port) if port else 80

    conn = http.client.HTTPConnection(host, port)
    headers = {"Content-Type": "application/json"} if body else {}
    conn.request(method, path, body=json.dumps(body) if body else None, headers=headers)
    resp = conn.getresponse()
    data = resp.read()
    conn.close()
    return json.loads(data)


def discover_merchant():
    """Fetch merchant manifest and catalog."""
    manifest = _request("GET", "/manifest")
    catalog = _request("GET", "/catalog")
    return manifest, catalog


def pick_product(catalog, budget, category):
    """Simple rule-based product selection (no LLM)."""
    candidates = [
        p for p in catalog
        if p["category"] == category and p["price"] <= budget and p["availability"] == "in_stock"
    ]
    if not candidates:
        return None
    return min(candidates, key=lambda p: p["price"])


def request_mandate(product):
    """Request a mandate for the selected product."""
    body = {
        "user_id": USER_ID,
        "merchant_id": MERCHANT_ID,
        "buyer_agent_id": "agent-001",
        "max_amount": BUDGET,
        "currency": "INR",
        "scope": [product["category"]],
        "frequency": "one_time",
        "expires_in_hours": 1,
    }
    return _request("POST", "/mandate", body)


def execute_purchase(mandate, product):
    """Execute the purchase via the gateway."""
    body = {
        "mandate_id": mandate["mandate_id"],
        "product_id": product["product_id"],
        "quantity": 1,
        "shipping_address": SHIPPING_ADDRESS,
        "expected_price": product["price"],
    }
    return _request("POST", "/purchase", body)


def main():
    print(f"[buyer-agent] Discovering merchant at {GATEWAY_URL}...")
    manifest, catalog = discover_merchant()
    print(f"[buyer-agent] Merchant: {manifest['merchant_name']}")
    print(f"[buyer-agent] Catalog: {len(catalog)} products")

    print(f"[buyer-agent] Picking product in '{CATEGORY}' under ₹{BUDGET // 100}...")
    product = pick_product(catalog, BUDGET, CATEGORY)
    if not product:
        print("[buyer-agent] No matching product found. Exiting.")
        return

    print(f"[buyer-agent] Selected: {product['title']} @ ₹{product['price'] // 100}")

    print("[buyer-agent] Requesting mandate...")
    mandate = request_mandate(product)
    if "error" in mandate:
        print(f"[buyer-agent] Mandate failed: {mandate['error']}")
        return
    print(f"[buyer-agent] Mandate issued: {mandate['mandate_id']}")

    print("[buyer-agent] Executing purchase...")
    result = execute_purchase(mandate, product)
    if "error" in result:
        print(f"[buyer-agent] Purchase failed: {result['error']}")
        return
    print(f"[buyer-agent] Purchase complete: {result['status']} (order: {result['order_id']})")


if __name__ == "__main__":
    main()
