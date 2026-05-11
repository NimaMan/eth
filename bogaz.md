# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only bottleneck management here:
what is currently limiting the pipeline, how we measured it, what owns it, and
the next action that moves or removes it.

## Operating Rule

A run is useful only if it tells us which limit dominates: strategy quality,
fill modeling, decision auditing, backtest parity, mempool signal recall,
memory pressure, live-feed regressions, or execution.

## Focus Order

> **North star**: The pipeline must produce a profitable strategy on paper with
> realistic fills before real capital is deployed. Infrastructure is no longer
> the dominant limit; strategy quality and fill realism are.

| Order | Bottleneck | Owner | What To Watch | Next Focus |
| --- | --- | --- | --- | --- |
| 1 | **Strategy policy quality** | `alpha/strategies` | entry selectivity, exit coverage, risk reaction latency, skipped vs taken ratio, V2/V3/V4 behavior, PnL per pool | Replace `Snipe All v1` with a selective strategy: add pool-quality scoring, activate tax/honeypot/LP-approval exits, add position sizing, and filter low-expectation pools. |
| 2 | **Paper fill realism** | `alpha/engine` | synthetic execution reports versus worst achievable block price, slippage model, gas cost model | Replace perfect paper fills with worst-case block fill modeling in `PaperExecutionAdapter`. PnL must reflect what a real fill would have achieved. |
| 3 | **Trader decision ledger completeness** | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | every decision input has `TokenPoolId`, market payload, signal payload, rule id, decision, order, execution report, exit reason, and PnL snapshot | Make every skip, entry, and exit auditable in Postgres so the frontend can explain strategy behavior and we can train filters from losers. |
| 4 | **Backtest and replay alignment** | `alpha/backtest`, `alpha/engine` | same strategy state machine in historical and live paper runs, same `TokenPoolId` matching, historical lower-bound PnL | Run historical replays with the new selective strategy + worst-case fills. Compare backtest lower-bound PnL to live paper behavior. Iterate on rules offline. |
| 5 | **Mempool signal recall and timing** | `mempool_processor`, future `alpha/mempool_risk` | IPC drops, queue depth, arrival writes, first-seen timestamps, LP approvals before liquidity removals, V2/V3/V4 pool identity coverage | Improve early liquidity-removal detection across pool types. LP approval, removal intent, token, canonical `TokenPoolId`, and first-seen time must be persisted before the trader consumes them. |
| 6 | **Live feed failure isolation watch** | `alpha/live/feed`, `eth_token_server`, `eth_token`, `tx_simulator` | live status, failed block, block apply time, simulation validation errors, V2/V3/V4 tracked-pool counters | Watch the May 11 Reth-or-Redis live-state fix during the next run; optional pool metadata and buy/sell simulation failures must remain pool-level errors, not tracker-level failures. |
| 7 | **Live warmup memory pressure while filling processed-block cache** | `eth_token_server`, `alpha/live/feed`, `tx_processor`, Reth static files | systemd cgroup memory, RSS, cgroup `anon`/`file`, disk-cache hits/misses, cache write time | Add allocator trimming to the live warmup path and avoid running large backfills while token-server warmup is filling missing cache entries. |
| 8 | **Real execution handoff** | `alpha/engine`, `tx_executor` | adapter boundary, execution reports, nonce/gas failures, real order id to `TokenPoolId` mapping, receipt polling | Only replace the paper adapter with a `tx_executor` adapter after live paper PnL is consistently positive with worst-case fills and decision auditing is complete. |

## Frontend Alignment

ASENA renders this ledger at `/eth/bogaz/`. The page must keep the same order
as the table above and should only derive metrics from existing read-only APIs.

| Frontend Card | Source API | Metrics Shown |
| --- | --- | --- |
| Live Feed Readiness And Failure Isolation | `/eth/tokens/api/live/status`, `/eth/tokens/api/live/pools` | status, warmup, current block, apply time, processed-block source, tracked tokens, V2/V3/V4 pools, cache hit/miss, updated time, failures |
| Live Warmup Memory Pressure While Filling Processed-Block Cache | `/eth/tokens/api/live/status` | warmup, cache hit/miss, miss rate, cache read/write time, upstream time, apply time, tracked tokens/pools, memory API exposure status |
| Mempool Signal Recall And Timing | `/eth/tokens/api/mempool/signals?since_days=14&limit=500` | total signals, latest signal age, trading-enabled count, LP approvals, liquidity removals, approval/removal ratio, tax signals, token/pool coverage |
| Trader Decision Ledger Completeness | `/eth/trade/api/strategies/:strategy_id` | runtime, mode, trading flag, heartbeat age, live block/status, positions, orders/reports, risk count |
| Paper Fill Realism | `/eth/trade/api/strategies/:strategy_id` | report count, confirmed count, reports with block, reports with tx hash, gas modeled, fill errors |
| Strategy Policy Quality | `/eth/trade/api/strategies/:strategy_id` | active/scaffolded rules, open positions, risk counts by kind, latest risk, latest order |
| Backtest And Replay Alignment | `/eth/trade/api/runs`, `/eth/trade/api/strategies/:strategy_id` | range runs, completed range runs, latest range status/heartbeat, trader runs, latest trader status |
| Real Execution Handoff | `/eth/trade/api/strategies/:strategy_id` | mode, trading flag, reports with tx hash, adapter, run id, runtime status |

Frontend status semantics:

- `blocked`: source API is unavailable, live tracker failed, or a hard runtime
  error is present.
- `watch`: the stage is running but still incomplete, intentionally paper-only,
  or missing a required measurement such as memory diagnostics or worst-case
  fill modeling.
- `clear`: the stage has no currently known blocker for its role.

If this file changes the focus order, stage names, or monitored fields, update
`interface/asena/eth/static/bogaz.js` in the same change.

> **Note (May 10, 2026)**: The focus order above was inverted. Infrastructure
> stages 1/2 are now 6/7. Strategy quality (was 6) is now 1. If you update the
> frontend `bogaz.js` trust-domain order, match this new priority: Strategy
> Policy → Paper Fill → Decision Ledger → Backtest → Mempool Signals → Live
> Feed → Memory → Execution Handoff.

## Token Detail Page Improvement Backlog

These tasks target `/eth/tokens/live/:token_address/` and should be picked up
one by one. They are UI/product tasks, not a change to the bottleneck focus
order above.

Playwright review on May 10, 2026:

- The requested live URL
  `/eth/tokens/live/0xb37494a6f836e9e8c86ff4df9c0134544e92bfcb/`
  currently returns the detail shell with `token not found`, but still renders
  the same summary cards and bottom sections with `unknown` / `-` values.
- A populated control page,
  `/eth/tokens/live/0x0c8ff351a41e4384ea2dc8e0e6f076e3a0e14e96/`,
  shows the current layout: summary cards `Stage`, `Created`, `Risk`, `Pools`;
  sections `Market Snapshot`, `Pools`, `Activity Blocks`, `Address PnL`,
  `Token Network`, `Liquidity Tokens`; and bottom cards `Lifecycle`,
  `Risk & Controls`, `Metadata`, `Creation`.
- The populated page currently shows bottom-card duplication and noisy values:
  `Lifecycle` repeats stage/latest/risk, `Risk & Controls` repeats generic
  clear/no fields, `Metadata` includes `Decimals` and `Total supply raw`, and
  buy/sell activity rows render values like `0.3500 0xc02aaa...756cc2`.
- The current token network renders `67` visible nodes and `160` visible edges
  for the populated page, including token/pool nodes and visible address labels
  such as `0x67317c...bbc8a9`, which makes the graph hard to read.

Likely implementation owners:

- Template: `interface/asena/eth/tokens/templates/eth/tokens/detail.html`.
- Detail rendering: `interface/asena/eth/tokens/static/js/tokens/detail.js`.
- Activity rendering: `interface/asena/eth/tokens/static/js/tokens/activity.js`.
- Network rendering:
  `interface/asena/eth/tokens/static/js/tokens/network/graph.js`,
  `interface/asena/eth/tokens/static/js/tokens/network/pnl.js`,
  `interface/asena/eth/tokens/static/js/tokens/network/core.js`.
- CSS:
  `interface/asena/eth/tokens/static/css/tokens/detail.css`,
  `interface/asena/eth/tokens/static/css/tokens/network/index.css`.
- Backend data contracts if needed:
  `eth_token_server/src/views/token.rs`,
  `eth_token_server/src/views/pool.rs`,
  `eth_token_server/src/views/network.rs`.

### TD-1: Consolidate token facts and remove bottom cards

Current state:

- The top summary grid has `Stage`, `Created`, `Risk`, and `Pools`.
- The bottom of the page has separate `Lifecycle`, `Risk & Controls`,
  `Metadata`, and `Creation` cards.
- Token-level useful fields are split across those bottom cards.
- Low-value fields such as `Decimals` and `Total supply raw` are visible.
- Generic `Risk clear` / `Risk unknown` appears in the top summary, lifecycle,
  risk card, and pool card state strip.

Target behavior:

- Remove the bottom `Lifecycle`, `Risk & Controls`, `Metadata`, and `Creation`
  cards from the token detail page.
- Remove the generic `Risk` summary card and generic `Risk` display attributes.
  If a token has concrete evidence such as hidden mint, liquidity removal, LP
  approval, or ownership-control evidence, show that as explicit evidence or a
  flag. Do not show empty `risk=clear` / `risk=unknown` boilerplate.
- Add one top-level token facts card near the top of the page. It should merge
  the useful `Metadata` and `Creation` information and keep token-level totals.
- Keep token-level fields in this new card:
  address/link, symbol, name, stage/latest block/latest timestamp, created
  block/time, creator, creation tx, pool count/protocols, total transaction
  count, active activity-block count, unique address count if available, total
  buy volume, total sell volume, and total bribe ETH.
- Do not show `total_supply_raw` or `decimals`. Only show scaled supply if a
  later product decision says it is useful; default is to omit it.
- Keep pool-level fields in pool cards only:
  pool address/id, protocol, denom/currency, current liquidity, denom reserve,
  token supply in pool, liquidity/FDV, price/initial, can buy, can sell, buy
  tax, sell tax, pool lifecycle/stage, LP supply, LP holders, LP approval data,
  pool creation block, trading block, and last sync.

Implementation notes:

- Replace the side `<aside class="token-detail-side">` cards in
  `detail.html` with a single token facts section in the main flow.
- Replace `renderDl(el.metadataList, ...)`, `renderDl(el.creationList, ...)`,
  `renderDl(el.lifecycleList, ...)`, and `renderDl(el.riskList, ...)` in
  `detail.js` with a focused token facts renderer.
- Remove `detailRisk` from the summary grid and any generic risk field from
  `renderDetail`. If explicit evidence remains, name it by evidence type.

Validation:

- Playwright page text for a populated token no longer contains headings
  `Lifecycle`, `Risk & Controls`, `Metadata`, or `Creation`.
- Playwright page text no longer contains `RISK clear`, `RISK unknown`,
  `DECIMALS`, or `TOTAL SUPPLY RAW`.
- The new token facts card contains transaction count, active blocks, total buy
  volume, total sell volume, bribe ETH, creation block/time, creator, and
  creation tx.
- Pool cards still contain pool trading, liquidity, tax, LP, and pool lifecycle
  fields.

### TD-2: Audit token-level versus pool-level ownership

Current state:

- `Market Snapshot`, `Lifecycle`, `Risk & Controls`, and pool cards duplicate
  some pool state and token state.
- Token-level totals such as activity blocks and buy/sell volume are separated
  from token identity/creation facts.
- Pool-level facts such as buy/sell, tax, liquidity, and LP security leak into
  generic token cards.

Target behavior:

- Treat token facts as the page-level identity and aggregate behavior:
  identity, creation, latest observed block/time, number of transactions,
  active blocks, total buy volume, total sell volume, bribes, unique addresses,
  and number of pools.
- Treat pool facts as execution and market state:
  liquidity, reserves, price, FDV ratio, supply in pool, buy/sell viability,
  taxes, LP holder/approval state, pool stage, pool activity, and pool-specific
  last sync.
- `Market Snapshot` may stay as an aggregate top section, but it should not
  duplicate details that are already clearer in each pool card unless it is
  clearly marked as "primary pool" or "aggregate".
- Multi-pool tokens must avoid implying that one pool's buy/sell/tax state is a
  token-level truth.

Implementation notes:

- Create a small field ownership matrix in the implementation PR description
  before editing UI code.
- Prefer deriving the top token facts from `TokenSummary` / `TokenView` fields
  and pool sections from `PoolView`.
- If a field can be both token-level and pool-level, label it explicitly, for
  example `Primary pool liquidity` versus `Total buy volume`.

Validation:

- A token with one pool and a token with multiple pools both render without
  ambiguous token-level buy/sell/tax values.
- No pool-specific value appears in the token facts card unless its label says
  `primary pool` or `aggregate`.
- No token aggregate activity total appears inside a pool card unless it is
  filtered to that pool.

### TD-3: Simplify token network around hidden address patterns

Current state:

- The token network includes token nodes, pool nodes, pool connections, and many
  visible short address labels.
- The graph emphasizes obvious token/pool topology instead of hidden address
  patterns.
- Node radius currently mixes absolute profit, token balance, and activity
  count, so large holders/activity can dominate even when PnL is the intended
  signal.

Target behavior:

- On the token detail page, hide token and pool nodes from the visible graph.
  Do not show connections to the pool as graph edges; that does not add value
  here.
- Focus the graph on address-to-address relationships that suggest hidden
  coordination or manipulation: shared intermediary, shared fee source,
  temporal coactivity, direct transfers, repeated buy/sell timing, bribe
  relationships, creator-linked wallets, and other non-obvious address edges.
- Size address nodes by absolute PnL only, using a stable min/max radius and a
  log or square-root scale so one outlier does not flatten the graph.
- Color address nodes by PnL sign: green for profit, red for loss, neutral gray
  for flat/unknown.
- Do not use raw or shortened addresses as visible node labels. Use compact
  value labels such as `+0.76 ETH`, `-0.50 ETH`, or a role/cluster label when
  available. Keep the full address in `<title>`, tooltip, and the PnL table.
- Add a small summary above or beside the graph that reports visible clusters,
  visible addresses, visible hidden-pattern edges, omitted addresses, and total
  PnL represented.
- Keep the Address PnL table as the numeric companion. Hover/click should
  eventually cross-highlight table row and graph node.

Implementation notes:

- Frontend-only filtering can start in `network/graph.js`, but the better data
  contract is in `eth_token_server/src/views/network.rs`: build a token-detail
  graph view that filters to address nodes and address-to-address edges.
- Update `node_score` / graph node selection so token and pool nodes do not
  consume the visible node budget.
- Update `networkNodeRadius` to use only `abs(total_profit)`.
- Update `renderNetworkNode` so visible text does not render addresses.
- Update the legend to remove the pool legend item and explain PnL color/size
  plus hidden-pattern edge kinds.

Validation:

- Playwright on a populated token shows no visible graph labels containing
  `0x` and no labels beginning with `pool_`.
- The graph still renders useful address nodes and hidden-pattern edges when
  network data exists.
- Profitable addresses are green, losing addresses are red, and larger absolute
  PnL addresses have visibly larger nodes.
- The visible graph excludes token-to-pool and address-to-pool edges.

### TD-4: Improve Activity Blocks with pool-aware tables and charts

Current state:

- The Activity Blocks section is useful for finding build ranges, but the
  recent built activity table is hard to read.
- Volume cells include the raw denom address, for example
  `0.3500 0xc02aaa...756cc2`, even when the pool card already knows the denom.
- There is no chart for buy/sell activity, transaction count, or bribe bursts.
- Recent block activity is currently token-level; pool-level interpretation is
  hard when a token has multiple pools or multiple denoms.

Target behavior:

- Keep the indexed activity-block summary and `Build Range` / `Open Builder`
  actions as token-level controls.
- Add a chart for recent built activity:
  x-axis block/time, green buy-volume bars, red sell-volume bars, tx-count
  marker or line, and bribe ETH markers when present.
- Move or duplicate the recent built activity into pool cards as a `Pool
  Activity` panel when the activity can be attributed to a pool. The pool card
  should use the pool denom, so values render as `0.3500 ETH` or `0.3500 WETH`,
  not `0.3500 0xc02aaa...756cc2`.
- For token-level aggregate activity with multiple denoms, avoid concatenating
  amount plus address. Use denom symbols in headers/chips, separate rows per
  denom, or an aggregate display that has clear labels and full denom in the
  tooltip only.
- Add summary numbers near the chart: recent tx count, buy volume, sell volume,
  net flow, bribe ETH, first/last built block, and number of rows represented.
- Keep the detailed table below the chart, right-align numeric cells, and use
  consistent precision.

Implementation notes:

- `activity.js` owns the current table. `formatVolumeByDenom` should accept a
  display context with known denom metadata so table cells can show symbols
  instead of addresses.
- `detail.js` / pool card rendering should pass pool denom metadata into any
  pool-level activity renderer.
- If the backend does not expose pool-attributed recent block activity, extend
  `PoolView` or `TokenDetailResponse` to include `recent_pool_activity` rows.
  Keep the existing `/tokens/:address/activity-blocks` endpoint for indexed
  range discovery.
- The chart can be SVG for consistency with existing pool price/liquidity
  charts. It must have stable dimensions and degrade to an empty state when
  there are fewer than two rows.

Validation:

- Playwright on a populated token shows an activity chart above the recent
  activity table.
- The Buy Volume and Sell Volume cells no longer contain `0x` denom addresses
  when the denom is known.
- A one-pool token shows pool activity in the pool card and the numbers match
  the existing recent block rows.
- A multi-pool token does not merge pool-specific volumes without marking them
  as aggregate.
- Desktop and mobile screenshots show no overlapping table text, chart labels,
  or buttons.

### TD-5: Clean live token not-found detail state

Current state:

- A live token that has already fallen out of retention can render a full detail
  layout with `Stage unknown`, `Risk unknown`, `Pools 0`, and empty bottom
  cards.

Target behavior:

- For missing live tokens, render a compact not-found state with the requested
  token address, a short explanation, and links back to Live Tokens, Mempool
  Signals, and Token Builder.
- Do not render full market, pool, lifecycle, risk, metadata, creation, or
  network sections when there is no token payload.

Validation:

- Playwright against the missing live URL shows the compact not-found state and
  does not show `RISK unknown`, `STAGE unknown`, or the bottom card headings.

## Token Network & Fund Flow Infrastructure Backlog

These tasks build the backend analysis modules and frontend visualization for
token network investigation, fund flow tracing, and shady-pattern detection.
They depend on the existing `eth_token/src/network/` graph pipeline, the
`reth_chain_query/src/reth_index/` address-block-participation index, and the
processed-block cache.

### Existing modules (what we have)

| Module | Path | State | Purpose |
|--------|------|-------|---------|
| Graph model | `eth_token/src/network/model/` | Implemented | Node kinds, edge kinds, labels, evidence |
| Graph ingest | `eth_token/src/network/ingest/` | Implemented | Tx → batch → graph (transfers, authority, pools) |
| Graph raw | `eth_token/src/network/graph/raw.rs` | Implemented | `RawTokenNetworkGraph` with merge logic |
| Graph simplified | `eth_token/src/network/graph/simplified.rs` | Implemented | View-scoring, node selection (80 nodes / 160 edges) |
| Address activity | `eth_token/src/network/activity/` | Implemented | Per-address PnL, balances, costs, movements |
| Address block index | `reth_chain_query/src/reth_index/` | Implemented | MDBX: address → blocks, trades, metrics, tokens, pools |
| Processed block cache | `tx_processor/src/tx_processor/cache.rs` | Implemented | Flat binary `.pblock.zst` files |
| Network view | `eth_token_server/src/views/network.rs` | Implemented | `TokenNetworkView` with PnL + graph subsets |
| Fund flow (generic) | `tx_fund_flow/src/fundflownetwork/` | Implemented | Generic ETH/token flow extraction, BFS discovery, Cytoscape export |
| Fund flow (eth_db) | `tx_fund_flow/src/eth_db_fetcher/` | Implemented | Postgres-backed address/tx/participant queries |
| Clusters | `eth_token/src/network/clusters/` | **Empty** | Placeholder `.rs` files with module comments only |
| Cluster snapshots | `eth_token/src/network/snapshots/cluster.rs` | **Empty** | Placeholder comment only |
| Cross-token | *(none)* | **Missing** | No module for tracing operators across multiple tokens |
| Enrichment | *(none)* | **Missing** | No module for populating known address labels |
| Temporal | *(none)* | **Missing** | No module for block-internal ordering analysis |
| Token↔FundFlow bridge | `eth_token/src/network/flow/` | **New stubs** | Integration layer between token graph and `tx_fund_flow` |

### Proposed new folder structure

```
eth_token/src/network/
  clusters/
    components.rs          # Connected-component detection (fill existing)
    intermediaries.rs      # Shared-intermediary discovery (fill existing)
    time_windows.rs        # Temporal coactivity clustering (fill existing)
    scoring.rs             # Cluster confidence + risk-feature scoring (fill existing)
  flow/                    # NEW: Fund flow tracing
    mod.rs
    trace.rs               # Trace denom/token flows between addresses
    path.rs                # Shortest-path / multi-hop path finding
    timeline.rs            # Block-by-block fund flow timeline
    README.md
  cross_token/             # NEW: Cross-token operator tracing
    mod.rs
    operator.rs            # Detect repeat operators across tokens
    overlap.rs             # Address overlap between token networks
    README.md
  enrichment/              # NEW: Address label enrichment
    mod.rs
    known_book.rs          # Integrate external known-address databases
    heuristics.rs          # Contract, router, fresh-wallet heuristics
    README.md
  temporal/                # NEW: Temporal analysis
    mod.rs
    ordering.rs            # Tx ordering within blocks (funded-then-bought)
    windows.rs             # Sliding window coactivity detection
    README.md
```

### TN-1: Connected-component clustering

Fill `eth_token/src/network/clusters/components.rs`.

- Input: `RawTokenNetworkGraph` or `TokenNetworkGraphView`
- Output: `BTreeMap<NetworkNodeId, ComponentId>` mapping each node to a component
- Exclude weak edges (`FeeSourceTouches`, `SharedIntermediary`, `TemporalCoactivity`) from the default component graph, but provide a flag to include them.
- Components should be sorted by total PnL (sum of address profits) or by member count.

### TN-2: Shared-intermediary clustering

Fill `eth_token/src/network/clusters/intermediaries.rs`.

- Detect addresses that receive funds from multiple token holders but do not hold the token themselves.
- Output: list of intermediary nodes with in-degree (number of funders) and total denom received.
- Flag intermediaries that are also labeled `Cex`, `Bridge`, `Router`, or `Contract`.

### TN-3: Time-window coactivity clustering

Fill `eth_token/src/network/clusters/time_windows.rs`.

- Group addresses that trade the token in the same block window (e.g., 1 block, 3 blocks, 10 blocks).
- Output: coactivity clusters with block range, member addresses, and shared activity kind (buy, sell, add liquidity, remove liquidity).
- This is the backend for the "coordinated dump" and "launch group" patterns.

### TN-4: Cluster scoring and risk features

Fill `eth_token/src/network/clusters/scoring.rs`.

- Compute a risk score per cluster based on:
  - Control actor overlap (does the cluster contain Creator/Owner/Admin/TaxWallet?)
  - Profit concentration (what % of total token profit is in this cluster?)
  - Funding homogeneity (were all members funded by the same 1-3 addresses?)
  - Timing synchrony (how tight is the buy/sell window?)
  - Liquidity event correlation (did the cluster remove liquidity together?)
- Output: `ClusterRiskScore` struct with score, reasons, and confidence.

### TN-5: Wire cluster IDs into TokenNetworkView

Update `eth_token_server/src/views/network.rs`.

- Run cluster detection during `TokenNetworkView::from_graph`.
- Add `clusters: Vec<NetworkClusterView>` to `TokenNetworkView`.
- Each cluster view: id, member node ids, total profit, risk score, primary labels.
- Add `cluster_id: Option<String>` to `TokenNetworkNodeView` so the frontend can color by cluster.

### TN-6: Fund flow tracing (bridge to `tx_fund_flow`)

`tx_fund_flow/src/fundflownetwork/` already implements generic fund flow extraction,
BFS graph discovery, and Cytoscape export. **Do not duplicate it.**

Instead, build the integration bridge in `eth_token/src/network/flow/`:

- Take addresses of interest from the token network graph (e.g., a suspicious cluster)
- Delegate actual flow extraction to `tx_fund_flow::FundFlowAnalyzer` or `GraphExplorer`
- Use `RethIndexReader::get_address_participation_blocks` to narrow the block range
- Use the processed-block cache to load specific blocks for evidence
- Format results as `FundFlowTrace` compatible with `TokenNetworkView`
- Expose via server API: `/tokens/:address/fund-flow?target=:address&max_hops=3`
- Frontend: clicking "Trace Funds" on a graph node opens the `tx_fund_flow` Cytoscape view
  scoped to the token's address set

### TN-7: Cross-token operator tracing

Create `eth_token/src/network/cross_token/`.

- Input: multiple `RawTokenNetworkGraph`s for different tokens.
- Detect addresses that appear in multiple token graphs with similar patterns (e.g., same funder, same intermediary, snipe-then-dump behavior).
- Output: `CrossTokenOperatorView` with operator address, tokens touched, repeated patterns, total cross-token profit.
- This requires the server to either keep multiple token graphs in memory or query a shared operator index.

### TN-8: Address enrichment

Create `eth_token/src/network/enrichment/`.

- Populate `Cex`, `Bridge`, `Router` labels from external data or heuristics.
- Heuristic: high out-degree + no token holdings + `FeeSourceTouches` to many addresses = likely Router.
- Heuristic: receives from many addresses + sends to few large balances + labeled by community = likely CEX deposit.
- Maintain a local known-address book JSON/DB that grows over time.

### TN-9: Temporal analysis

Create `eth_token/src/network/temporal/`.

- For a given address pair, analyze block-level ordering:
  - "Address A sent 0.5 ETH to Address B in tx 5, then Address B bought TOKEN in tx 6"
- Use `log_index` within blocks for sub-block ordering.
- Detect patterns: fund-then-buy, buy-then-transfer, remove-then-dump.
- Output: `TemporalPattern` with sequence of events, block ranges, and confidence.

### TN-10: Layer 1 frontend — Overview + Control Structure

Frontend: `interface/asena/eth/tokens/static/js/tokens/network/`.

- Cluster-colored force-directed graph (Cytoscape).
- Control actor badges (Creator, Owner, Admin, TaxWallet) on nodes.
- Red-flag annotation strip above graph.
- Summary: visible clusters, hidden nodes, total represented PnL.

### TN-11: Layer 2 frontend — Trading Clusters

- Community-detection layout with cluster grouping.
- Cluster summary cards (member count, total profit, risk score).
- Click cluster → highlight all members, dim rest.
- Cross-highlight with Address PnL table.

### TN-12: Layer 3 frontend — Financial Flow

- Sankey or path-tracing view for token → denom → CEX flows.
- PnL heatmap on nodes.
- Edge amount labels.
- Time slider to filter by block range.

### TN-13: Frontend graph ↔ PnL table integration

- Click PnL table row → pan/zoom graph to node and highlight neighborhood.
- Click graph node → scroll PnL table to row and highlight.
- Hover either → highlight both.

### TN-14: Edge evidence panel

- Click any graph edge → slide-out panel showing:
  - Edge kind, weight, amount
  - Actual transaction hashes with Etherscan links
  - Block number, timestamp, gas used
- This turns the graph from "pretty picture" into "auditable evidence".

## Active Bottleneck: Strategy Policy Quality + Paper Fill Realism

Observed on May 10, 2026; updated on May 11 after the live-state restore fix.
The dominant limit is not infrastructure; it is that the strategy loses money
on paper and the paper fills are unrealistically perfect.

### Latest State Snapshot (May 10, 2026 15:27 UTC)

Live pipeline:
```text
eth-token-server
  status:               live
  current block:        25065005
  warmup:               complete (7000 blocks)
  live blocks processed: 96
  txs scanned:          1,913,402
  txs processed:        1,870,809
  tx failures:          794
  tracked tokens:       648
  tracked pools:        405 (V2: 277, V3: 4, V4: 124)
  cache hits:           7094
  cache misses:         2
  last block apply:     39 ms
  last cache read:      13 ms
```

Paper strategy (`snipe-all-v1`):
```text
mode:                 paper
status:               running since 2026-05-08 14:37 UTC
positions:            167 (open: 70, closed: 97)
orders:               282
execution reports:    264 confirmed, 0 failed
risk events:          144 (103 critical)
buy volume:           1.85 ETH
sell volume:          0.97 ETH
realized PnL:         0.00 ETH
ROI:                  0.0%
```

### Interpretation

- **Live infrastructure is healthy**: Cache hit rate is ~99.97%, block apply is
  ~39 ms, tracked set is 648 tokens / 405 pools. The pipeline can sustain live
  paper trading.
- **The strategy is not selective**: It buys every eligible pool. 167 buys in ~2
  days with 0.01 ETH each = 1.85 ETH deployed. 70 positions are still open.
- **Paper fills hide losses**: `PaperExecutionAdapter` reports perfect fills
  (`filled_amount = intent.amount`). Realized PnL is reported as exactly 0.0,
  which is mathematically implausible for 97 closed positions unless the fill
  model is perfect and gas/slippage are ignored.
- **Exits are incomplete**: Only `liquidity_removal` exits are active. Tax,
  honeypot, and LP-approval exits are scaffolded but hold. 103 critical risk
  events fired; many of those should have triggered sells but did not.
- **Live-state context lag resolved**: The May 10 `snapshot base block ... ahead
  of local historical context` cascade is no longer an active bottleneck. The
  simulator now uses exactly two sources during live operation: Reth historical
  state first, then Redis live state snapshots. Missing-context errors are
  treated as transient during uncached block replay.

### Immediate Operating Rule

- **Do not add real capital until paper PnL is positive with worst-case fills.**
- The next engineering block is `alpha/strategies` + `alpha/engine`, not
  `tx_executor` or live feed tuning.
- Backfill and large cache work can run freely; live warmup is complete and
  memory is stable.

### Next Actions

| Priority | Action | Owner | Validation |
| --- | --- | --- | --- |
| 1 | Implement worst-case paper fill modeling in `PaperExecutionAdapter`: use block-level price impact, apply slippage, deduct gas, and record fill divergence vs intent. | `alpha/engine` | Paper PnL on closed positions becomes negative (reflecting reality) instead of exactly zero. |
| 2 | Activate tax/honeypot exit rule in `snipe_all/rules/tax.rs`: sell on `RiskKind::TaxChange` or `Honeypot` when an open position exists. | `alpha/strategies` | Frontend risk table shows sells tied to tax/honeypot events; closed-position count rises. |
| 3 | Activate LP-approval exit rule in `snipe_all/rules/lp_approval.rs`: sell on `RiskKind::LpApproval` for private creators. | `alpha/strategies` | Same as above for LP-approval events. |
| 4 | Add pool-quality entry filter to `snipe_all/rules/entry.rs`: score pools by creator history, network cluster risk, holder concentration, and mempool signal density. Reject below threshold. | `alpha/strategies`, `eth_token` | Entry count drops; taken/total ratio is visible in decision ledger; PnL per pool improves. |
| 5 | Add position sizing to `SnipeAllConfig`: dynamic buy amount based on pool score, portfolio heat, and recent win rate. | `alpha/strategies`, `alpha/core` | Config exposes `min_buy`, `max_buy`, `score_multiplier`; buy amounts vary across positions. |
| 6 | Wire decision-audit rows for every skip so the frontend can show *why* a pool was rejected. | `alpha/engine`, `alpha/store` | `strategy_observations` table has skip rows with rule id, score, and reason. |
| 7 | Run a 7-day historical backtest with the new selective strategy + worst-case fills. | `alpha/backtest` | Backtest PnL is comparable to live paper PnL; rules iterate offline. |
| 8 | Build `TxExecutorAdapter` implementing `EngineExecutionAdapter` only after paper PnL is positive for 3 consecutive days. | `alpha/engine`, `tx_executor` | Real-submission mode is gated by a config flag and requires explicit operator enable. |

## Processed-Block Cache Baseline

Current cache layout objective:

```text
processed-block-cache/
  ethereum-mainnet/
    <block_number>.pblock.zst
```

Recent measured numbers after switching to the flat binary layout:

```text
single fresh block fill:
  block: 25050002
  txs: 507
  EVM/process: 512.7 ms
  cache write: 12.0 ms
  immediate read: 6.6 ms

single cached block read:
  block: 25050002
  read: 5.3 ms

100-block cached range:
  range: 25043001-25043100
  cache hit rate: 100%
  plan time: 0.255 ms
  parallel read wall time: 56.8 ms
  wall avg: 0.57 ms/block
  individual read p50/p95/max: 3.16 / 6.51 / 9.02 ms
```

What this means:

- Cache read planning by block number is not currently the bottleneck.
- Cached range replay is fast enough for warmup once blocks are present.
- The expensive path is cache miss fill: Reth block processing plus file-cache
  pressure from Reth static files and DB access.

## Live Feed Readiness And Failure Isolation

The live token tracker must reach `live` reliably before paper trading can
produce dependable measurements.

Known failure class:

- Simulation validation errors, such as insufficient simulated funds, can fail
  runtime warmup if treated as block-level errors.
- Immediate live retention pruning of newly scammed pools keeps the live set
  small but leaves no inspection window. Example: MOTH
  `0xec402ba62e9d67650359ba859ddc1ae22a84840f` had a `DRAINING` mempool
  liquidity-removal signal at block `25,063,350`, but the token detail showed
  `pools=0` after the pool fell below the `0.1 ETH` live WETH floor.

Target behavior:

- Optional pool metadata failures and buy/sell simulation failures should be
  recorded on the affected pool.
- They should not fail the whole live tracker.
- Runtime failure should be reserved for unrecoverable processed-block loading,
  ordering, or state corruption errors.
- Risk-bearing depleted pools should be retained for `15,000` blocks when
  reserves fall sharply from above the live retention floor to below it, then
  pruned as before. Token Lab case:
  `token_lab/investigations/mothman_live_retention_liquidity_removal_25063339/`.

## Path to Profitable Deployment

This section is a concise roadmap from the current state (live paper, zero
selectivity, perfect fills) to a deployed strategy that makes money.

### Stage 0 — Current State (COMPLETE)

- [x] Live token tracker reaches `live` reliably
- [x] Processed-block cache fills and reads fast (>99% hit rate)
- [x] Paper trader binary runs continuously against live state
- [x] Postgres decision ledger persists positions, orders, reports, risks
- [x] Mempool signals (liquidity removal, tax, honeypot, LP approval) are
  detected and published
- [x] Frontend renders live status, strategy detail, and pipeline bottlenecks

### Stage 1 — Make Paper Honest (NEXT)

Goal: Paper PnL must reflect reality. If the strategy loses money, paper must
show a loss.

- [ ] `PaperExecutionAdapter` models worst-case fill: block-level price impact,
  configured slippage bps, gas cost deduction.
- [ ] `SimulatedExecutionAdapter` in backtest uses the same worst-case rule so
  backtest and live paper are aligned.
- [ ] Front-end performance chart shows "modeled PnL" separately from "perfect
  PnL" during the transition.

Acceptance: Running the current `Snipe All v1` against 7 days of historical data
produces negative PnL (because buying everything with realistic fills is
unprofitable).

### Stage 2 — Activate Exits

Goal: Stop holding bags. Every critical risk event should trigger a sell.

- [ ] `rules/tax.rs`: sell on `TaxChange` / `Honeypot` when position is open.
- [ ] `rules/lp_approval.rs`: sell on `LpApproval` for private creators.
- [ ] `rules/creator_label.rs`: label creators public/private; wire label into
  LP-approval rule logic.
- [ ] Add a "scam" exit: if `RiskKind::Scam` or critical risk fires, exit
  immediately even if it means a 100% loss.

Acceptance: Closed-position count rises; average hold time for bad tokens
drops; risk-event-to-sell latency is < 1 poll cycle (2s).

### Stage 3 — Selective Entry

Goal: Do not buy every pool. Buy only pools with positive expected value.

- [ ] Add `PoolScore` to `eth_pool_classification` or `alpha/strategies`:
  combine creator history, network cluster risk score, holder concentration,
  liquidity depth, denom quality, and mempool signal density.
- [ ] `rules/entry.rs` rejects pools below a configurable `min_score`.
- [ ] Decision audit rows record score + reason for every skip.
- [ ] Frontend strategy detail shows "evaluated / skipped / entered" counts.

Acceptance: Entry rate drops by > 50% (from ~80/day to < 40/day) while
maintaining or improving the win rate of closed positions.

### Stage 4 — Position Sizing

Goal: Bet more on high-conviction pools, less on marginal ones.

- [ ] `SnipeAllConfig` adds `min_buy`, `max_buy`, `score_multiplier`.
- [ ] `PortfolioLimits` enforces max open positions, max exposure per denom,
  max daily drawdown.
- [ ] Engine rejects buy intents that would breach portfolio limits.

Acceptance: Buy amounts vary across positions; portfolio heat metric is
visible in frontend.

### Stage 5 — Backtest-Driven Iteration

Goal: Iterate on rules offline without waiting for live days.

- [ ] Run 30-day historical replay with selective strategy + worst-case fills.
- [ ] Compare backtest PnL curve to live paper PnL curve.
- [ ] Tune `min_score`, slippage model, and exit thresholds in backtest.
- [ ] Re-run live paper with tuned parameters.

Acceptance: A rule change can be validated in backtest within hours and
redeployed to live paper with confidence.

### Stage 6 — Live Execution Adapter

Goal: Submit real transactions. ONLY after paper is profitable.

- [ ] Build `TxExecutorAdapter` implementing `EngineExecutionAdapter`.
- [ ] Convert `OrderIntent` → swap calldata (V2/V3/V4 route + slippage).
- [ ] Submit to `tx_executor`, poll for receipt, map real tx hash to
  `TokenPoolId`.
- [ ] Add a manual "enable real trading" flag with capital limit and circuit
  breaker.

Acceptance: The same strategy that was profitable on paper produces real fills
with tracked PnL. Real capital is limited to a small test amount until 30 days
of positive real PnL are recorded.

### Gate Criteria Summary

| Gate | Requirement | Current |
| --- | --- | --- |
| G1: Honest paper | Paper PnL reflects worst-case fills | ❌ Perfect fills |
| G2: Active exits | All critical risks trigger sells | ❌ Only liquidity removal |
| G3: Selective entry | < 50% of eligible pools entered | ❌ 100% entered |
| G4: Sized bets | Variable position sizing | ❌ Fixed 0.01 ETH |
| G5: Backtest parity | Backtest PnL ≈ live paper PnL | ❌ Not validated |
| G6: Real money | Paper PnL positive for 7+ days | ❌ 0.0 ETH on 97 closes |

Do not proceed to the next stage until the current stage's acceptance criteria
are met and recorded in this file with a dated measurement.

## Resolved Incident: Live Tracker Failure Cascade (May 10, 2026 20:54 UTC)

Status: resolved in code on May 11, 2026.

The live token tracker (`eth_token_server` PID 2925369) transitioned from
`live` to `failed` at block **25067207** after tx-level snapshot restoration
started failing at block **25067028**. The first error was:

```text
cannot restore live state snapshot for block 25067028:
snapshot base block 25067027 is ahead of local historical context block 25067024
```

Root cause:

- The simulator rejected Redis `ChainStateSnapshot` restores when the snapshot
  base was ahead of `latest_historical_context_block_number()`.
- That strict check used `min(Reth Finish stage, static-file header view)`.
  During live operation, Reth may have executed state before static-file headers
  are visible.
- The tracker then accumulated 2,705 tx-level failures, slowed down, fell behind
  the Redis live-block retention window, and finally failed on uncached block
  **25067207**.

Fix shipped:

- `tx_simulator/src/block_context/mod.rs`: live state now has exactly two
  sources: Reth historical state first, then Redis live state snapshots. Snapshot
  restore can walk Redis snapshot bases until it finds a Reth base, then merges
  snapshot overlays in order.
- `tx_processor/src/processed_block_provider/load.rs`: uncached block replay now
  treats live context lag, missing live headers, missing Redis snapshots, and
  trace failures as transient retryable errors.
- `tx_simulator/src/block_context/mod.rs`: parent live snapshots are reused when
  building new snapshots, so the base does not unnecessarily rebase to the live
  tip.

Validation:

- `cargo check -p tx_simulator`
- `cargo check -p eth_live_feed`
- `cargo test -p tx_processor processed_block_provider::load::tests --lib`

Current monitoring:

- Watch `/eth/tokens/api/live/status` after restart.
- This incident should not remain in the active bottleneck list. It is now a
  live-feed regression watch item only.
- Acceptance: `live_status` returns to `live` and stays there without
  `last_error` containing `snapshot base block` or `failed to process uncached
  block`.
