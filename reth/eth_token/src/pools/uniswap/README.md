# uniswap

Uniswap-family pool implementations.

This folder owns concrete Uniswap protocol versions while shared pool state remains in `pools::base`, `pools::data_models`, and `pools::reserves`.

## Layout

- `v2.rs`: Uniswap V2-style pair pools, sync/swap/mint/burn events, and LP token tracking.
- `v3.rs`: Concentrated-liquidity pools, tick state, mint/burn/swap handling.
- `v4.rs`: PoolManager-based pools, PoolId handling, balance deltas, and hook-aware state.
- Future `common.rs`: shared Uniswap helpers if V2/V3/V4 start duplicating orientation or event conversion logic.

## Boundaries

- Decoded transaction/log events come from `tx_processor`.
- Simulation belongs to existing Rust simulator APIs.
- Protocol-agnostic pool state belongs one level up in `pools`.

## Import Style

Prefer protocol-specific paths in implementation code:

```rust
use eth_token::pools::uniswap::v2::UniswapV2Pool;
```

The top-level `eth_token::pools::UniswapV2Pool` re-export is kept for ergonomic callers.
