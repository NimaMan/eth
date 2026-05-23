# eth_token

Agent operating map for ERC-20 token state, AMM pool state, token health, and
block-level token updates.

## Purpose

- Consume `tx_processor::ProcessedBlock` and `ProcessedTransaction` output.
- Maintain token, pool, lifecycle, LP, approval, activity, health, and network
  state for historical range builds and live token tracking.
- Provide the canonical Rust token/pool state used by `eth_chain_server`,
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
  `eth_chain_server`.
- Strategy decisions or mempool signal decisions.
- Python compatibility layers; expose stable Rust APIs first, then bind through
  `pyreth` only when needed.

## Data Flow

```text
ProcessedBlock / ProcessedTransaction
  -> BlockTokenProcessor applies txs in block/index order
  -> token registry + tracked token index + pool state + health/network state
  -> eth_chain_server live/range views
  -> mempool_processor context and alpha market events
```

## Block Apply Contract

`eth_token` is the source-state engine. It should finish a full block before any
consumer treats the state as tradable or page-visible.

The block apply contract is:

```text
processed block
  -> apply every relevant transaction in block/index order
  -> coalesce post-block pool simulations
  -> update token/pool/analytics state as-of the end of the block
  -> return TokenBlockUpdateReport
```

`TokenBlockUpdateReport` should stay compact and delta-oriented. It is the handoff
from token source state to server/runtime consumers. It can include updated token
addresses, discovered/simulated pools, transaction failures, pool-simulation
failures, and active observation rows, but it should not require downstream code
to rescan every tracked token to understand what changed in this block.

Token analytics observations are source features only. Labels, active-horizon
targets, training rows, and page story aggregates are owned by
`token_lab/risk_atlas/scam_analytics/risk_atlas` and should be written/read through its DB.

For live trading, the important invariant is:

```text
trading/risk consumers read state only after eth_token has completed the block
and eth_chain_server has committed that state for the block.
```

Frontend snapshots and Risk Atlas exports are downstream read models. They should
not feed back into token state or block trading decisions.

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
- V2 candidate routing is intentionally split from full pool metadata. For
  unknown V2 pair events, candidate discovery should read only pool identity
  (`token0`, `token1`, protocol validation). Fetch decimals only when a pool is
  registered on a tracked token.
- Same-block complex deployments can require exact metadata/pool replay; avoid
  assuming token metadata is stable before the processed block is fully applied.
- UI/API callers should use explicit view DTOs from `eth_chain_server`, not raw
  internal token structs.
- Do not recreate log decoding here. If a field needs receipt/log/trace truth,
  add it to `tx_processor` first and consume the processed fact here.
