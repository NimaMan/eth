# Tracks

Tracks are typed, per-angle state. Each track answers one question and carries
its own evidence references. Tracks are allowed to coexist in combinations that
legacy lifecycle labels cannot express, such as a pool that is still in a
trading lifecycle while custody risk is latent and sell route simulation fails.

Behavior-risk tracks are additive and optional by default:

- transfer policy: blacklist, whitelist, pause, trading pause, sniper blacklist
- sell restrictions: max_tx, max_wallet, cooldown, one_sell_per_block, low sell
  limit
- tax policy: extreme sell tax, asymmetric buy/sell tax, modifiable tax,
  personalized tax, high sell gas
- contract posture: closed source, proxy, external policy call, hidden owner,
  ownership reclaimable, selfdestruct capable
- supply control: mintable, owner balance changes, holder burn, rebasing,
  reflection, fee-on-transfer
- behavioral outcomes: high sell fail rate, high siphon rate, buys without
  sells, sniper blacklist clusters, reused confiscation patterns

The aggregate type is `PoolTrackedState` in `state/model.rs`.
