"""Intent parsing for the buyer agent.

The LLM is used ONLY to choose *which* product satisfies a natural-language
brief (e.g. "a wireless mouse under 1500 for travel"). The final purchase
decision — mandate, policy gate, Razorpay call, refund — is made by the
gateway's deterministic engine, never by the model. This satisfies the
project rule: no LLM ever makes a money decision, and any model used is an
OpenRouter/opencode model, not OpenAI.

If ``OPENROUTER_API_KEY`` is not configured the agent transparently falls
back to a rule-based selection, so it always works with zero external deps.
"""
import json
import os
from typing import Optional, Tuple

import http.client

OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"
DEFAULT_MODEL = "meta-llama/llama-3.1-8b-instruct:free"

SYSTEM_PROMPT = (
    "You are the intent parser for a buyer agent in an agentic-commerce "
    "system. You are given a product catalog and a shopper brief. Choose the "
    "single best-matching in-stock product that fits the brief and is within "
    "budget. You must respect the budget strictly. Respond with ONLY strict "
    'JSON of the form {"product_id": "<id>", "reason": "<short reason>"}.'
)


def _rule_based(catalog, budget, category):
    cands = [
        p
        for p in catalog
        if p.get("category") == category
        and p["price"] <= budget
        and p.get("availability") == "in_stock"
    ]
    if not cands:
        return None, "rule-based", "no in-stock product matched the category/budget"
    p = min(cands, key=lambda p: p["price"])
    return (
        p,
        "rule-based",
        f"cheapest in-category in-stock match at Rs{p['price']/100:.2f}",
    )


def _catalog_block(catalog, budget, category):
    lines = []
    for p in catalog:
        if p.get("category") != category:
            continue
        lines.append(
            f"- id={p['product_id']} | {p['title']} | "
            f"Rs{p['price']/100:.2f} | {p['availability']}"
        )
    return "\n".join(lines) or "(no products in this category)"


def _llm_select(catalog, api_key, budget, category, brief):
    model = os.getenv("OPENROUTER_MODEL", DEFAULT_MODEL)
    user = (
        f"Shopper brief: {brief}\n"
        f"Budget: Rs{budget/100:.2f} (hard limit).\n"
        f"Category filter: {category}\n"
        f"Catalog (category {category}):\n{_catalog_block(catalog, budget, category)}\n"
        "Return the single best product_id as JSON."
    )
    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user},
        ],
        "response_format": {"type": "json_object"},
        "temperature": 0.2,
    }
    parsed = http.client.urlsplit(OPENROUTER_URL)
    conn = http.client.HTTPSConnection(parsed.netloc, timeout=20)
    conn.request(
        "POST",
        parsed.path,
        body=json.dumps(body),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
            "HTTP-Referer": "https://mandate-gateway.local",
            "X-Title": "Mandate Gateway Buyer Agent",
        },
    )
    resp = conn.getresponse()
    raw = resp.read().decode()
    conn.close()
    if resp.status != 200:
        raise RuntimeError(f"openrouter {resp.status}: {raw[:200]}")
    data = json.loads(raw)
    content = data["choices"][0]["message"]["content"]
    guess = json.loads(content)
    pid = guess.get("product_id")
    product = next((p for p in catalog if p["product_id"] == pid), None)
    if not product:
        raise RuntimeError(f"llm returned unknown product_id {pid}")
    if product["price"] > budget:
        raise RuntimeError("llm breached budget")
    if product.get("availability") != "in_stock":
        raise RuntimeError("llm chose out-of-stock item")
    return product, "llm", guess.get("reason", "llm match within budget")


def select_product(
    catalog, *, budget: int, category: str, brief: Optional[str] = None
) -> Tuple[Optional[dict], str, str]:
    """Return (product, method, reasoning). Never raises — degrades safely."""
    api_key = os.getenv("OPENROUTER_API_KEY")
    if api_key:
        try:
            return _llm_select(catalog, api_key, budget, category, brief or "")
        except Exception as exc:  # model must never break the purchase loop
            p, method, why = _rule_based(catalog, budget, category)
            return p, method, f"llm unavailable ({exc}); {why}"
    return _rule_based(catalog, budget, category)
