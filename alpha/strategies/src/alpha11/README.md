# Alpha11 Strategy Family

Alpha11 is the deployable live strategy layer for the current ETH validation
run. It composes the reusable `baseline/snipe_all` engine and owns the product
defaults that should be explicit in the strategy name.

The current live-real promotion target is:

`alpha11-univ2-lp30-pool-update-block-hold16`

The live-backtest comparison sweep also includes hold12, hold14, hold15,
hold18, hold20, hold25, hold50, hold100, and the all-pools hold16 probe so the
selected hold window can be compared against nearby, longer-tail, and wider-pool
variants before promotion.

The all-pools live-backtest probe strategy is:

`alpha11-all-pools-lp30-pool-update-block-hold16`

That variant keeps the hold16, LP30, buy-size, liquidity-floor, init-policy,
exit, and bankroll defaults, but it removes the protocol allowlist from the
strategy spec. It is intended to measure all eligible pools visible to the
shared entry and simulation stack. It does not make non-executable pools
tradeable: the shared entry rules still reject unsupported quote assets, hooked
V4 pools, missing V4 metadata, or any route the simulator cannot execute. This
variant is a live-backtest probe, not the current live-real deployment target.

The controlled public validation variant is:

`alpha11-univ2-lp30-pool-update-block-hold3-validation`

That variant has the same Uniswap V2, LP30, pool-update-block, and deployed V2
vault assumptions, but it uses hold3 and a `0.01 ETH` entry bankroll so we can
prove one mined buy, one mined sell, receipt reconciliation, tx index, actual
gas cost, and finality recheck before enabling the main hold16 strategy.
It also carries `max_entry_pools = 1` in the strategy spec. Live runs must not
pass separate strategy parameters such as buy size, liquidity floors, bankroll,
entry-pool cap, or hold blocks for Alpha11.

The systematic operator checklist for this one-position mined validation lives
in `alpha/live/readiness/gates/pre_live_mined_validation/`. Alpha11's concrete
validation file is
`alpha/live/readiness/strategies/alpha11/hold3_mined_validation.md`. A failed
Kartal or signer policy rejection is still useful gate evidence when it records
the exact rejected request and journal reason; it does not count as a passed
mined-validation trade.

The live-real deploy path carries the shared entry init policy explicitly in
strategy config. The first concrete Gate 2 setting rejects entries when pool age
is above `5` blocks or `price / initial price` is above `2`.

Shared Alpha11 defaults:

- `univ2`: only enter Uniswap V2 pools.
- `all-pools`: no protocol allowlist; entry still requires an executable
  route and valid pool metadata.
- `lp30`: block entry when LP approval exceeds the shared 30% gate.
- Exit an open position on liquidity-removal risk events. Both confirmed-chain
  `liquidity_removal` and pending `mempool_liquidity_removal` events are
  treated as sell signals.
- Exit an open position on LP approval risk events after entry, except for
  launch-window approvals within `<=2` chain blocks of trading enabled, which
  are deferred to max-hold. In live runs this can come from `mempool_signal`
  before the approval is mined; in mined-chain/risk-atlas replay it comes from
  confirmed-chain evidence. Reports should preserve the source so `mempool
  lp_approval` is not confused with `mined-chain lp_approval`.
- Buy size is `0.01 ETH` per entry.
- Liquidity floors are `0.5 ETH` for ETH/WETH pools and `1000` for USD-stable
  quote pools.
- `pool-update-block-hold16`: force exit after 16 distinct pool-update blocks
  while the position is open.
- `entry_init_policy`: only enter pools with age `<= 5` blocks and
  `price / initial price <= 2` when that evidence is available.
- Initial entry bankroll is `0.555 ETH`; buys consume bankroll, confirmed sells
  replenish it, and realized profit can be redeployed.

## Entry Gates

Alpha11 entries use all reusable `SnipeAllStrategy` entry checks plus the
Alpha11-owned launch defaults above.

Gate 2 is the shared init policy for already-active pools. The initial Alpha11
deployment threshold is deliberately tight while Risk Atlas continues to
compare pool age at entry, current price/initial, liquidity, and realized
outcomes.

## Risk Signal Sources

Alpha11 records the source of each risk-driven decision. The important sources
for deploy review are:

- `mempool_signal`: pending public mempool evidence, such as LP approval or
  remove-liquidity transactions before they are mined.
- `risk_atlas_mined_chain`: confirmed-chain evidence derived from mined blocks.
- `market`: pool-update or max-hold decisions, not a risk-signal source.

For `alpha11-univ2-lp30-pool-update-block-hold16`, a sell reason of
`exit.lp_approval` means the strategy exited on LP-token approval evidence. A
hold reason with code
`exit.lp_approval.early_approval_deferred_to_max_hold` means the approval was
inside the `<=2` active-block launch window and the position remains governed by
max hold. Those deferrals must carry the signal id, approval percentage, age
basis, active-block age, and matching risk-event evidence in the persisted
strategy decision details. Mined-chain replay records a deterministic
`risk_atlas_mined_lp_approval:<block>:<token>:<pool>` source event id in the
same evidence fields so it can be audited with the same checks as mempool
signals.
A sell reason of `exit.mempool_liquidity_removal_signal` means it exited on a
pending remove-liquidity transaction. A sell reason of `exit.liquidity_removal`
means confirmed-chain liquidity removal was already visible.

## Layout

| Path | Responsibility |
|------|----------------|
| `config.rs` | Alpha11 constants and wrapper config. |
| `strategy.rs` | Thin Alpha11 wrapper around the reusable `SnipeAllStrategy`. |
| `live/` | Live runtime wrapper and named live specs exposed to the trader registry. |
| `presets/` | Concrete product presets used by tests, docs, and future launch tooling. |

Keep reusable entry/exit logic in `baseline/snipe_all` or `shared_rules`.
Alpha11 should only own product defaults, lifecycle composition, and named
presets.
