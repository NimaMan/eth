# 100 Scam Token Review Goal

## Objective

Review the first 100 scam or suspect scam token/pool cases as token-centric
labels. The output should tell us how the scam happened, whether the label is
correct, and how long the pool survived from trading enabled to the first
verified scam block.

Stop after 100 audited rows so the cohort can be manually reviewed before the
schema or detectors are scaled.

## Scope

In scope:

- token and pool lifecycle;
- chain-truth evidence for trades, reserve changes, transfers, LP actions, and
  sell failures;
- token-builder observations and pool snapshots;
- mechanism cluster assignment;
- time-to-scam from `trading_enabled_block` to `label_block`.

Out of scope for labels:

- strategy PnL;
- whether a strategy entered or exited;
- whether a strategy was early or late;
- backtest ranking.

Strategy data can be recorded as an evidence reference only when it helped find
the case.

## Batch Acceptance Criteria

The 100-token checkpoint is complete when:

- the label ledger contains 100 reviewed token/pool label rows;
- every row has `token`, `pool`, `label`, `mechanism`, `confidence`, and
  `label_block` or a clear `needs_chain_truth` status;
- every verified row has `trading_enabled_block` or a documented reason why it
  is unknown;
- every row has an evidence reference;
- every verified scam has a cluster from `clusters/README.md`;
- a cohort summary reports counts by mechanism, confidence, and time bucket.

## Review Workflow

1. Select candidate scam/suspect pools from existing investigations, token
   server outputs, reserve-drain detections, sell-failure detections, and graph
   anomalies.
2. For each token/pool, establish `trading_enabled_block` using the documented
   evidence priority.
3. Establish `label_block` as the first reliable blocked, drained, or unsafe
   block.
4. Classify `mechanism` from chain behavior.
5. Compute `blocks_from_trading_enabled_to_label` and approximate minutes.
6. Record confidence as `verified`, `probable`, `needs_chain_truth`, or
   `needs_mechanism_review`.
7. Add an evidence reference and short note.
8. Stop at 100 rows and review the cluster distribution before continuing.

## First Cohort Output

The first cohort should produce durable label rows, a Risk Atlas distribution
snapshot, and a small human-auditable summary report. Large raw extracts should
stay local unless they are small enough to audit directly.
