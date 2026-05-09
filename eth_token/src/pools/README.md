# pools

AMM pool state machines and pool-level liquidity/trading analysis.

This folder corresponds to Python modules under `erc20_token/pools`.

## Responsibilities

- Track pool reserves, prices, swaps, liquidity events, and trading status.
- Provide common pool abstractions used by Uniswap V2, V3, V4, and related pool types.
- Bridge pool state into token snapshots.
- Use Rust simulation APIs for buy/sell checks rather than Python wrappers.

## Boundaries

- Event decoding comes from `tx_processor`.
- Swap simulation comes from existing Rust simulator modules.
- Token-level transfer state belongs in `state`.
- Network graph construction belongs in `network`.

## First Port Targets

1. `pool_data_models.py` as Rust structs.
2. `base_pool.py` state transitions that are protocol-agnostic.
3. `uniswap_v2_pool.py`, then V3, then V4.
4. Pool manager orchestration.
