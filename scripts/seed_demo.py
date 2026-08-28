#!/usr/bin/env python3
"""Seed a rich, idempotent demo into a running Mandate Gateway stack.

Creates a demo merchant mapped to the live catalog owner (merchant-001) so its
dashboard shows the continuous buyer-agent's purchases, mandates and audit; a
demo admin; a scoped agent key; one immediate purchase; and one reconciliation
mismatch so the Reconciliation view is never empty. Safe to re-run.

Run AFTER `docker compose up`:
    python3 scripts/seed_demo.py
"""
import http.client
import json
import os
import sys

GW = os.getenv("GATEWAY_URL", "http://localhost:8000")
DEMO_EMAIL = "demo@merchant.local"
DEMO_PASS = "Demo@1234"
ADMIN_EMAIL = "admin@merchant.local"
ADMIN_PASS = "Admin@1234"


def req(method, path, body=None, token=None, timeout=15):
    parsed = GW.replace("http://", "")
    host, _, port = parsed.partition(":")
    port = int(port) if port else 80
    conn = http.client.HTTPConnection(host, port, timeout=timeout)
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    conn.request(method, path, body=json.dumps(body) if body else None, headers=headers)
    r = conn.getresponse()
    data = r.read()
    conn.close()
    try:
        return json.loads(data) if data else {}
    except Exception:
        return {"_raw": data.decode(errors="replace")}


def auth(email, password, role, tenant_id=None):
    body = {"role": role, "email": email, "name": f"Demo {role.title()}", "password": password}
    if tenant_id:
        body["tenant_id"] = tenant_id
    r = req("POST", "/auth/signup", body)
    if "token" in r:
        return r["token"]
    r = req("POST", "/auth/login", {"email": email, "password": password})
    if "token" not in r:
        print(f"[seed] auth failed for {email}: {r}")
        sys.exit(1)
    return r["token"]


def main():
    print(f"[seed] Gateway: {GW}")

    m_token = auth(DEMO_EMAIL, DEMO_PASS, "merchant", tenant_id="merchant-001")
    print("[seed] demo merchant ready (tenant merchant-001)")

    a_token = auth(ADMIN_EMAIL, ADMIN_PASS, "admin", tenant_id="admin-demo")
    print("[seed] demo admin ready")

    # Scoped agent key issued BY the merchant (populates Agent Console).
    keys = req("GET", "/agents/keys", token=m_token)
    existing = keys if isinstance(keys, list) else keys.get("keys", [])
    if not any(k.get("label") == "demo-buyer-agent" for k in existing):
        k = req("POST", "/agents/keys",
                {"label": "demo-buyer-agent", "scopes": ["mandate:issue", "purchase:exec"]},
                token=m_token)
        api_key = k.get("key")
        print(f"[seed] merchant agent key issued ({api_key[:12] if api_key else 'n/a'}…)")
    else:
        print("[seed] merchant agent key already exists")
        api_key = None

    # One immediate purchase so the first dashboard load is not empty.
    cat = req("GET", "/catalog")
    prod = next((p for p in cat if p.get("category") == "electronics"
                 and p.get("availability") == "in_stock"), None)
    if prod and api_key:
        mk = req("POST", "/mandate",
                 {"merchant_id": "merchant-001", "max_amount": 50000, "currency": "INR",
                  "scope": [prod["category"]], "frequency": "one_time", "expires_in_hours": 1},
                 token=api_key)
        if "mandate_id" in mk:
            pur = req("POST", "/purchase",
                      {"mandate_id": mk["mandate_id"], "product_id": prod["product_id"],
                       "quantity": 1, "shipping_address": "123 Demo Street, Bangalore",
                       "expected_price": prod["price"]}, token=api_key)
            print(f"[seed] immediate purchase: {pur.get('status')} order {pur.get('order_id')}")
        else:
            print(f"[seed] immediate mandate failed: {mk}")

    # Reconciliation mismatch (admin) only if none exist yet.
    mm = req("GET", "/reconciliation/mismatches", token=a_token)
    if isinstance(mm, list) and len(mm) == 0:
        d = req("POST", "/admin/simulate-drift", {}, token=a_token)
        print(f"[seed] reconciliation mismatch seeded: {d.get('mismatch_id', d)}")
    else:
        print("[seed] mismatch already present — skipped")

    print("\nDONE. Open the dashboard and sign in with:")
    print(f"   Merchant demo : {DEMO_EMAIL} / {DEMO_PASS}")
    print(f"   Admin demo    : {ADMIN_EMAIL} / {ADMIN_PASS}")
    print("The continuous buyer-agent keeps adding live purchases every ~30s.")


if __name__ == "__main__":
    main()
