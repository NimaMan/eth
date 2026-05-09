# Odd Behavior Catalog

This catalog lists behavior that should be reviewed by the token lab. Some
items are scams, some are numerical traps, and some are simulator/indexer parity
risks. Each confirmed item should eventually become a detector, a guardrail, or
a display rule.

## Pool Reserve Behavior

- `sync_without_transfer`: pool reserves change after a direct `sync()` without
  matching token or denom transfers in the same transaction.
- `reserve_discontinuity`: token or denom reserve changes by an extreme ratio
  compared with the previous sync.
- `denom_drain`: denom reserve drops below a safety threshold after prior
  meaningful liquidity.
- `token_reserve_dust`: token reserve becomes tiny relative to total supply,
  making AMM spot price meaningless.
- `lp_supply_zero_with_reserves`: LP total supply is zero or effectively zero
  while the pool still has reserves.

## Trading Behavior

- `cannot_sell`: simulated buy succeeds but simulated sell fails.
- `observed_sell_simulator_fail`: chain has a successful sell-like transaction,
  but our simulator cannot reproduce a sell at the same pre-state.
- `high_tax`: buy or sell tax exceeds the configured safety threshold.
- `tax_state_change`: tax or transfer behavior changes after a control
  transaction touches the token.
- `buy_only_activity`: many buys exist, but no normal sells exist.

## Token Control Behavior

- `owner_sync_or_control_tx`: creator or owner calls a control method that
  changes pool behavior or reserves.
- `hidden_mint_or_rebase`: token balance changes without normal transfer logs
  explaining the reserve movement.
- `trading_state_flip`: trading is enabled or disabled through a token control
  call that should trigger pool re-simulation.
- `renounce_after_setup`: ownership is renounced after unsafe state is already
  configured.

## Numerical Behavior

- `price_ratio_extreme_low_supply`: price ratio is extreme because the token
  reserve in pool is tiny.
- `fdv_extreme_low_supply`: FDV is extreme because price is computed from dust
  reserves.
- `supply_ratio_invalid`: pool token reserve exceeds total supply or cannot be
  reconciled with token decimals.
- `initial_price_unstable`: initial price is based on an early tiny or
  non-representative sync.
