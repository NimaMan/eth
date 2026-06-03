# Pool State Tracking

This module splits overloaded pool state into orthogonal tracking angles. The
current `BasePool`, `PoolRuntimeState`, and `PoolStateFlags` remain in place;
this folder adds a first typed layer that projects from those structures without
migrating persistence or runtime internals.

## Flow

Raw observations feed per-angle track states. Track states are the typed source
for derived labels and consumer views.

1. Raw observations: pool events, reserve snapshots, trade simulations, custody
   findings, PnL counterparty roles, and liquidity observations.
2. Per-angle track states: identity, market structure, lifecycle, liquidity,
   routeability, valuation, activity, LP control, custody, risk, roles,
   eligibility, and evidence quality.
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
| Risk mechanism | Why is this unsafe? | holder_balance_backdoor_drain, pair_balance_backdoor_drain, direct_lp_liquidity_removal |
| Counterparty roles | Who are actors in tx flow? | user trader; Banana Gun router pass-through; pool; WETH bridge |
| Eligibility | Is this pool in our tradable/research cohort? | eligible active WETH pool; ineligible low-liquidity pool; eligible risk pool after scam |
| Evidence quality | How confident are we? | event-backed reserve update, simulation-backed sell failure, trace-backed custody drain |

## Compatibility Boundary

`PoolTrackedState::from_base_pool` is the current integration hook. It projects
from `BasePool` and `PoolStateFlags` into the new typed tracks. Existing DB rows,
flat labels, and UI-ready flags can continue to be emitted as projections while
the runtime internals remain stable.
