#!/usr/bin/env bash
# End-to-end smoke test against the running docker stack (gateway :8000).
# Exercises the full trust-engine path + the engineered-failure recovery.
set -u
G=http://localhost:8000
PASS=0; FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS+1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL+1)); }

echo "== 1. Merchant signup =="
MJWT=$(curl -s -X POST $G/auth/signup -H 'Content-Type: application/json' \
  -d '{"role":"merchant","email":"e2e_m@merchant.local","name":"E2E M","password":"pw","tenant_id":"merchant-001"}' \
  | python3 -c "import sys,json;print(json.load(sys.stdin).get('token',''))")
[ -n "$MJWT" ] && ok "merchant JWT issued" || bad "merchant signup"

echo "== 2. Agent signup + scoped key =="
AJWT=$(curl -s -X POST $G/auth/signup -H 'Content-Type: application/json' \
  -d '{"role":"agent","email":"e2e_a@buyer.local","name":"E2E A","password":"pw","tenant_id":"agent-e2e"}' \
  | python3 -c "import sys,json;print(json.load(sys.stdin).get('token',''))")
AKEY=$(curl -s -X POST $G/agents/keys -H 'Content-Type: application/json' -H "Authorization: Bearer $AJWT" \
  -d '{"label":"e2e","scopes":["mandate:issue","purchase:exec"]}' \
  | python3 -c "import sys,json;print(json.load(sys.stdin).get('key',''))")
[ -n "$AKEY" ] && ok "agent key issued" || bad "agent key"

echo "== 3. Catalog discoverable =="
ELEC=$(curl -s $G/catalog | python3 -c "import sys,json;d=json.load(sys.stdin);p=[x for x in d if x['category']=='electronics'][0];print(p['product_id']);open('/tmp/e2e_price','w').write(str(p['price']))")
P=$(cat /tmp/e2e_price)
[ -n "$ELEC" ] && ok "electronics product $ELEC @ $P" || bad "catalog"

echo "== 4. Mandate issue (scoped) =="
MID=$(curl -s -X POST $G/mandate -H 'Content-Type: application/json' -H "Authorization: Bearer $AKEY" \
  -d "{\"merchant_id\":\"merchant-001\",\"max_amount\":5000000,\"currency\":\"INR\",\"scope\":[\"electronics\"],\"frequency\":\"one_time\"}" \
  | python3 -c "import sys,json;print(json.load(sys.stdin).get('mandate_id',''))")
[ -n "$MID" ] && ok "mandate $MID" || bad "mandate issue"

echo "== 5. Purchase at intent price (expect order) =="
BUY=$(curl -s -X POST $G/purchase -H 'Content-Type: application/json' -H "Authorization: Bearer $AKEY" \
  -d "{\"mandate_id\":\"$MID\",\"product_id\":\"$ELEC\",\"quantity\":1,\"shipping_address\":\"x\",\"expected_price\":$P}")
echo "    $BUY"
echo "$BUY" | grep -q "order_id" && ok "purchase created order" || bad "purchase"

echo "== 6. Policy: wrong-scope mandate rejected =="
MID2=$(curl -s -X POST $G/mandate -H 'Content-Type: application/json' -H "Authorization: Bearer $AKEY" \
  -d "{\"merchant_id\":\"merchant-001\",\"max_amount\":5000000,\"currency\":\"INR\",\"scope\":[\"groceries\"],\"frequency\":\"one_time\"}" \
  | python3 -c "import sys,json;print(json.load(sys.stdin).get('mandate_id',''))")
REJ=$(curl -s -X POST $G/purchase -H 'Content-Type: application/json' -H "Authorization: Bearer $AKEY" \
  -d "{\"mandate_id\":\"$MID2\",\"product_id\":\"$ELEC\",\"quantity\":1,\"shipping_address\":\"x\",\"expected_price\":$P}")
echo "$REJ" | grep -qi "OutOfScope" && ok "out-of-scope purchase blocked" || bad "scope block ($REJ)"

echo "== 7. Auth gating =="
curl -s -o /dev/null -w "    no-key purchase: %{http_code}\n" -X POST $G/purchase -H 'Content-Type: application/json' \
  -d "{\"mandate_id\":\"$MID\",\"product_id\":\"$ELEC\",\"quantity\":1,\"shipping_address\":\"x\",\"expected_price\":$P}"
curl -s -o /dev/null -w "    no-key purchase: %{http_code} (expect 401)\n" -X POST $G/purchase -H 'Content-Type: application/json' \
  -d "{\"mandate_id\":\"$MID\",\"product_id\":\"$ELEC\",\"quantity\":1,\"shipping_address\":\"x\",\"expected_price\":$P}"

echo "== 8. Admin metrics (merchant forbidden, admin ok) =="
curl -s -o /dev/null -w "    merchant -> /admin/metrics: %{http_code} (expect 403)\n" $G/admin/metrics -H "Authorization: Bearer $MJWT"
ADM=$(curl -s -X POST $G/auth/signup -H 'Content-Type: application/json' \
  -d '{"role":"admin","email":"e2e_admin@merchant.local","name":"E2E Admin","password":"pw","tenant_id":"merchant-001"}' \
  | python3 -c "import sys,json;print(json.load(sys.stdin).get('token',''))")
curl -s -o /dev/null -w "    admin -> /admin/metrics: %{http_code} (expect 200)\n" $G/admin/metrics -H "Authorization: Bearer $ADM"

echo "== 9. Engineered failure: drift price, buy at old intent -> mismatch =="
curl -s -X POST $G/admin/simulate-drift -H 'Content-Type: application/json' \
  -d "{\"product_id\":\"$ELEC\",\"new_price\":$((P+500000))}" | python3 -c "import sys,json;print('    drift:',json.load(sys.stdin).get('message',''))"
MID3=$(curl -s -X POST $G/mandate -H 'Content-Type: application/json' -H "Authorization: Bearer $AKEY" \
  -d "{\"merchant_id\":\"merchant-001\",\"max_amount\":5000000,\"currency\":\"INR\",\"scope\":[\"electronics\"],\"frequency\":\"one_time\"}" \
  | python3 -c "import sys,json;print(json.load(sys.stdin).get('mandate_id',''))")
curl -s -X POST $G/purchase -H 'Content-Type: application/json' -H "Authorization: Bearer $AKEY" \
  -d "{\"mandate_id\":\"$MID3\",\"product_id\":\"$ELEC\",\"quantity\":1,\"shipping_address\":\"x\",\"expected_price\":$P}" >/dev/null
sleep 2
MM=$(curl -s $G/reconciliation/mismatches -H "Authorization: Bearer $MJWT" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d))")
[ "${MM:-0}" -gt 0 ] && ok "mismatch detected ($MM)" || bad "mismatch detection"

echo "== 10. Audit trail has entries =="
curl -s "$G/audit?limit=5" -H "Authorization: Bearer $MJWT" | python3 -c "import sys,json;d=json.load(sys.stdin);print('   ',len(d),'recent events')" && ok "audit feed" || bad "audit"

echo ""
echo "RESULT: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] && echo "END-TO-END OK" || echo "END-TO-END HAS FAILURES"
