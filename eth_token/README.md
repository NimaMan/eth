# eth_token

Agent operating map for ERC-20 token state, AMM pool state, token health, and
block-level token updates.

## Purpose

- Consume `tx_processor::ProcessedBlock` and `ProcessedTransaction` output.
- Maintain token, pool, lifecycle, LP, approval, activity, health, and network
  state for historical range builds and live token tracking.
- Provide the canonical Rust token/pool state used by `eth_token_server`,
  mempool context, and alpha reads.

## Owns

- ERC-20 token metadata, snapshots, transfer/control-address state, and
  lifecycle state.
- AMM pool state machines, currently strongest on Uniswap V2.
- Token health/scam/trading status derived from processed facts.
- Token network/activity views built from processed events.
- Block-level token update orchestration in `manager` and `tracking`.

## Does Not Own

- Raw RPC tracing, Reth DB access, or transaction/block decoding; use
  `tx_processor`, `tx_simulator`, and `reth_chain_query`.
- Live process hosting, HTTP/SSE views, or endpoint DTOs; use
  `eth_token_server`.
- Strategy decisions or mempool signal decisions.
- Python compatibility layers; expose stable Rust APIs first, then bind through
  `pyreth` only when needed.

## Data Flow

```text
ProcessedBlock / ProcessedTransaction
  -> BlockTokenProcessor applies txs in block/index order
  -> token registry + tracked token index + pool state + health/network state
  -> eth_token_server live/range views
  -> mempool_processor context and alpha market events
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Public module map | `src/lib.rs`, `src/README.md` |
| Block-level token application | `src/manager/`, `src/tracking/` |
| ERC-20 state and metadata | `src/erc20/` |
| Pool state machines | `src/pools/`, especially `src/pools/uniswap/v2.rs` |
| Chain metadata lookup/cache | `src/chain_metadata/` |
| Health/scam status | `src/health/` |
| Network/activity views | `src/network/` |
| Range and parity examples | `examples/README.md` |

## Tests And Commands

```bash
cargo test -p eth_token
cargo run -p eth_token --example token_tracking_range
cargo run -p eth_token --example uniswap_v2_lp_tracker_parity
cargo run -p eth_token --example uniswap_v2_pool_replay_reserves
```

## Current Hazards

- V2 tracking is the active parity surface; V3/V4 pool state and discovery are
  incomplete compared with V2.
- Same-block complex deployments can require exact metadata/pool replay; avoid
  assuming token metadata is stable before the processed block is fully applied.
- UI/API callers should use explicit view DTOs from `eth_token_server`, not raw
  internal token structs.
- Do not recreate log decoding here. If a field needs receipt/log/trace truth,
  add it to `tx_processor` first and consume the processed fact here.
