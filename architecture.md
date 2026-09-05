# Architecture

## Trust Flow

```mermaid
flowchart LR
    subgraph AGENT["AI BUYER AGENT"]
        A1["Buyer Agent<br/><i>discover · pick · mandate</i>"]
    end

    subgraph GATEWAY["MANDATE GATEWAY"]
        M["Mandate Engine<br/><i>issue · sign · verify</i>"]
        P["Policy Engine<br/><i>allow / block / escalate</i>"]
        RZ["Razorpay Client<br/><i>order · capture · refund</i>"]
        RC["Reconciliation<br/><i>intent vs outcome · recover</i>"]
    end

    subgraph CATALOG["CATALOG"]
        CAT1["Catalog + Manifest<br/><i>ACP product feed</i>"]
    end

    subgraph DATA["DATA LAYER"]
        DB[("PostgreSQL<br/><i>audit log · mandates</i>")]
        RED[("Redis<br/><i>agent task queue</i>")]
    end

    A1 -->|"GET /catalog"| CAT1
    A1 -->|"POST /mandate"| M
    M -->|"verify signature"| P
    P -->|"allow (deterministic)"| RZ
    P -->|"block to audit"| DB
    RZ -->|"settle (test mode)"| D1["Razorpay<br/><i>test-mode API</i>"]
    RZ -->|"write audit"| DB
    RC -->|"compare intent vs outcome"| RZ
    RC -->|"auto-refund"| RZ
    DB -->|"stream audit"| DW["Dashboard<br/><i>Next.js live audit</i>"]

    style A1 fill:#0d1730,stroke:#fbbf24,stroke-width:1.5,color:#e6ebf5
    style M fill:#06251c,stroke:#10b981,stroke-width:2,color:#e6ebf5
    style P fill:#06251c,stroke:#10b981,stroke-width:2,color:#e6ebf5
    style RZ fill:#3b0d13,stroke:#f87171,stroke-width:2,color:#e6ebf5
    style RC fill:#1a1038,stroke:#a78bfa,stroke-width:2,color:#e6ebf5
    style CAT1 fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style DB fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style RED fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style D1 fill:#3b2c06,stroke:#fbbf24,stroke-width:1.5,color:#e6ebf5
    style DW fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style AGENT fill:transparent,stroke:#fbbf24,stroke-dasharray:4 4,stroke-width:1,color:#fbbf24
    style GATEWAY fill:transparent,stroke:#10b981,stroke-width:1.2,color:#10b981
    style CATALOG fill:transparent,stroke:#60a5fa,stroke-dasharray:4 4,stroke-width:1,color:#60a5fa
    style DATA fill:transparent,stroke:#60a5fa,stroke-dasharray:4 4,stroke-width:1,color:#60a5fa
```

## System Stack

```mermaid
flowchart TB
    subgraph BROWSER["PUBLIC / BROWSER"]
        B1["Merchant Dashboard<br/><i>localhost:3000</i>"]
        B2["Buyer Agent<br/><i>Python worker</i>"]
    end

    subgraph DOCKER["DOCKER COMPOSE"]
        S1["gateway<br/><i>axum Rust :8000</i>"]
        S2["dashboard<br/><i>Next.js TS :3000</i>"]
        S3["buyer-agent<br/><i>Python queue worker</i>"]
        S4["postgres<br/><i>PostgreSQL :5432</i>"]
        S5["redis<br/><i>Redis :6379</i>"]
    end

    SUB["Razorpay<br/><i>test-mode API</i>"]
    KEY["Ed25519 key<br/><i>MANDATE_MASTER_KEY</i>"]

    B1 -->|"HTTPS :3000"| S2
    B2 -->|"API :8000"| S1
    S2 -->|"gateway :8000"| S1
    S3 -->|"consume tasks"| S5
    S3 -->|"API :8000"| S1
    S1 -->|"orders / refunds"| SUB
    S1 -->|"SQL :5432"| S4
    S1 -->|"sign + verify"| KEY

    style S1 fill:#06251c,stroke:#10b981,stroke-width:2,color:#e6ebf5
    style S2 fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style S3 fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style S4 fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style S5 fill:#0d1730,stroke:#60a5fa,stroke-width:1.5,color:#e6ebf5
    style SUB fill:#3b2c06,stroke:#fbbf24,stroke-width:1.5,color:#e6ebf5
    style KEY fill:#3b2c06,stroke:#fbbf24,stroke-width:1.5,color:#e6ebf5
    style B1 fill:#0d1730,stroke:#fbbf24,stroke-width:1,color:#fbbf24
    style B2 fill:#0d1730,stroke:#fbbf24,stroke-width:1,color:#fbbf24
    style BROWSER fill:transparent,stroke:#fbbf24,stroke-dasharray:4 4,color:#fbbf24
    style DOCKER fill:transparent,stroke:#60a5fa,stroke-width:1.2,color:#60a5fa
```

## Components

| Component | Crate | Purpose |
|---|---|---|
| Mandate Engine | `gateway/crates/mandate-engine` | Ed25519 signing, verification, nonce replay protection |
| Policy Engine | `gateway/crates/policy-engine` | Deterministic allow/block/escalate, zero AI dependencies |
| Razorpay Client | `gateway/crates/razorpay-client` | Orders, Payments, Refunds API (test mode) |
| Catalog Service | `gateway/services/catalog-service` | Unified gateway server, 21 API routes |
| Reconciliation | `gateway/services/reconciliation` | Intent vs outcome mismatch detection and auto refund |
| Dashboard | `dashboard/` | Next.js 14 merchant app, 13 pages |
| Buyer Agent | `buyer-agent/` | Python queue worker, self drives every 30s |
