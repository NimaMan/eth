# Scam Pools Review Summary

Source run: `run-2`

This is a token-centric scam review export. Labels come from
token-server pool `scam_mechanism` evidence. Strategy entry, exit,
and PnL are intentionally ignored. Use `--limit 100` for the first
manual-review batch, or `--limit 0` for all currently available rows.

## Source Progress

- Range: `24994815..25094814`
- Status at export: `running`
- Blocks processed at export: `12550` / `100000`
- Pools available at export: `828`
- Rows exported: `100`

## Mechanisms

| Mechanism | Count |
| --- | ---: |
| `direct_lp_liquidity_removal` | 63 |
| `pair_balance_backdoor_drain` | 29 |
| `unknown_reserve_drain` | 6 |
| `reserve_dump_drain` | 2 |

## Confidence

| Confidence | Count |
| --- | ---: |
| `verified` | 92 |
| `probable` | 8 |

## Time To Scam Buckets

| Bucket | Count |
| --- | ---: |
| `early_11_to_100` | 67 |
| `delayed_101_to_1000` | 20 |
| `fast_1_to_10` | 13 |

## Protocols

| Protocol | Count |
| --- | ---: |
| `UNISWAP-V2` | 89 |
| `UNISWAP-V4` | 6 |
| `UNISWAP-V3` | 5 |

## Manual Review Notes

- `verified` means the token-server evidence includes the expected chain
  artifact for that mechanism, such as LP burn/removal or pair-balance
  transfer evidence.
- `probable` means the reserve-drain evidence is clear but the exact actor
  mechanism should still be manually checked.
- This batch is a draft review artifact. Do not treat it as a final
  production label set until the 100 rows are manually audited.
