# Direct LP-Removal Gate

Status: priority track for iteration 02.

## Why This First

Direct LP removal is the dominant labeled scam mechanism in the OOT test split.

| Mechanism | Test scam pools | Share |
| --- | ---: | ---: |
| `direct_lp_liquidity_removal` | 526 | 74.4% |
| `pair_balance_backdoor_drain` | 168 | 23.8% |
| `reserve_dump_drain` | 13 | 1.8% |

It is also the mechanism with the cleanest pre-scam signal. In test, the
direct-LP gate appears before scam for `524/526` direct-LP removal pools
(`99.6%` coverage). The gate uses the same strict 30% threshold as Alpha11:
`lp_removable_pct_as_of > 30`, `creator_lp_removable_pct_as_of > 30`, or
`creator_lp_router_removable_pct_as_of > 30`.

Gate metric artifact:
`artifacts/backdoor_horizon_oot_v1/gate/direct_lp_removal/`.

## Signal Shape

For direct LP removals in the test split, the first direct-LP gate signal
appears:

| Lead window | Pools | Cumulative coverage |
| ---: | ---: | ---: |
| `<= 1` block | 32 | 6.1% |
| `<= 2` blocks | 48 | 9.2% |
| `<= 3` blocks | 54 | 10.3% |
| `<= 4` blocks | 55 | 10.5% |
| `<= 10` blocks | 61 | 11.6% |
| `<= 20` blocks | 82 | 15.6% |
| `<= 50` blocks | 110 | 21.0% |
| `<= 100` blocks | 448 | 85.5% |
| `<= 200` blocks | 477 | 91.0% |
| `<= 500` blocks | 499 | 95.2% |

Lead-time summary:

- Test median lead: `75` blocks
- Test p25/p75: `64.75 / 85` blocks
- Train median lead: `54` blocks
- Validation median lead: `50` blocks

Interpretation: this is not mainly a 1-4 block signal. It is a high-confidence
danger state that often appears tens of blocks before removal.

## Current Model Behavior

The generic scam models are already mostly detecting direct LP-removal risk in
their top-ranked rows.

| Target | Best model | Direct-LP positives | Top 1% direct precision | Top 1% direct recall |
| --- | --- | ---: | ---: | ---: |
| `scam_within_1_chain_block_delta` | random forest | 94 | 16.15% | 44.68% |
| `scam_within_2_chain_block_delta` | random forest | 155 | 29.23% | 49.03% |
| `scam_within_3_chain_block_delta` | random forest | 201 | 32.69% | 42.29% |
| `scam_within_4_chain_block_delta` | hist gradient boosting | 243 | 33.46% | 35.80% |
| `scam_within_10_chain_block_delta` | hist gradient boosting | 468 | 30.77% | 17.09% |

The simple direct-LP gate has very high recall but poor
short-horizon precision because it fires early and remains active for many rows.

For example in test:

| Horizon | Rule precision | Rule recall |
| ---: | ---: | ---: |
| 1 block | 0.83% | 100.00% |
| 2 blocks | 1.35% | 98.06% |
| 4 blocks | 2.04% | 94.65% |
| 10 blocks | 3.71% | 89.32% |

Pool-level gate readout on the test split:

- Direct-LP scam pools: `526`
- Direct-LP scam pools with pre-scam warning: `524`
- Direct-LP scam pools without pre-scam warning: `2`
- Direct-LP warning coverage: `99.62%`
- Danger-state pools: `537`
- Non-direct danger-state pools: `13`

The model improves ranking precision by combining the LP-removable state with
age, activity, price, and liquidity context.

## Signal Audit

Question: does the model have a usable signal, or are we asking it to predict
without evidence?

Answer:

- For direct LP-removal existence, yes. The pre-scam LP-removable signal exists
  for nearly every direct-LP scam pool.
- For exact 1-4 block timing, only partially. Once the LP-removable state is
  active, the short-horizon positive base rate is tiny: `0.83%` for 1 block and
  `2.04%` for 4 blocks. The model can rank this above base rate, but it cannot
  make the raw danger state precise.

Within-gate test metrics:

| Horizon | Gated rows | Positives | Base rate | PR AUC | Precision@1% | Recall@1% |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `within_1_chain_block_delta` | 11,279 | 94 | 0.83% | 0.3452 | 36.28% | 43.62% |
| `within_2_chain_block_delta` | 11,279 | 152 | 1.35% | 0.4089 | 53.10% | 39.47% |
| `within_3_chain_block_delta` | 11,278 | 194 | 1.72% | 0.3893 | 61.06% | 35.57% |
| `within_4_chain_block_delta` | 11,278 | 230 | 2.04% | 0.3317 | 55.75% | 27.39% |

The top countdown features inside the gate are:

- `last_lp_approval_to_as_of_chain_block_delta`
- `trading_enabled_to_last_lp_approval_chain_block_delta`
- `buy_volume_denom`
- `lp_approval_count_in_block`
- `token_transfer_to_total_supply_ratio`
- `token_transfer_to_pool_token_reserve_ratio`

Interpretation: we do have a countdown signal, but it is mostly about LP
approval timing and recent activity. We do not yet have the stronger execution
trigger signal: the liquidity-removal transaction entering mempool, creator
bundle behavior, or a pending router/remove-liquidity call. Without that, the
model can rank urgency but not confidently predict the exact next block.

## Direct-LP-Specific Heads

Trained artifact:
`artifacts/backdoor_horizon_oot_v1/mechanism_models/direct_lp_liquidity_removal_gate/`.

These targets predict only `direct_lp_liquidity_removal` within the horizon,
not any scam type.

| Target | Test positives | PR AUC | ROC AUC | Precision@1% | Recall@1% | Precision@5% | Recall@5% |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `within_1_chain_block_delta` | 94 | 0.3452 | 0.9513 | 18.46% | 51.06% | 5.16% | 71.28% |
| `within_2_chain_block_delta` | 155 | 0.4012 | 0.9257 | 28.08% | 47.10% | 7.70% | 64.52% |
| `within_3_chain_block_delta` | 201 | 0.3763 | 0.9036 | 29.62% | 38.31% | 8.85% | 57.21% |
| `within_4_chain_block_delta` | 243 | 0.3152 | 0.8857 | 30.77% | 32.92% | 9.31% | 49.79% |
| `within_10_chain_block_delta` | 468 | 0.1984 | 0.8227 | 32.69% | 18.16% | 11.09% | 30.77% |
| `within_50_chain_block_delta` | 4,815 | 0.7810 | 0.9249 | 94.98% | 5.11% | 93.75% | 25.21% |
| `within_100_chain_block_delta` | 10,204 | 0.9488 | 0.9745 | 98.45% | 2.49% | 97.37% | 12.32% |

Readout:

- The direct-LP-specific 1-block head improves over the generic 1-block scam
  head on PR AUC (`0.3452` vs `0.2518`) and top-1% recall (`51.06%` vs
  `44.68%`).
- The short-horizon heads are useful for ranking urgent exits, but they should
  not be the first gate. Even at the top 1%, precision is roughly `18-33%`
  because many LP-removable pools sit in the danger state before the actual
  removal block.
- The 50/100-block heads model the danger state very well. This matches the
  lead-time evidence: the common signal is "LP can be removed soon enough to
  matter", not "LP will be removed in exactly the next few blocks".

## Gate Design

Use a two-stage gate instead of a single short-horizon classifier.

### Stage 1: Direct-LP Danger State

Purpose: decide whether the pool is structurally unsafe because LP can be
removed.

Primary rule:

```text
direct_lp_danger_state =
  lp_removable_pct_as_of > 30
  OR creator_lp_removable_pct_as_of > 30
  OR creator_lp_router_removable_pct_as_of > 30
```

Expected behavior:

- High recall.
- Can fire many blocks before removal.
- Suitable for "do not enter" or "exit unless already protected" risk gating.

### Stage 2: Removal Urgency

Purpose: rank direct-LP danger-state rows by expected near-term removal risk.

Use model score and timing features:

- `scam_within_1_chain_block_delta` score
- `scam_within_2_chain_block_delta` score
- `scam_within_4_chain_block_delta` score
- `last_lp_approval_to_as_of_chain_block_delta`
- `first_lp_approval_to_as_of_chain_block_delta`
- `active_observation_index`
- `total_liquidity_denom`
- `price_to_initial_ratio`
- `buy_volume_denom`
- `token_transfer_to_pool_token_reserve_ratio`

Candidate gate output:

```text
direct_lp_gate_state:
  clear
  lp_removable_watch
  lp_removable_exit
  imminent_direct_lp_removal
```

Initial policy proposal:

| State | Condition | Action |
| --- | --- | --- |
| `clear` | no LP-removable evidence | no direct-LP gate |
| `lp_removable_watch` | LP removable, model scores low | show risk; avoid new long-hold entries |
| `lp_removable_exit` | LP removable and top-ranked model score | exit/avoid |
| `imminent_direct_lp_removal` | LP removable and 1/2-block score in top slice | urgent exit / no entry |

### LP Approval Exit Policy

Risk Atlas separates two LP approval timing regimes:

- Immediate approval: last LP approval appears `<= 2` chain blocks from
  trading enabled. In the current direct-LP test run this is `333/529`
  removals (`62.9%`). Median removal age is `78` blocks, and only `10/333`
  (`3.0%`) removed inside a 15-block hold window.
- Late fresh approval: last LP approval appears after the launch window and
  within `<= 2` blocks before removal. This is `72/529` removals (`13.6%`).

Policy: immediate launch-window LP approval is a no-entry/max-hold caution.
Late fresh LP approval is an urgent exit trigger.

We should tune the score thresholds from ranked validation/test slices rather
than from raw probabilities, because the current classifiers are not calibrated.

## Next Work

1. Done: build direct-LP-specific targets:
   `direct_lp_liquidity_removal_within_{1,2,3,4,10,50,100}_chain_block_delta`.
2. Done: train direct-LP-specific heads and compare against the generic scam
   heads.
3. Done: add first-pass gate-oriented metrics:
   rule precision/recall, first-warning lead time, and pool-level capture.
4. Review the two direct-LP false negatives where the direct-LP gate did not
   appear pre-scam.
5. Add false-positive duration and threshold policy for `lp_removable_watch`,
   `lp_removable_exit`, and `imminent_direct_lp_removal`.
