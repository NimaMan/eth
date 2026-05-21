# Alpha11 Strategy Family

Alpha11 is the deployable live strategy layer for the current ETH validation
run. It composes the reusable `baseline/snipe_all` engine and owns the product
defaults that should be explicit in the strategy name:

- `univ2`: only enter Uniswap V2 pools.
- `lp30`: block entry when LP approval exceeds the shared 30% gate.
- `pool-update-block-hold15`: force exit after 15 distinct pool-update blocks
  while the position is open.
- Initial entry bankroll is `0.225 ETH`; buys consume bankroll, confirmed sells
  replenish it, and realized profit can be redeployed.

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
