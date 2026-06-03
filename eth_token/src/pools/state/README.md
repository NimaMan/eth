# Pool State Tracking

This module splits overloaded pool state into orthogonal tracking angles. The
current `BasePool`, `PoolRuntimeState`, and `PoolStateFlags` remain in place;
this folder adds a first typed layer that projects from those structures without
migrating persistence or runtime internals.

## Flow

Raw observations feed per-angle track states. Track states are the typed source
for derived labels and consumer views.

1. Raw observations: pool events, reserve snapshots, trade simulations, custody
   findings, PnL counterparty roles, liquidity observations, and behavior-risk
   scanner/simulation observations.
2. Per-angle track states: identity, market structure, lifecycle, liquidity,
   routeability, valuation, activity, LP control, custody, risk, roles,
   transfer policy, sell restrictions, tax policy, contract posture, supply
   control, behavioral outcomes, eligibility, and evidence quality.
3. Derived labels: compact strings for UI filters, reporting, compatibility,
   and migration bridges.
4. Consumer views: PnL state, Risk Atlas state, and live-trading state.

Labels and DB rows are projections. They are not the source of truth. The source
of truth is the observation history plus the typed track state derived from it.
During this transition, `BasePool` and `PoolStateFlags` are compatibility inputs
for building `PoolTrackedState`.

## Tracking Angles

| Angle | Question | Example |
| --- | --- | --- |
| Identity | What is this pool? | pool=0x6053..., token=0x3832..., denom=WETH, protocol=uniswap_v2, creator=0x... |
| Market structure | How does the pool work? | V2 token orientation; V3 fee_tier=3000/tick spacing; V4 pool key/hooks |
| Lifecycle | What broad phase is it in? | discovered -> liquidity_deposited -> trading -> liquidity_removed |
| Liquidity | Are reserves economically meaningful? | 11.65 WETH reserve active; 0.00001 WETH dust; 0 WETH drained |
| Routeability | Can we execute buy/sell now? | raw_can_buy=true, raw_can_sell=false means buy works but sell simulation fails |
| Valuation | Can we price positions safely? | priced from reserves; terminal_zero after drain; no_mark when no usable price |
| Activity | What has actually happened? | 217 swaps, buy volume 47 WETH, sell volume 36 WETH, latest swap block 25227098 |
| LP / ownership | Who controls liquidity? | LP owned by creator, LP burned/locked, LP approval exposure >20% |
| Custody / token control | Can token balances be changed by control actors? | latent owner transferFrom power; realized holder balance drain; buyer token confiscation |
| Transfer policy | Can transfers/trading be selectively blocked? | blacklist, whitelist, pause, trading pause, sniper blacklist |
| Sell restrictions | Can holders sell normal sizes at normal cadence? | max_tx, max_wallet, cooldown, one_sell_per_block, low_sell_limit |
| Tax policy | Can token taxes make exit unsafe or mutable? | extreme sell tax, asymmetric buy/sell tax, modifiable tax, personalized tax, high sell gas |
| Contract posture | What bytecode/source posture increases behavior risk? | closed_source, proxy, external_policy_call, hidden_owner, ownership_reclaimable, selfdestruct_capable |
| Supply control | Can supply or balances be changed outside normal transfers? | mintable, owner_can_change_balance, owner_can_burn_holder, rebasing, reflection, fee_on_transfer |
| Behavioral outcomes | What early observed outcomes indicate scam behavior? | high_sell_fail_rate, siphon_rate_high, sells_absent_after_buys, sniper_blacklist_cluster, reused_confiscation_pattern |
| Risk mechanism | Why is this unsafe? | holder_balance_backdoor_drain, pair_balance_backdoor_drain, direct_lp_liquidity_removal |
| Counterparty roles | Who are actors in tx flow? | user trader; Banana Gun router pass-through; pool; WETH bridge |
| Eligibility | Is this pool in our tradable/research cohort? | eligible active WETH pool; ineligible low-liquidity pool; eligible risk pool after scam |
| Evidence quality | How confident are we? | event-backed reserve update, simulation-backed sell failure, trace-backed custody drain |

## Behavior-Risk Placeholder Tracks

The transfer policy, sell restriction, tax policy, contract posture, supply
control, and behavioral outcome tracks are intentionally observation-fed. They
do not require bytecode parser, verified-source, trace, or live simulation
integration yet. Scanner and adapter work can populate the typed observation
models in `observations/`; this layer will fold those observations into stable
track fields, labels, and views.

Current stable labels include:

- `policy:blacklist_present`, `policy:whitelist_present`,
  `policy:pause_present`, `policy:trading_pause_present`,
  `policy:sniper_blacklist_present`
- `restriction:max_tx`, `restriction:max_wallet`, `restriction:cooldown`,
  `restriction:one_sell_per_block`, `restriction:low_sell_limit`
- `tax:extreme_sell_tax`, `tax:asymmetric_buy_sell_tax`, `tax:modifiable`,
  `tax:personalized`, `tax:high_sell_gas`
- `contract:closed_source`, `contract:proxy`,
  `contract:external_policy_call`, `contract:hidden_owner`,
  `contract:ownership_reclaimable`, `contract:selfdestruct_capable`
- `supply:mintable`, `supply:owner_can_change_balance`,
  `supply:owner_can_burn_holder`, `supply:rebasing`, `supply:reflection`,
  `supply:fee_on_transfer`
- `behavior:high_sell_fail_rate`, `behavior:siphon_rate_high`,
  `behavior:sells_absent_after_buys`, `behavior:sniper_blacklist_cluster`,
  `behavior:reused_confiscation_pattern`

## Compatibility Boundary

`PoolTrackedState::from_base_pool` is the current integration hook. It projects
from `BasePool` and `PoolStateFlags` into the base typed tracks, with the new
behavior-risk tracks defaulting empty until observations are attached. Existing
DB rows, flat labels, and UI-ready flags can continue to be emitted as
projections while the runtime internals remain stable.
