# Uniswap v4 Track

This track hosts everything specific to supporting Uniswap v4 pools with the Baygus Router.

## Goals

1. Reverse-engineer behaviour of production routers and document it in `../references`.
2. Define the minimal adapter interface that the core router will call into when the target venue
   is Uniswap v4.
3. Implement and test:
   - PoolManager lock lifecycle management.
   - Hook-aware swap execution (including hook data propagation).
   - Settlement/take for both ERC-20 and native currencies.
   - Safety checks (reentrancy, slippage, fee caps).

## Deliverables

- `AdapterUniswapV4.sol` (or equivalent) inside `../contracts/src/adapters/`.
- Integration tests that simulate swaps against canonical pool setups (single pool, multi-hook,
  dynamic fee).
- Documentation in `../docs/` covering:
  - State machine for `lock → swap → settle`.
  - Hook invocation expectations.
  - Gas optimisation and griefing considerations.

As we extend the router to other venues, mirror this structure (e.g. create `sushiswap-v2/`,
`curve/`) so each protocol keeps its research, specifications, and adapters isolated yet consistent.
