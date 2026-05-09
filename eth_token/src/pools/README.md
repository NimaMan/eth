# pools

AMM pool state machines and pool-level liquidity/trading analysis.

This folder corresponds to Python modules under `erc20_token/pools`.

## Responsibilities

- Track pool reserves, prices, swaps, liquidity events, and trading status.
- Provide common pool abstractions used by Uniswap V2, V3, V4, and related pool types.
- Bridge pool state into token snapshots.
- Use Rust simulation APIs for buy/sell checks rather than Python wrappers.

## Layout

- `uniswap/`: Uniswap V2, V3, V4, and shared concentrated-liquidity helpers.
- `sushiswap/`: SushiSwap V2.
- `balancer/`: Balancer protocol implementations. Current tracked version: V2.
- `curve/`: Curve protocol implementations. Current tracked version: V1-style pool state.
- `base.rs`, `reserves.rs`, `tax.rs`, `trading_simulation.rs`: protocol-agnostic pool state and shared pool simulation helpers.

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

## Uniswap V2 Port Status

Implemented:

- Base reserve, price, liquidity, lifecycle, and reserve-threshold scam state.
- V2 sync/swap/mint/burn event handling.
- V2 trading status fields: can buy, can sell, buy tax, sell tax, and tax-check block/tx.
- Historical and live pool buy/sell simulation entry points.
- LP token tracking for V2 pair tokens: transfers, mint/burn supply effects, holders, holder shares, approvals, router approval percentage, last approval block, and last approval event.
- A chain-state parity example for LP total supply, holder balances, and allowances.

Still missing against Python:

- V3 and V4 pool implementations.
- Pool-manager style aggregation across V2/V3/V4.
- Liquidity matrix and best-price views.
- Denom symbol/name enrichment in the view layer.
- Token tax/max-buy events that should influence when trading simulation is retriggered.
