# Live Non-Capital Backtest: Max-Hold 50

Run ID: `live-noncapital-maxhold50-twoweek-requested-20260514-1020`

Replay source: `snipe-all-v1-chain-sim-live-v4`

Requested window: `24,991,345..25,092,144` (`100,800` blocks, roughly two
weeks at 12 second blocks).

Actual observation coverage is partial. The replay source reaches the live
head, but pool observations start at block `25,066,498`, so this is a
two-week-requested current-regime run, not a complete two-week historical
replay.

## Command

```bash
RUN_ID=live-noncapital-maxhold50-twoweek-requested-20260514-1020
RUST_LOG=info cargo run --release -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id "$RUN_ID" \
  --replay-run-id snipe-all-v1-chain-sim-live-v4 \
  --from-block 24991345 \
  --to-block 25092144 \
  --skip-primed \
  --buy-amount-wei 10000000000000000 \
  --min-liquidity-eth 0.5 \
  --min-liquidity-usd 1000 \
  --max-hold-blocks 50
```

Backtest completion:

```text
events_processed=127878
reports_generated=1089
confirmed_reports=983
failed_reports=106
positions=533
open_positions=59
```

## Source Coverage

| Event source | Rows | Suppressed | Usable | Min block | Max block |
| --- | ---: | ---: | ---: | ---: | ---: |
| `pool_update` | 30,819 | 3,741 | 27,078 | 25,066,498 | 25,092,144 |
| `mempool_signal` | 299 | 0 | 299 | 25,073,547 | 25,091,886 |

Mempool signals were not included in this run. This matches the previous
`maxhold50` candidate comparison and keeps this run pool-only.

## Strategy Lab Summary

| Metric | Value |
| --- | ---: |
| Positions | 533 |
| Open positions | 59 |
| Failed positions | 71 |
| Buy failed positions | 12 |
| Sell failed positions | 59 |
| Entry cost ETH | 5.21 |
| Realized PnL ETH | 13.466052023155642365 |
| Unrealized PnL ETH | -0.59 |
| Total PnL ETH | 12.876052023155642365 |
| ROI on open cost | 21.823816988399393839 |
| Snapshots | 8,922 |
| Positions with snapshots | 521 |
| Open without snapshot | 0 |

## Concentration

| Metric | ETH |
| --- | ---: |
| Total PnL | 12.876052023155642365 |
| Excluding top 1 | 10.786898210103026081 |
| Excluding top 2 | 9.299122389974037897 |
| Excluding top 5 | 7.630085911299305152 |
| Excluding top 10 | 5.913359147437672841 |

This is materially better than the earlier `maxhold200` concentration profile:
the run stays positive after removing the top 10 winners.

## Failure Buckets

| Error class | Reports |
| --- | ---: |
| `Sell transaction failed: TransferHelper: TRANSFER_FROM_FAILED` | 88 |
| `v4 universal router buy reverted` | 7 |
| `Buy transaction failed: UniswapV2: TRANSFER_FAILED` | 3 |
| `Universal Router V4 sell transaction failed: Empty revert payload...` | 3 |
| `Buy transaction failed: TF` | 2 |
| `unsupported balance storage layout` | 2 |
| `Sell transaction failed: UniswapV2: INSUFFICIENT_OUTPUT_AMOUNT` | 1 |

The main policy issue remains failed-exit exposure. There are 59 sell-failed
positions, all visible in Strategy Lab, and they need retry/chunk/skip rules
before this policy can support real execution.

## Top 10 Winners

| Rank | Token | Entry | Snapshot | PnL ETH | ROI |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1 | `0x12a77658112Cf42914cB614D13653ed5852DA1e5` | 25,077,695 | 25,077,746 | 2.089153813052616284 | 208.9153813052616284 |
| 2 | `0x0654750A6012bD42e06a0f1141153Dc8C60F7Fb1` | 25,077,320 | 25,077,371 | 1.487775820128988184 | 148.7775820128988184 |
| 3 | `0x3B0e37179fC806302f4A15f85577a06A6Eb38B4F` | 25,074,419 | 25,074,470 | 0.691971070602529721 | 69.1971070602529721 |
| 4 | `0x0Cf71133561eA2bEBf43e0A65e8fC96E6719Cc1F` | 25,073,665 | 25,073,716 | 0.500913620886640182 | 50.0913620886640182 |
| 5 | `0x7F48956fc95308bF1B80a647d8e1100a76C6d5F9` | 25,075,641 | 25,075,692 | 0.476151787185562842 | 47.6151787185562842 |
| 6 | `0xa45Baf1b087fD5db85E7a24a6c1F3497bEE329EB` | 25,076,207 | 25,076,258 | 0.413885007425812446 | 41.3885007425812446 |
| 7 | `0xd32D36D899a3C09E39284eC0Ff3E5BBae9043A16` | 25,078,431 | 25,078,482 | 0.395615203268878671 | 39.5615203268878671 |
| 8 | `0x1c8e311cba6B8E2a0257c143abd2955CD9bF4678` | 25,086,643 | 25,086,694 | 0.315486331206024438 | 31.5486331206024438 |
| 9 | `0xC3439980582F4f2acf8be59F1dcdEa5aB6e4AFDd` | 25,077,061 | 25,077,112 | 0.304647209178342520 | 30.464720917834252 |
| 10 | `0x7C83E4aA6E7b89e219f4Dfce41407bDac97Bd94b` | 25,079,971 | 25,080,022 | 0.287093012782574236 | 28.7093012782574236 |

Spot check: rank 1 entered a buyable/sellable Uniswap V2 WETH pool at block
`25,077,695`, sold after the max-hold monitor at block `25,077,746`, and
Token Lab checks passed for replay join, entry confirmation, raw/decimal
consistency, snapshot presence, and liquidity threshold.

## Worst 10 Losers

| Rank | Token | Entry | Snapshot | PnL ETH | ROI |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1 | `0x4A494ed91F36088e28c3fd6aDD31B390D6564989` | 25,075,193 | 25,075,244 | -0.01 | -1 |
| 2 | `0x91295c422502F622326697c304B283763FCc2421` | 25,075,116 | 25,075,167 | -0.01 | -1 |
| 3 | `0x22D6CB86a0B3CE004A3852de8085d90b8f35F7C1` | 25,074,557 | 25,074,608 | -0.01 | -1 |
| 4 | `0xed38B4DA9e6Ea4B05599a0D47983dC993e2F0B10` | 25,074,654 | 25,074,705 | -0.01 | -1 |
| 5 | `0xA609AF74A8bCf3C9698Ca20eB94EA425D50f6e19` | 25,074,480 | 25,074,531 | -0.01 | -1 |
| 6 | `0xA65D6d7d06E21C7CF1166a13E71253e54CdA6035` | 25,078,308 | 25,078,359 | -0.01 | -1 |
| 7 | `0xF3A793883B9091C652D5DD181D037FBD68b2C5c1` | 25,074,239 | 25,074,290 | -0.01 | -1 |
| 8 | `0xb468eC67e6739e616695695AEBC2c4F1bf42DF82` | 25,074,528 | 25,074,579 | -0.01 | -1 |
| 9 | `0xBfB71dDd3055a01Cc2eE8BA37237ec1F9b0C225c` | 25,074,175 | 25,074,226 | -0.01 | -1 |
| 10 | `0x4fD80A8867e0e55D5422CD611391B23033130325` | 25,074,185 | 25,075,260 | -0.01 | -1 |

Spot check: worst rank 1 entered a buyable/sellable Uniswap V2 WETH pool at
block `25,075,193`, then the max-hold exit failed and the position remained
`sell_failed`. Token Lab shows the pool later fell to near-zero reserve and
became non-buyable/non-sellable by block `25,075,239`.

## Read

`maxhold50` remains the best current live non-capital candidate from the runs we
have: it is profitable, less concentrated than max-hold 200, and the top/worst
mechanics match the expected 51-block monitor behavior.

It is not ready for real execution. The next blocker is failed-exit policy and
decision-ledger explainability, followed by true two-week replay coverage.

## Retry Comparison

After this baseline, bounded failed-exit retry support was added behind explicit
CLI flags and tested with:

```bash
RUN_ID=live-noncapital-maxhold50-retry20x3-twoweek-requested-20260514-1030
RUST_LOG=info cargo run --release -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id "$RUN_ID" \
  --replay-run-id snipe-all-v1-chain-sim-live-v4 \
  --from-block 24991345 \
  --to-block 25092144 \
  --skip-primed \
  --buy-amount-wei 10000000000000000 \
  --min-liquidity-eth 0.5 \
  --min-liquidity-usd 1000 \
  --max-hold-blocks 50 \
  --exit-retry-interval-blocks 20 \
  --max-exit-retries 3
```

Result:

| Metric | Baseline | Retry 20x3 |
| --- | ---: | ---: |
| Execution reports | 1,089 | 1,197 |
| Confirmed reports | 983 | 983 |
| Failed reports | 106 | 214 |
| Positions | 533 | 533 |
| Open positions | 59 | 59 |
| Sell-failed positions | 59 | 59 |
| Total PnL ETH | 12.876052023155642365 | 12.876052023155642365 |
| PnL ex top 10 ETH | 5.913359147437672841 | 5.913359147437672841 |

Read: bounded retry worked mechanically but did not recover stuck exits in this
window. It only added failed reports. Do not promote retry-only as the next
policy; the next useful policy needs chunk sizing, no-observed-sell filtering,
or earlier risk exits before the pool becomes unsellable.
