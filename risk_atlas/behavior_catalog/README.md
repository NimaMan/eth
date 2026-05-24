# Odd Behavior Catalog

This catalog lists behavior that should be reviewed through Risk Atlas
investigations. Some
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

## Scam Mechanism Families

### `pair_balance_backdoor_drain`

Production label: `pair_balance_backdoor_drain`

Display label: `Backdoored Pair-Balance Drain`

This mechanism covers cases where the pair's token balance is changed through
token-contract behavior rather than a normal LP burn/removal path. The pool may
still have burned or locked LP, so an LP-only review can incorrectly conclude
the pool is safe. The important evidence is that token balance leaves or
collapses around the pair, then a later swap or reserve update drains the quote
side.

Common evidence:

- token transfer or balance movement from the pair that is not explained by a
  normal pool event;
- reserve discontinuity after meaningful prior liquidity;
- direct `sync()` that updates reserves to an already-manipulated pair balance;
- quote/denom reserve drain in the same block or soon after;
- creator/control-address involvement.

The `sync()` call is not itself the drain. `sync()` is a normal V2 pair method
that copies current token balances into the pair's stored reserves. The scam
signal is the combination of an unexplained pair-balance change plus `sync()`
making that manipulated balance visible to the AMM state. SSS is the anchor
case for the `sync_without_transfer + reserve_discontinuity +
token_reserve_dust` variant.

### `privileged_seller_reserve_drain`

Production label: `privileged_seller_reserve_drain`

Display label: `Privileged Seller Reserve Drain`

Use this when a normal retail route is blocked or fails, but a privileged or
otherwise special seller can still sell/dump and drain the quote reserve. The
key distinction from `pair_balance_backdoor_drain` is that the drain is
explained by a sell/swap path rather than a pair-balance mutation.
