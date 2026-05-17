# Risk Atlas Strategy Backtest, 20K Blocks

Source Risk Atlas run: `risk-atlas-run-2`
Block range: `25,090,165` to `25,110,164`
Generated: `2026-05-16`

## Implementation

The historical backtest runner now supports Risk Atlas replay runs by passing a Risk Atlas run id as `--replay-run-id`.

Example:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id risk-atlas-edge-v2-20k-20260516-iter2 \
  --replay-run-id risk-atlas-run-2 \
  --strategy-suite risk-atlas-edge-v2 \
  --from-block 25090165 \
  --to-block 25110164 \
  --execution-delay-blocks 1
```

Replay source:

- `risk_atlas_observations`
- eligible pools only
- `UNISWAP-V2` only for now, because the Risk Atlas observation export does not yet persist enough V3/V4 routing metadata for chain-sim execution
- mined LP approval rows become `LpApproval` risk events
- direct LP removal rows become `LiquidityRemoval` risk events
- pool observation rows become `PoolUpdated` market events

The `snipe-all` strategy now has two extra controls:

- `allowed_protocols`
- `block_entry_on_lp_approval`

## Iteration 1

Run id: `risk-atlas-edge-v1-20k-20260516-iter1`

Suite:

| Strategy | Core idea |
| --- | --- |
| `snipe-all-risk-atlas-lp-immediate-exit-v1` | exit on mined LP approval, no entry gate or hold limit |
| `snipe-all-risk-atlas-lp-launch-gate-v1` | LP approval entry gate plus LP exit |
| `snipe-all-risk-atlas-active-horizon-hold10-v1` | LP gate plus LP exit plus hold10 |
| `snipe-all-risk-atlas-v2-backdoor-fast-hold5-v1` | V2-only fast hold5 guard |
| `snipe-all-risk-atlas-protocol-guard-hold50-v1` | V2 protocol guard with longer hold50 |

Run summary:

- events processed: `59,646`
- reports generated: `12,326`
- confirmed reports: `6,134`
- failed reports: `29`
- positions: `3,320`
- open positions: `506`

Results:

| Strategy | Positions | Closed | Open | Failed | Realized ETH | Unrealized ETH | Total PnL ETH | Avg ROI |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| active-horizon-hold10-v1 | 554 | 550 | 3 | 1 | 1.239638 | -0.413705 | 0.825933 | 0.2213 |
| v2-backdoor-fast-hold5-v1 | 554 | 551 | 3 | 0 | 1.202134 | -0.643705 | 0.558429 | 0.2166 |
| lp-launch-gate-v1 | 554 | 538 | 15 | 1 | -0.376189 | -1.551049 | -1.927239 | -0.0715 |
| protocol-guard-hold50-v1 | 554 | 544 | 9 | 1 | -0.923843 | -1.532114 | -2.455957 | -0.1713 |
| lp-immediate-exit-v1 | 1,104 | 631 | 460 | 13 | 1.891454 | -5.573117 | -3.681664 | -0.1862 |

Read:

- The plain LP immediate exit variant buys too many pools and leaves too much open exposure.
- The LP launch gate cuts entries in half and removes much of the open exposure, but without a hold limit it still underperforms.
- The best first-pass signal was LP gate/exit plus an active hold limit.

## Iteration 2

Run id: `risk-atlas-edge-v2-20k-20260516-iter2`

Suite:

- all strategies are V2-only
- all strategies use LP approval entry gate
- all strategies exit on LP approval
- all strategies exit on direct LP removal
- sweep max active-hold blocks: `5`, `8`, `10`, `12`, `15`

Run summary:

- events processed: `59,646`
- reports generated: `11,060`
- confirmed reports: `5,516`
- failed reports: `14`
- positions: `2,770`
- open positions: `24`

Results:

| Strategy | Positions | Closed | Open | Failed | Realized ETH | Unrealized ETH | Total PnL ETH | Avg ROI |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| lp-gate-hold15-v2 | 554 | 547 | 6 | 1 | 2.070470 | -0.473726 | 1.596745 | 0.3772 |
| lp-gate-hold12-v2 | 554 | 548 | 5 | 1 | 1.534274 | -0.447292 | 1.086982 | 0.2738 |
| lp-gate-hold10-v2 | 554 | 550 | 3 | 1 | 1.239638 | -0.413705 | 0.825933 | 0.2213 |
| lp-gate-hold8-v2 | 554 | 550 | 3 | 1 | 1.007112 | -0.383705 | 0.623407 | 0.1794 |
| lp-gate-hold5-v2 | 554 | 551 | 3 | 0 | 0.616514 | -0.283705 | 0.332810 | 0.1107 |

Exit reasons:

| Strategy | LP approval exits | Liquidity removal exits | Max-hold exits |
| --- | ---: | ---: | ---: |
| hold5 | 354 | 21 | 178 |
| hold8 | 362 | 32 | 159 |
| hold10 | 365 | 35 | 153 |
| hold12 | 368 | 39 | 144 |
| hold15 | 371 | 42 | 137 |

Read:

- Longer hold improved this first sweep, with hold15 best.
- Hold15 still kept open exposure low: `6` open positions out of `554`.
- Failed exits stayed low: `14` total failed reports across the full second suite.
- The next sweep should test `15`, `20`, `25`, `30`, and possibly add a profit/stop guard so we do not only optimize hold length.

## Current Best Candidate

`snipe-all-risk-atlas-lp-gate-hold15-v2`

Rules:

- eligible V2 pools only
- block new entries after mined-chain LP approval has appeared for the pool
- exit held positions on mined-chain LP approval
- exit held positions on direct LP removal
- force exit after `15` active pool-update observations

20K result:

- total PnL: `+1.596745 ETH`
- avg ROI: `0.3772`
- closed: `547 / 554`
- open: `6 / 554`
- failed: `1 / 554`

## PnL Attribution And Leakage Check

For the current best candidate, the PnL came mainly from the active-hold exit,
not from the LP-approval exit itself.

| Exit reason | Positions | Realized ETH | Unrealized ETH | Total PnL ETH | Avg ROI | Median ROI |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| max hold active blocks | 137 | 2.444517 | -0.010000 | 2.434517 | 1.7843 | 1.5808 |
| open/unmapped | 6 | -0.000276 | 0.026274 | 0.025998 | 0.4333 | -0.0153 |
| LP approval | 369 | 0.036421 | -0.080000 | -0.043579 | 0.0107 | 0.0167 |
| liquidity removal | 42 | -0.410192 | -0.410000 | -0.820192 | -0.9998 | -1.0000 |

The largest factor is therefore early launch momentum captured by holding up to
15 active pool observations. The LP-approval rule mostly acts as a risk gate and
exit cap; it is not the source of the positive PnL in this run.

Important replay timing check:

- `0 / 554` bought positions had an LP approval before the buy was submitted.
- `296 / 554` bought positions had an LP approval in the buy confirmation block.
- those same-confirmation-block positions contributed `+1.297321 ETH`;
- excluding them leaves `+0.299424 ETH`.

This is not direct information leakage in the strategy decision, because the buy
orders were submitted before the LP approval was visible in confirmed history.
It is still the main replication caveat: the simulator assumes our order is
confirmed one block later as the last transaction in that block. If an LP
approval is mined earlier in that confirmation block, the replay can still fill
our buy after that approval. This is consistent with the current chain-sim
assumption, but the edge is only live-replicable if we can actually control or
approximate that next-block ordering.

LP approval exit timing was mostly actionable for positions that reached it:

- `364 / 369` LP-approval exits submitted before the later direct LP removal;
- `346 / 369` confirmed before direct LP removal;
- `18 / 369` confirmed in the same block as direct LP removal;
- `1 / 369` confirmed after direct LP removal.

## Next Iteration

The next useful iteration is not more LP approval wiring. It is:

- persist V3/V4 route metadata into Risk Atlas observations so protocol-specific strategies can be backtested over the full surface;
- add generic LP approval percent as a first-class numeric feature, separate from router-approved percent;
- sweep hold windows above `15` active observations;
- add take-profit/stop-loss constraints around the hold15 winner;
- add a separate pair-balance backdoor near-future target, because LP approval does not cover that scam family.
