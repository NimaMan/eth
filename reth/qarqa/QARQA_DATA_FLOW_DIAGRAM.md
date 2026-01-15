# QARQA Data Flow - Visual Architecture

## 🏗️ **System Architecture Diagram**

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                   QARQA ARCHITECTURE                                 │
└─────────────────────────────────────────────────────────────────────────────────────┘

┌─────────────────┐
│  User Input     │
│  TX Hash:       │
│  0xf7bd63...    │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              DATA ACCESS LAYER                                       │
│  ┌─────────────────┐  ┌─────────────────────┐  ┌──────────────────────┐           │
│  │ Transaction     │  │ Internal Transfers  │  │ Token Transfers      │           │
│  │ Fetcher         │  │ Fetcher            │  │ Fetcher              │           │
│  └────────┬────────┘  └─────────┬───────────┘  └──────────┬───────────┘           │
│           │                     │                          │                        │
│           ▼                     ▼                          ▼                        │
│  ┌─────────────────────────────────────────────────────────────────────┐           │
│  │                        PostgreSQL Database                           │           │
│  │  ┌──────────────┐  ┌───────────────────┐  ┌──────────────────┐    │           │
│  │  │ transactions │  │ internal_transfers│  │ token_transfers  │    │           │
│  │  └──────────────┘  └───────────────────┘  └──────────────────┘    │           │
│  └─────────────────────────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                           TRANSACTION ENRICHMENT                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐           │
│  │  Complete Transaction Object                                         │           │
│  │  ┌─────────────┐  ┌──────────────────┐  ┌────────────────────┐    │           │
│  │  │ Basic Data  │  │ Internal ETH:    │  │ Token Transfers:   │    │           │
│  │  │ • From      │  │ • WETH → 0x6b.. │  │ • V4 → 0x6b: USDC │    │           │
│  │  │ • To        │  │ • 0x6b → V4     │  │ • 0xfB → V3: WETH │    │           │
│  │  │ • Value     │  │ • WETH → 0x31.. │  │ • V3 → 0xfB: USDT │    │           │
│  │  │ • Gas       │  │ • 0x31 → V4     │  │ • ... (8 total)   │    │           │
│  │  └─────────────┘  └──────────────────┘  └────────────────────┘    │           │
│  └─────────────────────────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                          TX SIMULATION LAYER                                         │
│  ┌──────────────────────────┐        ┌──────────────────────────────────┐          │
│  │  Fund Flow Analyzer      │        │  State Change Analyzer          │          │
│  │  ┌───────────────────┐   │        │  ┌────────────────────────────┐ │          │
│  │  │ Extract Flows:    │   │        │  │ Calculate Net Changes:     │ │          │
│  │  │ • User → Network │   │        │  │ • User: -0.0039 ETH       │ │          │
│  │  │ • WETH → V4     │   │        │  │ • WETH: -16.325 ETH       │ │          │
│  │  │ • V4 → V3       │   │        │  │ • V4: +16.3 ETH, -40k USDC│ │          │
│  │  │ • V3 → WETH     │   │        │  │ • Router: -23 WETH +27k...│ │          │
│  │  └───────────────────┘   │        │  └────────────────────────────┘ │          │
│  └──────────────────────────┘        └──────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                         NETWORK BUILDING LAYER                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐           │
│  │                     Network Construction Pipeline                     │           │
│  │                                                                       │           │
│  │  1. Create Nodes          2. Create Edges         3. Filter & Optimize          │
│  │  ┌──────────────┐        ┌──────────────┐        ┌──────────────────┐         │
│  │  │ 👤 User      │        │ User→Network │        │ ❌ Intermediaries│         │
│  │  │ ⛏️ Network   │        │ WETH→V4      │        │ ✅ WETH+ETH     │         │
│  │  │ 🪙 WETH      │  ───▶  │ V4→V3        │  ───▶  │ ✅ Meaningful   │         │
│  │  │ 🏊 V4 Pool   │        │ V3→WETH      │        │    Nodes Only   │         │
│  │  │ 🏊 V3 Pool   │        │              │        │                  │         │
│  │  │ + 3 others   │        │              │        │                  │         │
│  │  └──────────────┘        └──────────────┘        └──────────────────┘         │
│  └─────────────────────────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                         VISUALIZATION EXPORT                                         │
│  ┌──────────────────────────┐        ┌──────────────────────────────────┐          │
│  │  Cytoscape Format        │        │  Network Statistics              │          │
│  │  {                       │        │  • Total Nodes: 8                │          │
│  │    "nodes": [...],       │        │  • Total Edges: 7                │          │
│  │    "edges": [...],       │        │  • ETH Volume: 23.055            │          │
│  │    "style": [...]        │        │  • Complexity: High              │          │
│  │  }                       │        │  • Intermediaries: 0             │          │
│  └──────────────────────────┘        └──────────────────────────────────┘          │
└─────────────────────────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                            FRONTEND DISPLAY                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐           │
│  │                    Cytoscape.js Network Graph                        │           │
│  │                                                                       │           │
│  │     👤 User ─────0.0039 ETH────▶ ⛏️ Network                          │           │
│  │        │                                                              │           │
│  │        │                         🏊 V4 Pool                          │           │
│  │        │                       ╱            ╲                        │           │
│  │     🪙 WETH ◀──────────────────              ╲                       │           │
│  │        │         16.325 ETH      40k USDC     ╲                      │           │
│  │        │                         13k USDT      ╲                     │           │
│  │        │                                        ▼                     │           │
│  │        └────────6.729 WETH──────────────▶ 🏊 V3 Pool                 │           │
│  │                                                                       │           │
│  └─────────────────────────────────────────────────────────────────────┘           │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

## 📊 **Data Structure Evolution**

### **1. Raw Database Records**
```
transactions:
├── hash: 0xf7bd63...
├── from: 0x5B43453F...
├── to: 0xfBd4cdB4...
├── value: 22646153 (wei)
└── gas_used: 287456

internal_transfers:
├── [WETH → 0x6bDf35, 10.829 ETH]
├── [0x6bDf35 → V4, 10.829 ETH]
├── [WETH → 0x3177F6, 5.496 ETH]
└── [0x3177F6 → V4, 5.496 ETH]

token_transfers:
├── [V4 → 0x6bDf35, 27,158 USDC]
├── [0x6bDf35 → Router, 27,158 USDC]
├── [Router → 0x6bDf35, 10.829 WETH]
└── ... (5 more)
```

### **2. Enriched Transaction Object**
```rust
Transaction {
    hash: 0xf7bd63...,
    from_address: User,
    to_address: Router,
    value: 0.000000000022646153 ETH,
    gas_used: 287456,
    internal_transfers: Vec<EthMovement>[4],
    token_transfers: Vec<TokenMovement>[8],
    timestamp: 1698765432,
}
```

### **3. Fund Flows**
```rust
Vec<FundFlow> [
    FundFlow { from: User, to: Network, eth: 0.0039, type: "Gas" },
    FundFlow { from: WETH, to: V4, eth: 16.325, type: "ETH" },
    FundFlow { from: V4, to: V3, tokens: [USDC, USDT], type: "Stablecoin" },
    FundFlow { from: V3, to: WETH, tokens: [WETH: 6.729], type: "Token" },
]
```

### **4. State Changes**
```rust
HashMap<Address, StateChange> {
    User: StateChange { eth: -0.0039, tokens: {} },
    WETH: StateChange { eth: -16.325, tokens: {} },
    V4: StateChange { eth: +16.325, tokens: {USDC: -40930, USDT: -13772} },
    V3: StateChange { eth: 0, tokens: {WETH: +6.729, USDT: -16865} },
    Router: StateChange { eth: 0, tokens: {WETH: -23.055, USDC: +27158, USDT: +30637} },
    // ... 3 more addresses
}
```

### **5. Network Graph**
```rust
Network {
    nodes: Vec<NetworkNode> [
        NetworkNode { address: User, type: EOA, net_change: -0.0039 },
        NetworkNode { address: WETH, type: Token, net_change: -16.325 },
        NetworkNode { address: V4, type: Pool, net_change: Complex },
        // ... 5 more nodes
    ],
    edges: Vec<NetworkEdge> [
        NetworkEdge { from: User, to: Network, amount: 0.0039, type: Gas },
        NetworkEdge { from: WETH, to: V4, amount: 16.325, type: ETH },
        // ... more edges
    ]
}
```

### **6. Final Visualization Data**
```json
{
    "elements": {
        "nodes": [
            {"data": {"id": "0x5B43...", "label": "User", "type": "user"}},
            {"data": {"id": "WETH", "label": "WETH Contract", "type": "token"}},
            // ... more nodes
        ],
        "edges": [
            {"data": {"source": "0x5B43...", "target": "0x0000...", "amount": 0.0039}},
            {"data": {"source": "WETH", "target": "V4", "amount": 16.325}},
            // ... more edges
        ]
    },
    "stats": {
        "total_nodes": 8,
        "total_edges": 7,
        "total_volume": 23.055,
        "has_intermediaries": false
    }
}
```

## 🔑 **Key Processing Steps**

### **Step 1: Data Collection**
- Fetch transaction, internal transfers, token transfers
- Join with token metadata (symbols, decimals)
- Sort by transfer index for correct ordering

### **Step 2: Flow Extraction**
- Group transfers by (from, to) pairs
- Aggregate amounts for same pairs
- Classify transfer types (ETH, token, gas)

### **Step 3: State Calculation**
- For each address: sum inflows - sum outflows
- Track each asset type separately
- Apply decimal adjustments for tokens

### **Step 4: Network Building**
- Create node for each address with state change
- Create edge for each fund flow
- Apply entity classification (user, pool, token, etc.)

### **Step 5: Optimization**
- Filter zero-change intermediaries
- Combine WETH amounts with ETH
- Optimize graph layout for clarity

### **Step 6: Export**
- Convert to Cytoscape.js format
- Calculate network statistics
- Prepare for frontend rendering

This architecture ensures accurate, fast, and visually clear representation of complex blockchain transactions!