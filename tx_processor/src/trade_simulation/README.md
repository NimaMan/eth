# Trade Simulation

`trade_simulation` owns tx-processor-level trading workflows. It builds or
receives unsigned transactions, runs them through `tx_simulator`, and converts
each simulated step into `ProcessedTransaction` values for tax, balance, and
tradeability analysis.

## Layout

| Path | Responsibility |
| --- | --- |
| `buy_swap_simulator.rs` | Single buy swap simulation. |
| `sell_swap/` | Sell-only simulation split into router protocols, Uniswap Universal Router flows, balance setup, denom extraction, and common result helpers. |
| `pool_buy_sell_simulator/` | Buy -> approve -> sell viability, prior transaction replay, buyer setup, fee policy, validation, and tax extraction. |
| `cross_venue_buy_approve_sell.rs` | Cross-venue buy/approve/sell sequence helper. |
| `types.rs` | `PoolType`, `PoolBuySellParameters`, results, and defaults. |

## Sell Swap Split

- `sell_swap/router_protocols.rs`: generic V2/V3 router paths, covering
  non-Uniswap protocols represented by `PoolType` such as SushiSwap,
  PancakeSwap V2, PancakeSwap V3, ShibaSwap V2, Fraxswap V2, and SushiSwap V3.
- `sell_swap/uniswap_v3_universal_router.rs`: Uniswap V3 Universal Router sell
  flow with ERC20 approval, Permit2 approval, and exact-input swap.
- `sell_swap/uniswap_v4_universal_router.rs`: Uniswap V4 Universal Router sell
  flow using `UniswapV4PoolConfig`.
- `sell_swap/balance_setup.rs`: synthetic seller token balance setup, including
  standard ERC20 balance-slot probing and recipient-transfer-diff fallback.
- `sell_swap/denom_output.rs`: denom received extraction from processed balance
  changes and pool transfer events.

## Boundaries

- Low-level calldata builders live in `tx_simulator::tx_builders`.
- Raw EVM execution and DB-backed simulation live in `tx_simulator`.
- Processed tx and block decoding live in `tx_processor/` and
  `block_processor/`.

## Execution Viability Checks

Trade simulation answers the execution question:

> Can this route buy and sell this token from the same pre-state that existed
> on chain?

Required checks:

- simulate buy at the target block;
- simulate sell after the buy;
- compare with observed on-chain buys and sells when they exist;
- classify failures as token behavior, router behavior, state setup, or
  simulator bug;
- record tax and received amounts in denom units.

Observed on-chain sells are especially important. If the chain has a successful
sell but this simulator says selling is impossible, the route and sender must be
replayed before that simulator result is used as a trading guardrail.
