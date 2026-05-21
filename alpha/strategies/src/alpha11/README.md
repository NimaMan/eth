# Alpha11 Strategy Family

Alpha11 is the deployable live strategy layer for the current ETH validation
run. It composes the reusable `baseline/snipe_all` engine and owns the product
defaults that should be explicit in the strategy name:

- `univ2`: only enter Uniswap V2 pools.
- `lp30`: block entry when LP approval exceeds the shared 30% gate.
- `price-to-initial-lte1p5`: block entry when the known pool price-to-initial
  ratio is greater than `1.5`; missing ratio data does not block entry.
- `pool-update-block-hold15`: force exit after 15 distinct pool-update blocks
  while the position is open.
- Initial entry bankroll is `0.225 ETH`; buys consume bankroll, confirmed sells
  replenish it, and realized profit can be redeployed.

## Entry Gates

Alpha11 entries use all reusable `SnipeAllStrategy` entry checks plus the
Alpha11-owned launch defaults above. The price-to-initial gate is intentionally
part of the strategy name because it changes the buy universe:

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
