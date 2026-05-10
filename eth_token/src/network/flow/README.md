# Fund Flow Tracing (Integration Layer)

This module is an **integration layer** between the token-scoped network graph
(`eth_token/src/network/`) and the generic fund flow system (`tx_fund_flow/`).

## Why this exists

`tx_fund_flow` already has:
- `FundFlowAnalyzer` — extracts flows from `ProcessedTransaction`
- `GraphExplorer` — BFS discovery from seed addresses via Postgres `eth_db`
- `CytoscapeExporter` — already exports to Cytoscape.js format

We do **not** want to duplicate that. Instead, this module:
1. Takes addresses of interest from the token network graph
2. Queries `tx_fund_flow` for deeper fund flow traces
3. Returns results in a format compatible with `TokenNetworkView`

## Data sources

| Source | Purpose | Path |
|--------|---------|------|
| Address block index | Find blocks where two addresses co-occur | `reth_chain_query/src/reth_index/` |
| Processed block cache | Load actual blocks to inspect transfers | `tx_processor/src/tx_processor/cache.rs` |
| `tx_fund_flow` | Generic fund flow network + graph discovery | `tx_fund_flow/src/fundflownetwork/` |
| Token network graph | Token-scoped edges (TokenTransfer, DenomTransfer) | `eth_token/src/network/graph/raw.rs` |

## Modules

- `trace.rs` — Bridge: token-network addresses → `tx_fund_flow` traces
- `path.rs` — Multi-hop path finding (delegates to `tx_fund_flow` graph discovery)
- `timeline.rs` — Block-by-block chronological fund flow timeline
