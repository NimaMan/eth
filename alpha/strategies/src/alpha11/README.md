# Alpha11 Strategy Family

Alpha11 is the deployable live strategy layer for the current ETH validation
run. It composes the reusable `baseline/snipe_all` engine and owns the product
defaults that should be explicit in the strategy name.

The visible live-backtest and live-real hold15 strategy name remains:

`alpha11-live-univ2-lp30-pool-update-block-hold15`

The controlled public validation variant is:

`alpha11-live-univ2-lp30-pool-update-block-hold3-validation`

That variant has the same Uniswap V2, LP30, pool-update-block, and deployed V2
vault assumptions, but it uses hold3 and a `0.01 ETH` entry bankroll so we can
prove one mined buy, one mined sell, receipt reconciliation, tx index, actual
gas cost, and finality recheck before enabling the main hold15 strategy.
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

The live-real deploy path adds an entry-only `price / initial price <= 1.5` cap
as runtime config. We intentionally do not encode that cap in the visible
strategy name; the cap must instead be visible in README/front-end readiness
copy and in the persisted strategy config field
`max_entry_price_ratio_to_initial`.

Shared Alpha11 defaults:

- `univ2`: only enter Uniswap V2 pools.
- `lp30`: block entry when LP approval exceeds the shared 30% gate.
- Exit an open position on liquidity-removal risk events. Both confirmed-chain
  `liquidity_removal` and pending `mempool_liquidity_removal` events are
  treated as sell signals.
- Exit an open position on LP approval risk events after entry. In live runs
  this can come from `mempool_signal` before the approval is mined; in
  mined-chain/risk-atlas replay it comes from confirmed-chain evidence. Reports
  should preserve the source so `mempool lp_approval` is not confused with
  `mined-chain lp_approval`.
- Buy size is `0.01 ETH` per entry.
- Liquidity floors are `0.5 ETH` for ETH/WETH pools and `1000` for USD-stable
  quote pools.
- `pool-update-block-hold15`: force exit after 15 distinct pool-update blocks
  while the position is open.
- Initial entry bankroll is `0.225 ETH`; buys consume bankroll, confirmed sells
  replenish it, and realized profit can be redeployed.

## Entry Gates

Alpha11 entries use all reusable `SnipeAllStrategy` entry checks plus the
Alpha11-owned launch defaults above.

The price-to-initial gate is not part of the live-backtest reference. It is a
live-real deployment default for the candidate we are trying to deploy:

- `price_ratio_to_initial > 1.5`: do not buy.
- `price_ratio_to_initial == 1.5`: buy is still allowed if all other gates pass.
- missing `price_ratio_to_initial`: buy is still allowed if all other gates pass.

This gate uses the pool snapshot's current price divided by the first valid
tracked pool price. It is an entry-only filter; it does not force sells for
already-open positions.

## Risk Signal Sources

Alpha11 records the source of each risk-driven decision. The important sources
for deploy review are:

- `mempool_signal`: pending public mempool evidence, such as LP approval or
  remove-liquidity transactions before they are mined.
- `risk_atlas_mined_chain`: confirmed-chain evidence derived from mined blocks.
- `market`: pool-update or max-hold decisions, not a risk-signal source.

For `alpha11-live-univ2-lp30-pool-update-block-hold15`, a sell reason of
`exit.lp_approval` means the strategy exited on LP-token approval evidence. A
sell reason of `exit.mempool_liquidity_removal_signal` means it exited on a
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
