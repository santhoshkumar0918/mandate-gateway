"""
Continuous buyer-agent worker.

Runs the agent as a managed background process instead of a one-shot script.
In production it consumes scheduled tasks from a Redis-backed queue
(``REDIS_URL`` set) and posts step events to a Redis stream ``agent:events``
so the dashboard can show agents operating live. When ``REDIS_URL`` is unset
(local dev / demo), it falls back to a timed loop that performs the same
authenticated discover→mandate→purchase flow, which we can verify without
Redis running.

Every run authenticates with a provisioned, scoped API key (ticket 04) — never
a hardcoded identity. A crash-safe processed-task set gives idempotency.
"""

import json
import os
import http.client
import signal
import sys
import time
import uuid

GATEWAY_URL = os.getenv("GATEWAY_URL", "http://localhost:8000")
REDIS_URL = os.getenv("REDIS_URL")
BUDGET = int(os.getenv("BUDGET", "50000"))
CATEGORY = os.getenv("CATEGORY", "electronics")
MERCHANT_ID = os.getenv("MERCHANT_ID", "merchant-001")
SHIPPING_ADDRESS = os.getenv("SHIPPING_ADDRESS", "123 Demo Street, Bangalore")

AGENT_EMAIL = os.getenv("AGENT_EMAIL", "agent-001@buyer.local")
AGENT_TENANT = os.getenv("AGENT_TENANT", "agent-001")
AGENT_PASSWORD = os.getenv("AGENT_PASSWORD", "buyer-agent-secret")
AGENT_API_KEY = os.getenv("AGENT_API_KEY")  # optional pre-provisioned key

LOOP_INTERVAL = int(os.getenv("LOOP_INTERVAL", "30"))
QUEUE_NAME = os.getenv("AGENT_QUEUE", "agent:tasks")
EVENT_STREAM = os.getenv("AGENT_EVENT_STREAM", "agent:events")

_shutdown = False


def _request(method, path, body=None, token=None):
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
    return json.loads(data) if data else {}


def authenticate() -> str:
    """Return a scoped API key, provisioning one on first run."""
    if AGENT_API_KEY:
        return AGENT_API_KEY
    resp = _request(
        "POST",
        "/auth/signup",
        {
            "role": "agent",
            "email": AGENT_EMAIL,
            "name": "Continuous Buyer Agent",
            "password": AGENT_PASSWORD,
            "tenant_id": AGENT_TENANT,
        },
    )
    jwt = resp.get("token") or _request(
        "POST",
        "/auth/login",
        {"email": AGENT_EMAIL, "password": AGENT_PASSWORD},
    )["token"]
    key = _request(
        "POST",
        "/agents/keys",
        {"label": "worker-key", "scopes": ["mandate:issue", "purchase:exec"]},
        token=jwt,
    )["key"]
    return key


def run_once(api_key: str) -> list[dict]:
    """Execute one discover→mandate→purchase run; return step events."""
    events: list[dict] = []
    manifest = _request("GET", "/manifest")
    catalog = _request("GET", "/catalog")
    events.append({"step": "discover", "merchant": manifest.get("merchant_name")})

    candidates = [
        p for p in catalog
        if p["category"] == CATEGORY and p["price"] <= BUDGET and p["availability"] == "in_stock"
    ]
    if not candidates:
        events.append({"step": "pick", "result": "no_match"})
        return events
    product = min(candidates, key=lambda p: p["price"])
    events.append({"step": "pick", "product": product["title"]})

    mandate = _request(
        "POST",
        "/mandate",
        {
            "merchant_id": MERCHANT_ID,
            "max_amount": BUDGET,
            "currency": "INR",
            "scope": [product["category"]],
            "frequency": "one_time",
        },
        token=api_key,
    )
    if "error" in mandate:
        events.append({"step": "mandate", "error": mandate["error"]})
        return events
    events.append({"step": "mandate", "mandate_id": mandate["mandate_id"]})

    result = _request(
        "POST",
        "/purchase",
        {
            "mandate_id": mandate["mandate_id"],
            "product_id": product["product_id"],
            "quantity": 1,
            "shipping_address": SHIPPING_ADDRESS,
            "expected_price": product["price"],
        },
        token=api_key,
    )
    events.append({"step": "purchase", "status": result.get("status"), "order_id": result.get("order_id")})
    return events


def emit(events: list[dict], task_id: str):
    if REDIS_URL:
        try:
            import redis

            r = redis.Redis.from_url(REDIS_URL)
            for e in events:
                r.xadd(EVENT_STREAM, {"task_id": task_id, "event": json.dumps(e)})
        except Exception as exc:  # never block the worker on emit failure
            print(f"[worker] event emit failed: {exc}", file=sys.stderr)
    for e in events:
        print(f"[worker] {e}")


def _processed_seen(r, task_id: str) -> bool:
    if not REDIS_URL:
        return False
    return r.sismember("agent:processed", task_id)


def _mark_processed(r, task_id: str):
    if REDIS_URL:
        r.sadd("agent:processed", task_id)


def loop_mode(api_key: str):
    """Local/dev mode: repeated authenticated runs on an interval."""
    print(f"[worker] loop mode (interval={LOOP_INTERVAL}s)")
    while not _shutdown:
        task_id = str(uuid.uuid4())
        events = run_once(api_key)
        emit(events, task_id)
        if _shutdown:
            break
        time.sleep(LOOP_INTERVAL)


def queue_mode(api_key: str):
    """Production mode: consume tasks from a Redis queue, idempotent."""
    import redis

    r = redis.Redis.from_url(REDIS_URL)
    print(f"[worker] queue mode consuming {QUEUE_NAME}")
    while not _shutdown:
        item = r.blpop(QUEUE_NAME, timeout=5)
        if not item:
            continue
        task = json.loads(item[1])
        task_id = task.get("task_id", str(uuid.uuid4()))
        if _processed_seen(r, task_id):
            print(f"[worker] skipping already-processed {task_id}")
            continue
        events = run_once(api_key)
        emit(events, task_id)
        _mark_processed(r, task_id)


def main():
    def handle_stop(signum, frame):
        global _shutdown
        _shutdown = True
        print("[worker] shutting down gracefully…")

    signal.signal(signal.SIGINT, handle_stop)
    signal.signal(signal.SIGTERM, handle_stop)

    api_key = authenticate()
    print(f"[worker] authenticated (key {api_key[:8]}…)")

    if REDIS_URL:
        queue_mode(api_key)
    else:
        loop_mode(api_key)


if __name__ == "__main__":
    main()
