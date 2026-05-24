# Scam Pools Review Summary

Source run: `run-2`

This is a token-centric scam review export. Labels come from
token-server pool `scam_mechanism` evidence. Strategy entry, exit,
and PnL are intentionally ignored. Use `--limit 100` for the first
manual-review batch, or `--limit 0` for all currently available rows.

## Source Progress

- Range: `24994815..25094814`
- Status at export: `completed`
- Blocks processed at export: `100000` / `100000`
- Pools available at export: `10506`
- Rows exported: `7626`

## Mechanisms

| Mechanism | Count |
| --- | ---: |
| `direct_lp_liquidity_removal` | 3565 |
| `unknown_reserve_drain` | 2180 |
| `pair_balance_backdoor_drain` | 1152 |
| `reserve_dump_drain` | 708 |
| `privileged_seller_reserve_drain` | 21 |

## Confidence

| Confidence | Count |
| --- | ---: |
| `verified` | 4717 |
| `probable` | 2909 |

## Time To Scam Buckets

| Bucket | Count |
| --- | ---: |
| `early_11_to_100` | 4616 |
| `delayed_101_to_1000` | 1662 |
| `late_gt_1000` | 778 |
| `fast_1_to_10` | 525 |
| `same_block` | 45 |

## Protocols

| Protocol | Count |
| --- | ---: |
| `UNISWAP-V2` | 4737 |
| `UNISWAP-V4` | 2765 |
| `UNISWAP-V3` | 117 |
| `PANCAKESWAP-V2` | 3 |
| `SUSHISWAP-V3` | 3 |
| `SUSHISWAP-V2` | 1 |

## Manual Review Notes

- `verified` means the token-server evidence includes the expected chain
  artifact for that mechanism, such as LP burn/removal or pair-balance
  transfer evidence.
- `probable` means the reserve-drain evidence is clear but the exact actor
  mechanism should still be manually checked.
- This batch is a draft review artifact. Do not treat it as a final
  production label set until the 100 rows are manually audited.
