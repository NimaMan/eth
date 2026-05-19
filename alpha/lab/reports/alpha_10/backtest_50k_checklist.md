# 50K Backtest And Validation Checklist

This checklist is for the later review of the same policy family on a 50K block
range. Do not promote a policy from the 20K evidence alone.

## Range Contract

Completed 50K review:

```bash
FROM_BLOCK=25067915
TO_BLOCK=25117914
REPLAY_RUN_ID=risk-atlas-alpha10-50k-25067915-25117914-20260518
RESULT_SET_ID=historical-25067915-25117914
RUN_ID=alpha10-risk-atlas-50k-25067915-25117914-20260518
```

The originally planned anchored range `25090165` to `25140164` failed because
the local source could not load block `25117918`. Keep that failure visible if
the anchored range is retried later.

The 50K review must record:

```bash
FROM_BLOCK=<50K start block>
TO_BLOCK=<50K end block>
REPLAY_RUN_ID=<Risk Atlas run id for exactly this 50K range>
RESULT_SET_ID=historical-${FROM_BLOCK}-${TO_BLOCK}
```

If the 50K range is anchored at the current 20K start, use:

```bash
FROM_BLOCK=25090165
TO_BLOCK=25140164
```

That is exactly 50,000 inclusive blocks.

Before running backtests, verify:

```bash
curl -s "http://127.0.0.1:40019/eth/tokens/api/analytics/risk-atlas/runs/${REPLAY_RUN_ID}" \
  | jq '.run | {run_id,start_block,end_block,block_count,status}'
```

Expected:

```text
start_block == FROM_BLOCK
end_block == TO_BLOCK
block_count == 50000
status == completed
```

## Baseline Commands

Run all commands from:

```bash
cd /home/nima/code/crypto/blockchains/eth
```

Set shared variables:

```bash
export FROM_BLOCK=<50K start block>
export TO_BLOCK=<50K end block>
export REPLAY_RUN_ID=<risk-atlas-50k-run-id>
export RESULT_SET_ID=historical-${FROM_BLOCK}-${TO_BLOCK}
export RUN_TAG=50k-$(date -u +%Y%m%d)
```

Primary alpha_10 suite:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id alpha10-risk-atlas-${RUN_TAG} \
  --replay-run-id "${REPLAY_RUN_ID}" \
  --strategy-suite alpha-10-risk-atlas \
  --from-block "${FROM_BLOCK}" \
  --to-block "${TO_BLOCK}" \
  --execution-delay-blocks 1
```

The command above is the one that produces the ten `alpha10-*` strategies in
one persisted run. The commands below remain useful for older comparison suites
and broader sweeps.

Baseline buy-confirm comparison:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id risk-atlas-alpha10-buy-confirm-${RUN_TAG} \
  --replay-run-id "${REPLAY_RUN_ID}" \
  --strategy-suite risk-atlas-lp-buy-confirm-block-comparison \
  --from-block "${FROM_BLOCK}" \
  --to-block "${TO_BLOCK}" \
  --execution-delay-blocks 1
```

V2-only buy-confirm comparison:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id risk-atlas-alpha10-buy-confirm-v2-${RUN_TAG} \
  --replay-run-id "${REPLAY_RUN_ID}" \
  --strategy-suite risk-atlas-lp-buy-confirm-block-comparison-uniswap-v2-only \
  --from-block "${FROM_BLOCK}" \
  --to-block "${TO_BLOCK}" \
  --execution-delay-blocks 1
```

Hold-window sweep:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id risk-atlas-alpha10-hold-sweep-${RUN_TAG} \
  --replay-run-id "${REPLAY_RUN_ID}" \
  --strategy-suite risk-atlas-edge-v3 \
  --from-block "${FROM_BLOCK}" \
  --to-block "${TO_BLOCK}" \
  --execution-delay-blocks 1
```

Take-profit sweep:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id risk-atlas-alpha10-tp-sweep-${RUN_TAG} \
  --replay-run-id "${REPLAY_RUN_ID}" \
  --strategy-suite risk-atlas-edge-v4 \
  --from-block "${FROM_BLOCK}" \
  --to-block "${TO_BLOCK}" \
  --execution-delay-blocks 1
```

Stop-loss plus take-profit sweep:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id risk-atlas-alpha10-sl-tp-sweep-${RUN_TAG} \
  --replay-run-id "${REPLAY_RUN_ID}" \
  --strategy-suite risk-atlas-edge-v5 \
  --from-block "${FROM_BLOCK}" \
  --to-block "${TO_BLOCK}" \
  --execution-delay-blocks 1
```

Retry sensitivity for failed exits:

```bash
cargo run -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id risk-atlas-alpha10-retry-${RUN_TAG} \
  --replay-run-id "${REPLAY_RUN_ID}" \
  --strategy-suite risk-atlas-lp-buy-confirm-block-comparison \
  --from-block "${FROM_BLOCK}" \
  --to-block "${TO_BLOCK}" \
  --execution-delay-blocks 1 \
  --exit-retry-interval-blocks 1 \
  --max-exit-retries 3
```

## Iteration-To-Run Matrix

| Iteration | Primary 50K evidence | Current runnable command |
| --- | --- | --- |
| 01 baseline hold15 buy-confirm | Baseline PnL, same-confirmation-block LP approval PnL, loser scan | `alpha-10-risk-atlas` |
| 02 V2-only universe | Protocol-restricted PnL and loss reduction | `alpha-10-risk-atlas` |
| 03 hold-window sweep | Hold15/20/30/40 comparison | `alpha-10-risk-atlas` |
| 04 take-profit sweep | TP3/5 over hold30 | `alpha-10-risk-atlas` |
| 05 stop-loss plus take-profit | SL70/85 with TP3 over hold30 | `alpha-10-risk-atlas` |
| 06 clean-lead LP exit | Review cut by LP approval lead and removal ordering | `losing-trades`, trade detail API, validation samples |
| 07 buy-confirm quarantine | Review cut excluding LP approval in buy-confirmation block | `losing-trades`, trade detail API, baseline result export |
| 08 retry and fee guard | Failed exit sensitivity | `alpha-10-risk-atlas` |
| 09 launch momentum gate | Winner/loser feature report before coding thresholds | trade detail snapshots, Risk Atlas observation export |
| 10 non-LP scam guard | No-risk-event loss size and mechanism split | `losing-trades`, Risk Atlas scam mechanism reports |

Iterations 06, 07, 09, and 10 are intentionally marked as review cuts or future
coded variants. They should not be silently treated as already implemented
strategy suites.

The 20K run proves iterations 01 through 05 and 08 are implemented in the
primary suite. Iterations 06, 07, 09, and 10 still require the review cuts above
before they should become new coded policies.

## Review URLs

After a run finishes:

```text
http://127.0.0.1:40019/eth/trade/backtests/historical/${FROM_BLOCK}-${TO_BLOCK}/
http://127.0.0.1:40019/eth/trade/backtests/historical/${FROM_BLOCK}-${TO_BLOCK}/strategies/<strategy-name>/
http://127.0.0.1:40019/eth/trade/backtests/historical/${FROM_BLOCK}-${TO_BLOCK}/strategies/<strategy-name>/validation/
```

Risk Atlas source page:

```text
http://127.0.0.1:40019/eth/tokens/analytics/risk-atlas/direct-lp-warning/?run_id=${REPLAY_RUN_ID}
```

## Required Lab Checks

For each candidate strategy:

```bash
cargo run -p eth_alpha_lab --bin eth_alpha_lab -- \
  losing-trades \
  --result-set "${RESULT_SET_ID}" \
  --strategy "<strategy-name>" \
  --limit 50
```

```bash
cargo run -p eth_alpha_lab --bin eth_alpha_lab -- \
  backtest-validation \
  --result-set "${RESULT_SET_ID}" \
  --strategy "<strategy-name>"
```

If `eth_alpha_lab strategy` is used, confirm it runs successfully first. In the
current workspace it failed on an unrelated protocol-bucket SQL grouping issue,
so it is not a required gate for this package until that command is fixed.

## Acceptance Table

Record one row per candidate:

| Strategy | Trades | Closed | Open | Failed | Realized ETH | Unrealized ETH | Total ETH | ROI % | Losers | Loss ETH | Top-5 winner share | Verdict |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| baseline hold15 buy-confirm | | | | | | | | | | | | |
| best hold sweep | | | | | | | | | | | | |
| best TP sweep | | | | | | | | | | | | |
| best SL/TP sweep | | | | | | | | | | | | |
| retry sensitivity | | | | | | | | | | | | |

## Do Not Promote If

- The result only improves total PnL by carrying more open unrealized exposure.
- The strategy gets most PnL from buy-confirmation-block LP approval ordering.
- The top one to five winners explain the whole edge.
- Worst-loser review shows the same preventable loss bucket grew.
- Validation reports timing, accounting, or replay-readiness failures.
