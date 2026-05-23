# Token Network Visualization Design

> Objective: build an interface that helps understand anything shady that might be going on with a token — hidden connections between addresses, coordinated trading, control structures, and financial anomalies.

---

## 0. Layered Mental Model

The token network is not one graph with one meaning. It is a set of evidence
layers around a token. Each layer answers a different question and has different
confidence semantics.

### First-Order Token Network

This is the graph we can build directly from token-scoped events:

- token contract node;
- pool nodes;
- addresses that hold, transfer, trade, create, control, or provide liquidity;
- tracked-token transfer edges;
- pool trade, pool creation, liquidity, LP, and control edges.

This layer is already useful. If an address transfers the tracked token to
another address, that is direct relationship evidence. If many traders interact
with the same pool, that is pool activity evidence. But this layer can also be
misleading if interpreted as entity clustering: a pool is expected to connect
many unrelated traders, and a router or denomination contract can become a noisy
hub.

### Second-Order Fund-Flow Context

The more informative scam signals often sit outside the token contract:

- one upstream address funds several wallets before they buy;
- wallets that look independent in the token graph cash out to the same sink;
- short ETH/WETH/stable paths connect holders, sellers, or control actors;
- funding happens shortly before first token activity;
- profits converge after a coordinated exit.

This belongs in `network/flow_context/`. It should use the token graph only to
select token-relevant seed addresses and windows. It should then use the
address-block participation index, processed-block cache, and `tx_fund_flow` to
extract non-token value movement around those seeds.

### Backbone

The backbone is the high-signal subgraph after suppressing noisy hubs. It should
preserve evidence that a hub was suppressed, but it should not let pools, WETH,
routers, CEX/bridge addresses, or token-local supernodes dominate the graph.

The backbone should emphasize:

- direct token transfers between relevant addresses;
- direct non-token fund flows between token actors;
- shared funders and shared sinks;
- short high-confidence flow paths;
- control and liquidity actor links;
- timing-aware funding or exit patterns.

### Risk/ML Feature Graph

Long term, the same layers should be usable as features for deterministic risk
rules or a graph model. A graph neural network should not receive an undifferentiated
hairball. It should receive typed nodes, typed edges, confidence/evidence
metadata, time windows, amounts, and hub-suppression signals.

Examples of risk features:

- supply concentration by connected holder cluster;
- fake-volume candidates: high pool interaction, low net balance change;
- shared-funder buyer groups;
- shared-sink profit convergence;
- creator/control actor trading or liquidity removal;
- synchronized first buys or coordinated sells;
- repeated operators across tokens.

Observed evidence and inferred evidence must remain separate. A direct token
transfer is not the same claim as "same operator." A shared funder is strong
context, but still an inference unless the path and timing are compelling.

---

## 1. What Questions Must This Answer?

### A. Ownership & Control ("Who really owns this?")
1. Who created the token? Are they still involved?
2. Was ownership renounced? If so, are there hidden admin roles still active?
3. Who controls the proxy admin? Who can upgrade the contract?
4. Is there a tax wallet? Where do the taxes go?
5. Are control actors (creator, owner, admin) also trading the token?
6. Are control actors connected to each other through funding or shared intermediaries?

### B. Clustering & Coordination ("Are these wallets working together?")
7. Do multiple buyer wallets share the same funding source?
8. Do multiple wallets send funds to the same deposit address?
9. Were many wallets created/funded in the same block or time window?
10. Is there a hub-and-spoke pattern — one address funding many buyers?
11. Are there circular token transfers with no net balance change (wash trading)?
12. Do addresses in a cluster sell at the same time (coordinated dump)?

### C. Trading Behavior ("Who is making money and how?")
13. Which addresses bought early and sold at peaks (insiders)?
14. Which addresses are still holding large bags?
15. Which addresses have massive unrealized losses (sacrifice wallets / exit liquidity)?
16. Is there fake volume — high trade count but no net movement?
17. Are there snipers who bought in the same block as pool creation?
18. Which addresses pay unusually high bribes?

### D. Liquidity & Pool Health ("Can I actually sell?")
19. Who controls the LP tokens? Is liquidity concentrated in one cluster?
20. Has liquidity been removed? By whom? Were they also control actors?
21. Are LP approvals granted to routers by addresses that also hold large token balances?
22. Is there a pattern of add-then-remove liquidity around pump events?

### E. External Connections ("Where does the money go?")
23. Do token flows lead to known CEX deposit addresses?
24. Are funds bridged to other chains?
25. Are there connections to known scam/mixer addresses?
26. Do profit-taking wallets have ENS names or other identities?

---

## 2. Shady Patterns to Detect

### Control Structure Red Flags
| Pattern | Signals |
|---------|---------|
| **Shadow Owner** | Owner renounced → ZeroAddress label, but Admin/ProxyAdmin labels still active on other addresses |
| **Creator Still Trading** | Creator label + PoolTrade edges on the same node |
| **Tax Wallet Drain** | TaxWallet label + high DenomTransfer outflows to addresses outside the token ecosystem |
| **Multi-sig Theater** | Ownership transferred to a Gnosis Safe or similar, but original owner still has Admin role |
| **Upgrade Trap** | ProxyAdmin label on an EOA (not a multisig) with PoolTrade activity |

### Coordination Red Flags
| Pattern | Signals |
|---------|---------|
| **Launch Group** | 5+ addresses with FeeSourceTouches to the same funder, all buying within 3 blocks of pool creation |
| **Same-Funder Network** | Addresses with Funding edges from a common source + TokenTransfer between them |
| **Deposit Cluster** | 3+ addresses sending DenomTransfer to the same intermediary (non-holder) address |
| **Wash Ring** | Closed loop of TokenTransfer edges (A→B→C→A) with near-zero net balance change |
| **Coordinated Dump** | Addresses in same connected component all selling in same 5-block window |
| **Bot Swarm** | One fee source touching 10+ addresses that each buy once and never sell |

### Trading Red Flags
| Pattern | Signals |
|---------|---------|
| **Insider Snipe** | First PoolTrade block = PoolCreation block + high token inflow + realized profit > 10x |
| **Sacrifice Wallet** | High token inflow + high denom outflow + negative realized profit > $1k |
| **Fake Volume** | High PoolTrade edge weight but token balance ≈ 0 and denom balance ≈ 0 (round-tripper) |
| **MEV Sandwich** | Address with PoolTrade edges where the same fee source has PoolTrade immediately before and after |
| **Bribe Launch** | PoolCreation block + fee source with bribe_amount > 0.5 ETH + immediate buy |
| **Gradual Rug** | ControlActor with LiquidityEvent (remove) edges + no corresponding add edges |

### Graph Structure Red Flags
| Pattern | Signals |
|---------|---------|
| **Supernode** | Single address with in-degree + out-degree > 50% of total edges |
| **Disconnected Profit** | Address with high realized profit but zero edges to any control actor or pool (self-sufficient bot?) |
| **Time-Window Cluster** | TemporalCoactivity edges linking 5+ addresses through the same hub in a single block |
| **Bridge/CEX Washing** | TokenTransfer → DenomTransfer → Cex/Bridge labeled node → DenomTransfer back to token buyer |

---

## 3. What Existing Tools Do (Benchmark)

### Bubblemaps (bubblemaps.io)
- **Visualization**: Bubble chart where bubble size = wallet holdings, proximity = transfer connections, color = cluster
- **Key insight**: Immediate visual clustering — "are 5 wallets holding 80% of supply?"
- **Weakness**: No time dimension, no PnL, no control structure, no edge detail
- **What to borrow**: The bubble/cluster metaphor is intuitive for supply concentration

### Arkham Intelligence
- **Visualization**: Force-directed graph with entity attribution ("This wallet belongs to Wintermute")
- **Key insight**: "Who is behind the wallet?" — entity labels from proprietary databases
- **Weakness**: Entity attribution is often wrong or missing for smaller tokens
- **What to borrow**: Entity grouping — collapse known clusters into single nodes

### Nansen Token God Mode
- **Visualization**: Dashboard with Smart Money labels, wallet tags, PnL rankings
- **Key insight**: "Are smart money wallets accumulating or dumping?"
- **Weakness**: Requires proprietary labeling; no interactive graph exploration
- **What to borrow**: PnL-centric address ranking with clear buy/sell signals

### Chainalysis Reactor / TRM Labs
- **Visualization**: Forensic graph with risk scoring, exposure wheel, cross-chain tracing
- **Key insight**: "Where did the money come from and where did it go?"
- **Weakness**: Enterprise-only, no real-time token-specific view
- **What to borrow**: Path tracing — click an address and see full fund flow history

### MetaSleuth / Breadcrumbs
- **Visualization**: Auto-pathfinding graph with fast investigation canvas
- **Key insight**: "Show me the path from Address A to Address B"
- **Weakness**: Manual investigation tool, not token-scoped
- **What to borrow**: Path highlighting and address search

### Etherscan
- **Visualization**: Raw tables and transaction logs
- **Key insight**: Ground truth for every transaction
- **Weakness**: No clustering, no visualization, no pattern detection
- **What to borrow**: Always provide a link back to raw transaction evidence

---

## 4. What Our Backend Already Supports

### Token Network (`eth_token/src/network/`)
- **Rich graph model**: 10 node kinds, 14 edge kinds, 25 label kinds with confidence scoring
- **Per-address activity tracking**: PnL, balances, buy/sell counts, fee history, bribes
- **Multi-layered edge evidence**: Each edge has examples, amounts, confidence, direction
- **Token-scoped pipeline**: Every transaction is analyzed in context of the tracked token
- **Weak edge detection**: `FeeSourceTouches`, `SharedIntermediary`, `TemporalCoactivity` are already flagged
- **View scoring**: Nodes are already ranked by importance (profit + balance + activity)

### Fund Flow (`tx_fund_flow/src/fundflownetwork/`)
- **Generic fund flow extraction** from `ProcessedTransaction` → ETH + token movements
- **Network builder**: Aggregates flows into `FundFlowNetwork` with nodes/edges
- **Graph discovery**: BFS exploration from seed addresses via Postgres `eth_db`
- **Visualization exporters**: Cytoscape.js, Vis.js, GraphML already implemented
- **Tx processor integration**: Converts `ProcessedTransaction` directly to fund flows

### Index Infrastructure (`reth_chain_query/src/reth_index/`)
- **Address block participation index**: MDBX database mapping address → blocks
- **Mempool arrival table**: txumber → first-seen timestamp for mined txs
- **Dormant table model files**: trade/address-metric/token/pool table structs
  exist, but they are not active until `RethIndexDB` opens them and a writer
  owns them

### Processed Block Cache (`tx_processor/src/tx_processor/cache.rs`)
- Flat binary `.pblock.zst` files
- ~0.57ms/block read when cached
- Used by live tracker and range runs

### Gaps
- **Cluster analysis is NOT implemented**: The `clusters/` module has empty placeholder files
- **Flow context scaffold exists, but adapters are not wired**:
  `network/flow_context/` defines seed/window/extraction/backbone contracts, but
  the real address-index, processed-block, and `tx_fund_flow` adapters still need
  implementation.
- **No temporal analysis**: We store block numbers but don't analyze ordering patterns ("funded then bought")
- **No external address book**: `Cex`, `Bridge`, `Router` labels exist but are not populated from external data
- **No pathfinding in token network**: Can't answer "show me how Address A funded Address B" (but `tx_fund_flow` has this!)
- **No cross-token view**: A multi-token operator appears as disconnected graphs
- **No nonce/gas analysis**: Can't detect fresh wallets or MEV bundles
- **No approval tracking**: ERC20 Approval events for the token itself are not ingested
- **No flash loan detection**: No analysis of flash loan initiators or cascading pool interactions
- **Token network and fund flow are only contract-integrated**: the
  `flow_context` seam exists, but production extraction still needs to call
  `tx_fund_flow`.

---

## 5. Proposed Visualization Layers

### Layer 1: Overview ("Should I even look closer?")
- **Token + Pool nodes** prominently displayed
- **Control actors** highlighted with badges (Creator, Owner, Admin, TaxWallet)
- **Cluster coloring** — addresses in the same connected component share a color
- **Red flags as annotation chips** above the graph: "Creator trading", "Same funder cluster", "Tax wallet draining"
- **Supply concentration bubbles** (Bubblemaps-style) overlaid or adjacent

### Layer 2: Control Structure ("Who owns what?")
- **Hierarchical or radial layout** with Token at center, Control actors in inner ring, traders in outer ring
- **ControlRelation edges** emphasized with thick red lines
- **Ownership timeline** — when did Owner change? When was trading enabled?
- **Admin role table** — list of all addresses with Admin/ProxyAdmin labels and their current balances

### Layer 3: Trading Clusters ("Who is working together?")
- **Community detection layout** — force-directed but with detected clusters visually grouped
- **Funding flow overlay** — show DenomTransfer inflows as animated arrows
- **Synchronized activity timeline** — when did addresses in Cluster X trade together?
- **Cluster summary cards**: "Cluster A: 12 addresses, $45k realized profit, funded by 0xabc..."

### Layer 4: Financial Flow ("Where did the money go?")
- **Sankey diagram** or **path-tracing graph** showing token → denom → CEX/mixer flows
- **PnL heatmap** on nodes (green/red intensity)
- **Edge amount labels** — show "$12.5k" on DenomTransfer edges
- **Time-filtered view** — slider to show only activity in a specific block range

### Layer 5: Evidence ("Prove it.")
- **Click any edge** → show the actual transactions that created it (with Etherscan links)
- **Click any node** → show full address activity: all movements, all costs, all labels with confidence
- **Export graph** as PNG or structured JSON for external analysis

---

## 6. Open Questions (for discussion)

### Q1: Scope — One token or cross-token?
> Should the network view be strictly scoped to one token, or should we have a mode where we can trace an address across multiple tokens the operator has touched? This is critical for detecting repeat scammers.

### Q2: Clustering — Should we implement it in Rust or compute in JS?
> The `clusters/` module is empty. We can either:
> - Implement community detection in Rust (e.g., Louvain algorithm) and send pre-computed clusters to the frontend
> - Send the full graph to the frontend and compute clusters there (limited by the 80-node cap)
> - Hybrid: compute coarse clusters in Rust, let the user explore sub-clusters interactively

### Q3: Real-time vs. post-hoc
> The live tracker has network graphs too. Should the visualization support real-time updates (new nodes/edges appearing as transactions happen), or is a static "analyze this snapshot" view sufficient?

### Q4: Address enrichment — do we want to integrate external label databases?
> We have the `KnownAddressBook` label source but no actual data. Options:
> - Integrate Arkham API, Etherscan labelcloud, or Nansen labels
> - Maintain our own growing address book (CEX hot wallets, known bridges, etc.)
> - Use heuristics only (Router, Contract, Wallet) without external attribution

### Q5: What is the primary "aha" moment we want?
> Different tools optimize for different insights:
> - Bubblemaps: "5 wallets own 80%" (supply concentration)
> - Arkham: "This is a Wintermute wallet" (entity attribution)
> - Nansen: "Smart money is dumping" (behavioral signal)
> - Chainalysis: "The money went to Binance" (fund tracing)
> 
> **What is the single most important insight we want a user to get in 10 seconds of looking at our graph?**

### Q6: Interaction depth — power user vs. quick scan
> Should the default view be simple (like Bubblemaps) with an "expert mode" toggle, or should we surface everything at once?

### Q7: Mobile
> Network graphs are inherently desktop experiences. Do we need a simplified mobile view (e.g., red flag list + key metrics), or is desktop-only acceptable?

### Q8: What patterns have you personally seen that we MUST catch?
> You have experience watching tokens launch, trading them, and analyzing the alpha engine's signals. What specific patterns have you seen that we should prioritize? Examples:
> - "The same 3 EOA addresses fund 20 sniper wallets every launch"
> - "The creator removes liquidity exactly 24 hours after enabling trading"
> - "A contract that looks like a token but is actually a honeypot"

---

## 7. Immediate Next Steps (Proposed)

1. **Wire `flow_context` to real data**: address-block participation index,
   processed-block loading, and `tx_fund_flow` extraction.
2. **Produce a token-specific backbone**: suppress pools, routers, WETH, CEX,
   bridges, zero/dead addresses, and token-local high-degree hubs while keeping
   suppression evidence.
3. **Implement cluster detection in Rust**: direct components first, then
   shared-funder/shared-sink/temporal clusters as separate evidence layers.
4. **Expose cluster IDs + confidence in `TokenNetworkView`** and, separately,
   expose `FlowContextSnapshot` for second-order evidence.
5. **Build the Layer 1 + Layer 2 frontend**: control structure, direct holder
   relations, and flow-context overlays/backbone.
6. **Add edge-click evidence panel**: show actual transactions, amounts,
   timestamps, and why an edge was inferred.
7. **Iterate toward risk features**: fake volume, coordinated funding, profit
   convergence, liquidity removal, and repeat-operator features.

---

*This document is a living design spec. Update it as decisions are made.*
