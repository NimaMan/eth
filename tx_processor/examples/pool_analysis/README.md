# Pool Analysis Examples

These examples exercise `tx_processor::trade_simulation` against known DEX pool
types. The common flow is:

1. Build `PoolBuySellParameters` with token, pool, pool type, denom token, block,
   decimals, and optional prior transactions.
2. Run `check_can_buy_sell_pool` for buy -> approve -> sell viability, or
   `simulate_sell_swap_with_params` for sell-only probes.
3. Inspect `PoolBuySellSimulationResult` / `SellSwapResult` for tradeability,
   taxes, denom spent/received, and failure reasons.

## Protocol Coverage

- V2-style router protocols: Uniswap V2, SushiSwap, PancakeSwap V2, ShibaSwap
  V2, and Fraxswap V2 through `PoolType::known_v2_protocol`.
- V3 router protocols: known V3 routers through `PoolType::known_v3_protocol`.
- Uniswap V3 sell-only Universal Router flow: Permit2 + Universal Router exact
  input.
- Uniswap V4 pool viability and sell-only flow: Universal Router V4 with
  `UniswapV4PoolConfig`.

## Running

```bash
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example can_buy_sell_common_tokens_uniswap_v2
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example can_buy_sell_common_tokens_uniswap_v4
RETH_DATADIR=/path/to/reth/mainnet cargo run -p tx_processor --example probe_sell_swap
```

Most examples are intentionally concrete: addresses, fee tiers, blocks, and
decimals are set in the file so a failing run can be debugged against a known
historical state.
