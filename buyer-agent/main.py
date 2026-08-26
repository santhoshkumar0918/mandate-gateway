"""
Reference Buyer Agent — thin demo harness.

Discovers a merchant, picks something within budget, requests a mandate,
and completes a purchase. LLM is used ONLY for intent parsing ("pick something
under ₹500 in category X") — never for the purchase decision itself.
"""

import os
import http

GATEWAY_URL = os.getenv("GATEWAY_URL", "http://localhost:8000")
BUDGET = int(os.getenv("BUDGET", "50000"))  # paise
CATEGORY = os.getenv("CATEGORY", "electronics")


def discover_merchant():
    """Fetch merchant manifest and catalog."""
    with http.client.HTTPConnection(GATEWAY_URL.replace("http://", "")) as conn:
        conn.request("GET", "/manifest")
        manifest = conn.getresponse().read()

        conn.request("GET", "/catalog")
        catalog = conn.getresponse().read()

    import json
    return json.loads(manifest), json.loads(catalog)


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
    import json
    with http.client.HTTPConnection(GATEWAY_URL.replace("http://", "")) as conn:
        body = json.dumps({
            "product_id": product["product_id"],
            "amount": product["price"],
            "category": product["category"],
        })
        conn.request("POST", "/mandate", body, {"Content-Type": "application/json"})
        return json.loads(conn.getresponse().read())


def execute_purchase(mandate, product):
    """Execute the purchase via the gateway."""
    import json
    with http.client.HTTPConnection(GATEWAY_URL.replace("http://", "")) as conn:
        body = json.dumps({
            "mandate_id": mandate["mandate_id"],
            "product_id": product["product_id"],
            "amount": product["price"],
        })
        conn.request("POST", "/purchase", body, {"Content-Type": "application/json"})
        return json.loads(conn.getresponse().read())


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
    print(f"[buyer-agent] Mandate issued: {mandate['mandate_id']}")

    print("[buyer-agent] Executing purchase...")
    result = execute_purchase(mandate, product)
    print(f"[buyer-agent] Purchase complete: {result['status']}")


if __name__ == "__main__":
    main()
