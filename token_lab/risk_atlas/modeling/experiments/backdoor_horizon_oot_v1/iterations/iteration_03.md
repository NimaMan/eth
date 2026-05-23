# Iteration 03

Status: completed - direct LP-removal gate focus.

## Dataset

- Experiment: `backdoor_horizon_oot_v1`
- Train rows: `101403`
- Validation rows: `28130`
- Test rows: `25989`
- Dropped overlapping test pools: `0`

## Model Changes

The iteration moves from generic scam probability to the dominant mechanism:
`direct_lp_liquidity_removal`.

Trained mechanism-specific hist-gradient-boosting heads for:

- `direct_lp_liquidity_removal_within_1_chain_block_delta`
- `direct_lp_liquidity_removal_within_2_chain_block_delta`
- `direct_lp_liquidity_removal_within_3_chain_block_delta`
- `direct_lp_liquidity_removal_within_4_chain_block_delta`
- `direct_lp_liquidity_removal_within_10_chain_block_delta`
- `direct_lp_liquidity_removal_within_50_chain_block_delta`
- `direct_lp_liquidity_removal_within_100_chain_block_delta`

Artifacts:
`artifacts/backdoor_horizon_oot_v1/mechanism_models/direct_lp_liquidity_removal_gate/`.

## Results

| Target | Test positives | PR AUC | ROC AUC | Precision@1% | Recall@1% | Precision@5% | Recall@5% |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `within_1_chain_block_delta` | 94 | 0.3452 | 0.9513 | 18.46% | 51.06% | 5.16% | 71.28% |
| `within_2_chain_block_delta` | 155 | 0.4012 | 0.9257 | 28.08% | 47.10% | 7.70% | 64.52% |
| `within_3_chain_block_delta` | 201 | 0.3763 | 0.9036 | 29.62% | 38.31% | 8.85% | 57.21% |
| `within_4_chain_block_delta` | 243 | 0.3152 | 0.8857 | 30.77% | 32.92% | 9.31% | 49.79% |
| `within_10_chain_block_delta` | 468 | 0.1984 | 0.8227 | 32.69% | 18.16% | 11.09% | 30.77% |
| `within_50_chain_block_delta` | 4,815 | 0.7810 | 0.9249 | 94.98% | 5.11% | 93.75% | 25.21% |
| `within_100_chain_block_delta` | 10,204 | 0.9488 | 0.9745 | 98.45% | 2.49% | 97.37% | 12.32% |

## Review Notes

- Direct LP removal is `74.4%` of labeled test scams, so this should be the
  first production trading gate.
- The direct-LP gate appears before scam for `524/526` direct-LP test pools, so
  it is the right structural danger-state signal. The gate is aligned with
  Alpha11's strict 30% threshold: LP-removable exposure must be `> 30%`.
- The first LP-removable signal usually appears far earlier than the last few
  blocks. Test median lead is `75` chain blocks, and `85.5%` of direct-LP scam
  pools show the signal within `100` blocks before removal.
- Short-horizon heads are useful for ranking urgency after the gate fires. They
  are not a replacement for the gate, because many true danger-state rows are
  not removed immediately.
- First-pass gate evaluation is now in
  `artifacts/backdoor_horizon_oot_v1/gate/direct_lp_removal/`: test warning
  coverage is `99.62%`, median first-warning lead is `75` chain blocks, and
  only `13` non-direct test pools enter the danger state.
- Inside the danger state, the current feature set has some countdown signal:
  1-block base rate is `0.83%`, while top-1% model precision is `36.28%`; 4-block
  base rate is `2.04%`, while top-1% model precision is `55.75%`.
- The signal comes mostly from LP approval timing, approval-in-block, buy volume,
  and transfer/reserve ratios. We do not yet have the stronger execution signal:
  a pending remove-liquidity tx or bundle/mempool evidence.
- The next direct-LP work is false-positive duration and threshold policy for
  watch, exit, and imminent-removal states.
