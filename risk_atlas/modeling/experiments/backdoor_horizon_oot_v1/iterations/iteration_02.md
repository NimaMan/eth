# Iteration 02

Status: pre-iteration investigation complete; training not started.

## Pre-Iteration Signal Review

Full report:
`experiments/backdoor_horizon_oot_v1/investigations/pre_iteration_02_signal_review.md`

Artifacts:
`artifacts/backdoor_horizon_oot_v1/investigations/pre_iteration_02/`

Reviewed test cases:

| Case | Category | Selected block | 4-block score | Scam block | Mechanism |
| --- | --- | ---: | ---: | ---: | --- |
| `uliq_holdout_backdoor_miss` | holdout false negative | `25144059` | 1.14% | `25144063` | `pair_balance_backdoor_drain` |
| `direct_lp_true_positive` | true positive | `25130547` | 96.33% | `25130549` | `direct_lp_liquidity_removal` |
| `backdoor_true_positive` | true positive with backdoor signal | `25127850` | 90.92% | `25127852` | `direct_lp_liquidity_removal` |
| `backdoor_false_negative` | false negative with backdoor signal | `25124408` | 0.43% | `25124411` | `pair_balance_backdoor_drain` |
| `near_horizon_false_positive` | high-score near miss | `25125020` | 80.12% | `25125031` | `direct_lp_liquidity_removal` |
| `unlabeled_backdoor_false_positive` | high-score unlabeled backdoor | `25130111` | 58.91% | - | - |

Main findings:

- The current model already understands the direct LP-removal family: when
  `lp_removable_pct_as_of >= 90` appears shortly before removal, scores are
  high.
- The ULIQ-style pair-balance backdoor family is not learned reliably. ULIQ had
  a backdoor signal 6 chain blocks before scam and scored only 1.14% on the
  positive 4-block row. Another pair-balance drain had the signal 15 chain
  blocks before scam and scored only 0.43%.
- The important token-control pattern is the joint arrival of
  `pair_balance_backdoor_signal_seen_as_of`,
  `control_transfer_from_holder_to_burn_seen_as_of`, and
  `control_transfer_from_without_transfer_log_seen_as_of`.
- Network jobs found creator+owner labelled control wallets for the two
  pair-balance drain misses. We should only use this once the feature can be
  made as-of safe.
- Mempool evidence is unavailable in this selected sample:
  `mempool_first_seen_ms` was empty for all selected evidence rows.

## High-Level Reset

Before doing more model tuning, the corpus-level stats show the priority order.

Test source scam mechanisms:

| Mechanism | Test scam pools | Share of test scam pools |
| --- | ---: | ---: |
| `direct_lp_liquidity_removal` | 526 | 74.4% |
| `pair_balance_backdoor_drain` | 168 | 23.8% |
| `reserve_dump_drain` | 13 | 1.8% |

Train source scam mechanisms:

| Mechanism | Train scam pools |
| --- | ---: |
| `direct_lp_liquidity_removal` | 1486 |
| `pair_balance_backdoor_drain` | 420 |
| `reserve_dump_drain` | 65 |
| `privileged_seller_reserve_drain` | 19 |

Pre-scam signal coverage:

| Split | Mechanism | Pools | LP removable >=90% | Backdoor signal seen | Holder burn seen |
| --- | --- | ---: | ---: | ---: | ---: |
| test | `direct_lp_liquidity_removal` | 526 | 99.6% | 0.4% | 0.0% |
| test | `pair_balance_backdoor_drain` | 168 | 0.0% | 22.6% | 22.6% |
| train | `direct_lp_liquidity_removal` | 1132 | 90.6% | 1.6% | 0.0% |
| train | `pair_balance_backdoor_drain` | 316 | 0.0% | 32.0% | 32.0% |

Lead times where the signal exists:

| Split | Mechanism | Signal | Pools | Median lead |
| --- | --- | --- | ---: | ---: |
| test | `direct_lp_liquidity_removal` | `lp_removable_pct_as_of >= 90` | 524 | 75 blocks |
| test | `pair_balance_backdoor_drain` | backdoor signal | 38 | 19 blocks |
| train | `direct_lp_liquidity_removal` | `lp_removable_pct_as_of >= 90` | 1026 | 54 blocks |
| train | `pair_balance_backdoor_drain` | backdoor signal | 101 | 46 blocks |

Priority read:

1. Direct LP removal is the first production/trading gate: it is the dominant
   scam mechanism and current features already cover it.
2. Pair-balance backdoor is the second priority: it is about one quarter of test
   scams, but current pre-scam backdoor signals only cover 22.6% of those pools.
   The immediate work is signal extraction and mechanics review, not more model
   tuning.
3. The 1/2/3-block backdoor target is too narrow for the current observations:
   ULIQ is visible at 4 blocks before drain, and the backdoor signal's median
   lead is much longer where it exists. We need a branch-state warning such as
   "backdoor path is active" plus horizon heads, not only 1/2/3-block labels.
4. Reserve-dump and privileged-seller mechanisms are smaller and should follow
   after direct LP and pair-balance backdoor.

## Dataset

- Experiment: `backdoor_horizon_oot_v1`
- Train rows: TBD
- Validation rows: TBD
- Test rows: TBD
- Dropped overlapping test pools: TBD

## Model Changes

- Add a dedicated backdoor-signal branch for token-control behavior.
- Add features for first signal block, blocks since first signal, signal counts,
  and interactions between control `transferFrom` and pair-balance movement.
- Keep direct LP-removal readiness as a separate branch from token-control
  backdoor behavior.
- Add an as-of-safe network/control feature set if we can derive creator/owner
  labels without looking past the observation block.

## Mechanism-Specific Gradient Boosting Probe

Script:
`scripts/train_mechanism_gradient_boosting.py`

Artifacts:

- Balanced:
  `artifacts/backdoor_horizon_oot_v1/mechanism_models/pair_balance_backdoor_drain/`
- Unweighted:
  `artifacts/backdoor_horizon_oot_v1/mechanism_models/pair_balance_backdoor_drain_unweighted/`

Target family:

```text
pair_balance_backdoor_drain_within_N_chain_block_delta
N = 1, 2, 3
```

This asks whether the pool gets a `pair_balance_backdoor_drain` event within
the next `N` raw chain blocks. Direct LP removals are negatives for this head.

Balanced histogram-gradient-boosting results:

| Target | Test positives | Positive rate | PR AUC | ROC AUC | Precision@1% | Recall@1% | Precision@5% | Recall@5% |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `pair_balance_backdoor_drain_within_1_chain_block_delta` | 8 | 0.03% | 0.0003 | 0.3950 | 0.00% | 0.00% | 0.00% | 0.00% |
| `pair_balance_backdoor_drain_within_2_chain_block_delta` | 25 | 0.10% | 0.0154 | 0.7884 | 2.31% | 24.00% | 1.23% | 64.00% |
| `pair_balance_backdoor_drain_within_3_chain_block_delta` | 55 | 0.21% | 0.0246 | 0.8404 | 2.31% | 10.91% | 3.08% | 72.73% |

Unweighted histogram-gradient-boosting results:

| Target | Test positives | Positive rate | PR AUC | ROC AUC | Precision@1% | Recall@1% | Precision@5% | Recall@5% |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `pair_balance_backdoor_drain_within_1_chain_block_delta` | 8 | 0.03% | 0.0009 | 0.6682 | 0.00% | 0.00% | 0.08% | 12.50% |
| `pair_balance_backdoor_drain_within_2_chain_block_delta` | 25 | 0.10% | 0.0011 | 0.4245 | 0.38% | 4.00% | 0.23% | 12.00% |
| `pair_balance_backdoor_drain_within_3_chain_block_delta` | 55 | 0.21% | 0.0174 | 0.8650 | 1.15% | 5.45% | 2.08% | 49.09% |

Current readout:

- The 1-block target is too sparse in this OOT split: only `8` positives.
- Balanced weighting gives better recall for 2/3-block detection, but the
  scores are not calibrated probabilities and top false positives remain high.
- ULIQ has no positive rows for the 1/2/3-block heads because the final
  observation is block `25144059` and the drain is block `25144063`, four
  chain blocks later.
- This confirms that type-specific heads are the right direction, but the next
  step is still feature work, not model tuning.

## Results

TBD; iteration 02 should train only after the feature changes above land.

## Review Notes

- True positives: TBD.
- False positives: TBD.
- False negatives: TBD.
- ULIQ holdout: current positive 4-block row scored 1.14% despite visible
  token-control flags; this is the primary iteration 02 regression target.
- Next feature change: first-signal timing, blocks-since-signal, signal-count
  features, and backdoor interaction features.
