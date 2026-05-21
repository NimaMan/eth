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
The hold window is part of the named strategy spec. Live runs must not pass a
separate `--max-hold-blocks` parameter for Alpha11.

The live-real deploy path adds an entry-only `price / initial price <= 1.5` cap
as runtime config. We intentionally do not encode that cap in the visible
strategy name; the cap must instead be visible in README/front-end readiness
copy and in the persisted strategy config field
`max_entry_price_ratio_to_initial`.

Shared Alpha11 defaults:

- `univ2`: only enter Uniswap V2 pools.
- `lp30`: block entry when LP approval exceeds the shared 30% gate.
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
