# Iteration 01

Status: completed baseline OOT training.

## Dataset

- Experiment: `backdoor_horizon_oot_v1`
- Train rows: `101403`
- Validation rows: `28130`
- Test rows: `25989`
- Dropped overlapping test pools: `0`

## Best Models

| Target | Model | Test PR AUC | Test ROC AUC | Precision@1% | Recall@1% |
| --- | --- | ---: | ---: | ---: | ---: |
| `scam_within_10_chain_block_delta` | `hist_gradient_boosting` | 0.1466 | 0.7517 | 31.15% | 8.64% |
| `scam_within_1_chain_block_delta` | `random_forest` | 0.2518 | 0.8855 | 16.15% | 39.25% |
| `scam_within_2_chain_block_delta` | `random_forest` | 0.3114 | 0.8507 | 29.23% | 39.79% |
| `scam_within_3_chain_block_delta` | `random_forest` | 0.2772 | 0.8077 | 32.69% | 31.84% |
| `scam_within_4_chain_block_delta` | `hist_gradient_boosting` | 0.2418 | 0.8035 | 33.46% | 25.51% |

## ULIQ Holdout

- `scam_within_10_chain_block_delta`: max p=0.2505 at block `25144048`; y=0.
- `scam_within_1_chain_block_delta`: max p=0.0888 at block `25144059`; y=0.
- `scam_within_2_chain_block_delta`: max p=0.3515 at block `25144048`; y=0.
- `scam_within_3_chain_block_delta`: max p=0.5343 at block `25144047`; y=0.
- `scam_within_4_chain_block_delta`: max p=0.0216 at block `25144047`; y=0.

## Review Notes

- Performance: the short horizons are meaningfully above base rate. The
  1-block target has a 0.41% positive rate, but the top 1% scored rows have
  16.15% precision and catch 39.25% of positives. The 2-block target is the
  strongest first-pass model by PR AUC at 0.3114.
- Trading usefulness: useful as a triage/ranking layer, not yet as a direct
  trading gate. Top 1% precision is high relative to base rate, but false
  positives remain common and the ULIQ-style fast backdoor is not caught.
- ULIQ holdout: the token-control backdoor signal fires at block `25144057`,
  six chain blocks before the scam at `25144063`, but the best 4-block model
  assigns only `0.0114` to the positive ULIQ row at block `25144059`. The
  signal exists in the row, so iteration 1 failed to learn enough weight for
  this rare mechanism.
- True positives: top-ranked rows should be reviewed for whether LP approval
  and reserve-movement features dominate, because the current models appear to
  prefer common direct-LP-removal patterns.
- False positives: review the top 1% false positives for possible missed scam
  labels before treating them as model mistakes.
- False negatives: prioritize rows with `pair_balance_backdoor_signal_seen_as_of`
  or `control_transfer_from_*_seen_as_of`; these are the rare mechanism misses
  we care about for fast scam trading protection.
- Infrastructure finding: the 50K train export must run in release mode, and
  compact Risk Atlas observation JSON is required for large imports. A repeated
  zero-output buy simulation on one pool dominated the slow tail; iteration 2
  should add a safe repeated-failed-simulation suppression path or a V2-only
  modeling export path.
- Next feature change: add a separate backdoor-signal model/rule branch with
  features for first signal block, blocks since first signal, signal counts,
  and explicit interactions between control `transferFrom` and pair-balance
  movement.
