# 70K Strategy Comparison - 2026-05-13

Replay source: `snipe-all-v1-chain-sim-live-v4`

Requested replay window: `25018863..25088862` (70,000 blocks).

Coverage caveat: pool observations in this source start at block `25066498`,
while mempool signals start at `25073547`. The backtest now emits
`BlockCompleted` monitor ticks for the whole requested window, but the early
part of the 70K window has no pool observations to create entries.

The interrupted exploratory run
`hist-snipe-all-maxhold200-monitor-70k-20260513-212818Z` was marked `aborted`
after it exposed block-by-block sell retry spam. The valid runs below use the
bounded monitor behavior.

## Code Verification

Commands:

```bash
cargo test -p eth_strategies -p eth_alpha_engine -p eth_alpha_backtest
cargo build --release -p eth_alpha_backtest
```

Key behavior covered by tests:

- `SnipeAllStrategy::on_position_monitor` exits a `BuyConfirmed` position after
  `max_hold_blocks` even without a pool update.
- The monitor does not retry a `SellFailed` position every block.
- LP-approval risk events submit an immediate sell for a matching open position.
- Existing strategy and entry-rule tests pass with the current WETH denom route
  requirement.

## Completed Runs

| Label | Run ID | Config |
| --- | --- | --- |
| maxhold200 | `hist-snipe-all-maxhold200-monitor-70k-20260513-213218Z` | `--max-hold-blocks 200` |
| maxhold50 | `hist-snipe-all-maxhold50-monitor-70k-20260513-213420Z` | `--max-hold-blocks 50` |
| tp3_sl70_maxhold200 | `hist-snipe-all-tp3-sl70-maxhold200-70k-20260513-213420Z` | `--take-profit-ratio 3.0 --stop-loss-ratio 0.7 --max-hold-blocks 200` |
| lpapproval_only | `hist-snipe-all-lpapproval-immediate-70k-20260513-213420Z` | `--include-mempool-signals --exit-lp-approval` |
| riskbundle_maxhold200 | `hist-snipe-all-riskbundle-maxhold200-70k-20260513-213420Z` | `--include-mempool-signals --exit-liquidity-removal --exit-lp-approval --exit-tax --exit-scam --max-hold-blocks 200` |

## Summary

| Label | Positions | Open | Buy failed | Sell failed | Buy orders | Sell orders | Reports | Confirmed | Failed | PnL ETH | PnL ex top 1 | PnL ex top 10 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| maxhold200 | 508 | 70 | 11 | 69 | 508 | 514 | 1022 | 924 | 98 | 13.319522 | 9.566667 | -1.014626 |
| maxhold50 | 508 | 56 | 11 | 56 | 508 | 532 | 1040 | 938 | 102 | 12.515832 | 10.426678 | 5.553139 |
| tp3_sl70_maxhold200 | 508 | 14 | 11 | 14 | 508 | 568 | 1076 | 980 | 96 | 3.972873 | 3.880844 | 3.359605 |
| lpapproval_only | 508 | 491 | 11 | 0 | 508 | 6 | 514 | 503 | 11 | 3.096810 | -1.879008 | -4.152874 |
| riskbundle_maxhold200 | 508 | 70 | 11 | 69 | 508 | 514 | 1022 | 924 | 98 | 13.584192 | 9.831337 | -0.749956 |

## Top/Worst 10 Verification

The `eth_alpha_lab strategy --limit 10` report was run for all five run IDs.

Observed mechanics:

- `maxhold200`: top 10 and worst 10 all snapshot at entry block + `201`.
  The monitor is firing on the first block strictly greater than
  `entry_block + 200`.
- `maxhold50`: top 10 snapshot at entry block + `51`; worst 10 mostly do the
  same, with a few later failed-exit retry snapshots. The shorter hold reduces
  concentration: PnL excluding top 10 stays positive at `5.553139 ETH`.
- `tp3_sl70_maxhold200`: top 10 exit after 1 to 11 blocks, confirming the
  take-profit/stop-loss pool-update path is active. Worst 10 are still
  `-0.01 ETH` losses, mostly max-hold exits or failed exits.
- `lpapproval_only`: only 6 sells occurred and all 6 had a prior LP-approval
  signal. This policy is too sparse by itself: it left 491 positions open and
  total PnL turns negative after removing the top winner.
- `riskbundle_maxhold200`: 12 confirmed sell blocks moved earlier than the
  maxhold-only run: 5 from LP approval and 7 from liquidity removal. Net PnL
  improved by `0.264670239 ETH` versus `maxhold200`.

## Current Read

`maxhold50` is the cleanest candidate from this batch: it keeps strong total
PnL, is much less dependent on the largest winners than max-hold 200, and has
fewer open failed exits.

`tp3_sl70_maxhold200` is the most controlled lifecycle policy: only 14 positions
remain open, top-winner concentration is low, and it exits winners quickly, but
it gives up the large tail gains that drive max-hold PnL.

LP approval should be an add-on risk exit, not a standalone exit policy.
