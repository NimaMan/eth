# Scam Mechanism Clusters

This taxonomy describes how the token or pool became unsafe. It should be based
on chain behavior, not on whether a strategy made money, lost money, entered, or
exited.

## Cluster Rules

- Assign one primary `mechanism` per token/pool label.
- Add secondary notes when multiple behaviors are present.
- Prefer `needs_mechanism_review` over a weak confident label.
- Use `unknown_reserve_drain` only when the bad outcome is verified but the
  chain mechanism is not yet isolated.

## Initial Clusters

| Mechanism | Definition | Early Attributes To Check |
| --- | --- | --- |
| `pair_balance_backdoor_drain` | Token balance is removed from or manipulated against the pair, often followed by a router sell or reserve collapse. | Pair token transfers not explained by swaps, `transferFrom(pair, actor)` anomalies, dead-address LP claims that do not prevent pair balance movement. |
| `direct_lp_liquidity_removal` | LP owner or privileged actor removes quote liquidity from the pool. | LP holder concentration, non-burned LP share, LP approvals, remove-liquidity calls. |
| `sell_blocked_honeypot` | Public buys succeed but public sells fail or are blocked. | Successful buys without successful non-owner sells, failed sell simulation, blacklist/whitelist storage changes, transfer revert signatures. |
| `tax_fee_flip` | Sell remains technically possible but effective tax becomes confiscatory after launch. | Net output collapses versus quote, transfer fee changes, owner-controlled fee setter calls. |
| `blacklist_or_whitelist_gate` | Selected holders or routers are prevented from selling or transferring. | Address-specific sell failures, whitelist-only transfer paths, owner blacklist events or storage writes. |
| `hidden_mint_supply_expansion` | Privileged mint or supply expansion is dumped into the pool. | Mint events, supply jumps, owner/deployer token inflows before reserve drain. |
| `privileged_seller_reserve_drain` | A privileged address sells or transfers tokens in a way normal holders cannot reproduce. | Successful privileged sells while retail sell simulation fails, deployer/owner inventory dumps. |
| `unknown_reserve_drain` | Quote reserve collapse is verified but the mechanism needs more chain truth. | Large reserve delta, price-to-init collapse, no confirmed mechanism yet. |

## Timing Buckets

Use these buckets for the first 100-token cohort summary:

| Bucket | Block Range | Approx Ethereum Time |
| --- | ---: | ---: |
| `same_block` | `0` | same block |
| `fast_1_to_10` | `1..10` | up to about 2 minutes |
| `early_11_to_100` | `11..100` | about 2 to 20 minutes |
| `delayed_101_to_1000` | `101..1000` | about 20 minutes to 3.3 hours |
| `late_gt_1000` | `>1000` | more than about 3.3 hours |

The bucket is computed from `label_block - trading_enabled_block`, not from
strategy entry.
