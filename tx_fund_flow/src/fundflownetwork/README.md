# fundflownetwork

Local operating map for ETH fund-flow graph construction.

## Purpose

- Discover candidate address neighborhoods from `eth_db.tx_participants`.
- Replay selected transactions through `tx_processor`.
- Convert `ProcessedTransaction` values into directed ETH/token fund-flow edges.
- Export `FundFlowNetwork` values for Rust examples and `fundflownetwork_py`.

## Owns

- Participant discovery: `src/graph_discovery/`.
- Processed transaction to fund-flow conversion: `src/tx_processor_integration.rs`.
- Discovery-driven directed network assembly: `src/processed_network_builder.rs`.
- Flow aggregation and net balance calculation: `src/fund_flow_analyzer.rs`.
- Node/edge graph construction: `src/network_builder.rs`, `src/network_types.rs`.
- Visualization exporters: `src/visualization.rs`.

## Does Not Own

- Raw transaction replay, log decoding, internal traces, or balance deltas; use `tx_processor`.
- Reth database access primitives; use `reth_chain_query` and `tx_processor`.
- Trading decisions, opportunity ranking, or execution.
- Token price/liquidity valuation.

## Data Flow

```text
seed address
  -> GraphExplorer::explore
  -> DiscoveryOutput { participant graph, priority txs }
  -> ProcessedFundFlowNetworkBuilder::select_transaction_hashes
  -> ProcessedTxProvider::process_transaction_by_hash
  -> extract_fund_flows_from_processed_tx
  -> FundFlowAnalyzer::analyze_fund_flows
  -> NetworkBuilder::build_centered_network
  -> CytoscapeExporter / VisJsExporter / GraphMLExporter
```

`fundflownetwork_py::PyFundFlowNetworkBuilder.build_from_address` uses this full path. It no longer returns a participant-only graph.

## Where To Look First

| Need | Start here |
| --- | --- |
| Full discovery-to-directed-network path | `src/processed_network_builder.rs` |
| Why an address is expanded or stopped | `src/graph_discovery/routing_rules.rs` |
| DB participant lookup | `src/graph_discovery/db_queries.rs` |
| Priority transaction selection | `src/graph_discovery/tx_selector.rs` |
| Exact extracted movements | `src/tx_processor_integration.rs` |
| Flow aggregation thresholds | `src/fund_flow_analyzer.rs` |
| Node and edge construction | `src/network_builder.rs` |
| Output shape | `src/visualization.rs` |
| Python entrypoint | `../fundflownetwork_py/src/lib.rs` |

## Tests And Commands

```bash
cargo check --manifest-path tx_fund_flow/Cargo.toml --all-targets
cargo test --manifest-path tx_fund_flow/Cargo.toml
cargo run --manifest-path tx_fund_flow/Cargo.toml -p tx_fund_flow-fundflownetwork --example test_graph_discovery
cargo run --manifest-path tx_fund_flow/Cargo.toml -p tx_fund_flow-fundflownetwork --example test_tx_processor_integration
```

Integration paths need `DATABASE_URL` for `eth_db` and `RETH_DATADIR` for Reth-backed transaction replay.

## Current Hazards

- Discovery edges are participant relationships, not money movements. Only the processed-network builder produces directed fund-flow edges.
- Transaction value in discovery is used for prioritization only; exact value movement comes from `ProcessedTransaction`.
- Non-WETH ERC20/ERC721/ERC1155 movements are extracted, but network edges do not yet carry token units or USD value. WETH can be treated as ETH.
- Gas edges are supported by extraction/analyzer but disabled in the Python builder by default.
- Gas destination is currently `Address::ZERO`, not validator/coinbase.
- Transaction replay can fail for individual hashes; the directed builder records failures and continues.
- `expand_node` and `get_fund_flow_insights` in the Python binding are intentionally not implemented yet.
