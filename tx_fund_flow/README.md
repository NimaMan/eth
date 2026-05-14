# tx_fund_flow

Agent operating map for fund-flow and network analytics over Ethereum
transaction data.

## Purpose

- Build graph/network views from processed Ethereum transactions.
- Support fund-flow ranking, discovery, analysis, and Python-facing analytics.
- Keep analytics isolated from core simulation/decoding crates.

## Owns

- Shared analytics types and errors in `src/core_types/`.
- DB-backed fetch helpers in `src/eth_db_fetcher/`.
- Fund-flow graph construction and analysis in `src/fundflownetwork/`.
- PyO3 analytics bindings in `src/fundflownetwork_py/`.

## Does Not Own

- Core transaction simulation; use `tx_simulator`.
- Processed transaction decoding and balance deltas; use `tx_processor`.
- Chain/entity query primitives; use `reth_chain_query`.
- Mempool, token-state, strategy, or execution decisions.

## Data Flow

```text
seed address / tx_processor ProcessedTransaction / DB-backed query output
  -> discovery-driven directed fund-flow graph/network construction
  -> ranking/analysis
  -> Rust examples or fundflownetwork_py bindings
```

## Relationship To Token Risk Graphs

`tx_fund_flow` is the generic money-movement engine. It should not become a
token-risk system, but it is the right source of second-order evidence for one.

The token network in `eth_token/src/network/` owns token-scoped facts:

- token transfers;
- pool trades;
- liquidity events;
- creator/owner/admin/control relations;
- per-address token activity and PnL proxies.

The token flow-context layer uses `tx_fund_flow` to answer questions that are
outside the token contract:

- did one upstream address fund several token traders?
- do profitable token sellers cash out to the same sink?
- is there a short ETH/WETH/stable path between token actors?
- did funding happen shortly before first token activity?
- does the high-signal backbone remain connected after pools, routers, WETH,
  CEX/bridge hubs, and other noisy nodes are suppressed?

Keep the boundary clean:

- `tx_fund_flow` extracts and aggregates directed fund-flow evidence.
- `eth_token::network::flow_context` chooses token-relevant seeds/windows and
  interprets the resulting flows as token-risk context.
- `eth_token::network::clusters` and future health/risk views decide whether
  evidence becomes a suspicious cluster, fake-volume signal, or graph-model
  feature.

## Where To Look First

| Need | Start here |
| --- | --- |
| Workspace members | `Cargo.toml` |
| Shared types | `src/core_types/README.md`, `src/core_types/` |
| DB fetching | `src/eth_db_fetcher/` |
| Network analysis | `src/fundflownetwork/README.md`, `src/fundflownetwork/` |
| Python bindings | `src/fundflownetwork_py/` |
| Discovery-to-directed-network builder | `src/fundflownetwork/src/processed_network_builder.rs` |
| Integration with tx processor | `src/fundflownetwork/src/tx_processor_integration.rs` |

## Tests And Commands

```bash
cargo test --manifest-path tx_fund_flow/Cargo.toml
cargo run --manifest-path tx_fund_flow/Cargo.toml -p tx_fund_flow-fundflownetwork --example basic_usage
cargo run --manifest-path tx_fund_flow/Cargo.toml -p tx_fund_flow-fundflownetwork --example build_fund_flow_network
```

## Current Hazards

- Treat this as analytics, not another processing pipeline. Do not duplicate
  `tx_processor` decoding or simulator logic.
- Several subcrates are outside the root ETH workspace; use
  `--manifest-path tx_fund_flow/Cargo.toml` for commands.
- Keep tests on real data or explicit integration fixtures. Avoid adding mock
  behavior that hides DB/schema mismatches.
