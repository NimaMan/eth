# Scam Analytics

Workspace for turning confirmed scammed-pool cases into token/pool labels,
durations, and pre-scam features that can train or calibrate a scam-probability
model.

This track is token-centric. Scam labels and model rows are based on token and
pool behavior, not on how any external workflow interacted with the token.

## Goal

Build a pool-first dataset where each row has:

- a pool/token identifier;
- a scam label and mechanism;
- confidence for that label;
- the block where trading was first enabled or first observed;
- the block where the pool first became blocked, drained, or otherwise unsafe;
- duration features, especially blocks and approximate minutes from trading
  enabled to the first bad block;
- only features that would have been known before the label block.

The primary scam-evolution modeling target is:

```text
P(pool becomes scam / blocked / drained within the next N active token/pool observations | state up to observation O)
```

An active token/pool observation is a block where the pool or token materially
changes state or emits a useful signal, such as a swap, mint, burn, sync/reserve
update, LP approval or transfer, token transfer touching the pool, buy/sell
observation, liquidity update, or price-ratio update. Idle Ethereum blocks do
not count as scam-evolution time for this target.

Keep chain-block horizons as a separate execution/capital-at-risk target:

```text
P(pool becomes scam / blocked / drained within the next N chain blocks | state up to block B)
```

The active-observation target should reduce label noise for scam prediction
because token/pool state evolves only when something happens to that token or
pool. Two pools can look similar at launch and then diverge through activity;
counting idle chain blocks as equivalent evolution hides that difference.

Use within-horizon targets first because they match the live question more
directly: given the state now, is the pool likely to become unsafe soon? Exact
next-observation targets can be exported later as a timing diagnostic, but the
primary risk curve should be `P(label within next N active observations)`.

Start with a fine-grained near-future horizon set:

```text
1, 2, 3, 5, 10, 15, 20, 30, 50, 100, 250, 500
```

## Folder Structure

| Folder | Purpose |
| --- | --- |
| `labels/` | Human-auditable scammed-pool labels and duration schema. |
| `features/` | Feature contract for model-ready rows and leakage rules. |
| `clusters/` | Human-readable scam mechanism taxonomy and detector attributes. |
| `artifacts/` | Generated extracts, raw API responses, and local notebooks. |

## Label Semantics

Use token/pool-level labels, not position-level outcomes.

Important event blocks:

| Field | Meaning |
| --- | --- |
| `pool_created_block` | First known pool creation/liquidity block. |
| `trading_enabled_block` | First reliable block where public trading was enabled or observed. |
| `trading_enabled_source` | Evidence source for `trading_enabled_block`, such as chain-truth trade, token-builder trade, `can_buy=true`, or pool creation fallback. |
| `first_observed_trade_block` | First block where our data sees real pool trade activity. |
| `first_blocked_block` | First block where the public sell path is blocked, if applicable. |
| `first_drain_block` | First block where reserves collapse or quote liquidity leaves. |
| `label_block` | Earliest block where the pool can be labeled as scam/blocked/drained. |
| `blocks_from_trading_enabled_to_label` | `label_block - trading_enabled_block` when both are known. |
| `minutes_from_trading_enabled_to_label` | Approximate duration using 12 seconds per Ethereum block. |

`label_block` should be the minimum reliable block for the chosen label. For
reserve-drain cases it is usually `first_drain_block`. For honeypots it is
usually `first_blocked_block`.

Use this priority when choosing `trading_enabled_block`:

1. first successful non-owner buy or sell from chain truth;
2. first observed pool trade from token-builder data;
3. first pool update where runtime trading flags show public buy availability;
4. pool creation or initial liquidity block, only as an explicit fallback.

Do not fill timing fields from guesswork. Unknown fields should stay empty until
chain truth or token-builder evidence supports them.

## 100-Token Review Goal

The first review batch is exactly 100 scam or suspect scam token/pool cases.
Stop after the first 100 audited rows and review the output before expanding the
dataset.

Deliverables for that checkpoint:

- 100 token/pool rows with label, mechanism, confidence, `trading_enabled_block`,
  `label_block`, and time-to-scam when known;
- each row linked to chain-truth, token-builder, or investigation evidence;
- each verified scam assigned to a mechanism cluster;
- unresolved cases marked `needs_chain_truth` or `needs_mechanism_review`, not
  forced into a confident label;
- a short cohort summary showing counts by mechanism and time-to-scam bucket.

## Full-Corpus Extension Goal

After the first 100-token checkpoint, extend the same token-centric schema to
all currently indexed pools with a verified `scam_mechanism`. The expanded
ledger should support category-specific deep dives, starting with
`direct_lp_liquidity_removal`.

For the direct-LP category, build rows that can answer:

- when the LP was approved relative to trading enabled, pool creation, and
  liquidity removal;
- whether LP approvals, holder concentration, or liquidity state were visible
  before the removal block;
- which static token/pool attributes were known before the label;
- which token-network and block-activity signals were present before a selected
  prediction horizon;
- whether the removal happens within the next `1`, `2`, `3`, `5`, `10`, `15`,
  `20`, `30`, `50`, `100`, `250`, or `500` active token/pool observations.

Controls are preliminary while the source token range build is still running.
Treat them as observed-negative-so-far examples until the range completes and
the labels are regenerated.

The `run-2` full-range export has completed. Use
`tools/build_current_snapshot.py` to regenerate the labels and direct-LP feature
files from one range-view snapshot, then run
`tools/validate_direct_lp_features.py` before using the training rows.

## Current Modeling Questions

- How many blocks does a pool survive from creation to `label_block`?
- How many blocks and approximate minutes does it survive from
  `trading_enabled_block` to `label_block`?
- What early graph/fund-flow features appear before the label?
- Which labels are predictable from launch behavior versus only from the drain
  transaction itself?
- How much of the signal is protocol/router noise versus actor coordination?
- How does the conditional risk curve change after each active token/pool
  observation?
- How much accuracy/actionability do within-horizon targets give us versus an
  exact-next-observation target?

## Target Construction

For active-observation horizons, construct an ordered observation timeline per
pool. If the removal label is at active observation index `R`, a positive row
for horizon `N` uses `as_of_active_observation_index = R - N`, includes only
evidence through that observation, and sets the target to true. A control row is
negative only when the pool is observed through at least `N` later active
observations without a removal label.

Rows should keep the chain-block coordinates as supporting metadata:

- `as_of_block`;
- `label_block`;
- `chain_blocks_to_label`;
- `chain_blocks_until_active_horizon_end`, when known.

The current completed export already includes active block/activity features,
but its `prediction_horizon_blocks` and `blocks_before_removal` columns are
chain-block horizons. Treat it as a baseline and execution-risk dataset, not the
final active-observation target.

When building intuition for this target, inspect individual pools block by block:
the useful row is the state after one active observation and before future
observations are known. The model should eventually reproduce that live mental
loop mechanically for every active observation in the training corpus.

## Leakage Rules

Feature rows must declare an `as_of_block`.

- Training rows for a horizon `N` can use only observations with
  `observation_block <= as_of_block`.
- A chain-block row is positive if `label_block <= as_of_block + N`.
- An active-observation row is positive if the label occurs within the next `N`
  active token/pool observations after the `as_of` observation.
- Do not include reserve collapse, sell failure, or backdoor transfer evidence
  that happens after `as_of_block`.
- Post-label investigation notes belong in labels or artifacts, not model
  features.

## First Data Sources

Start with already reviewed token-lab investigations, then expand to
token-server and indexed chain-observation outputs:

- `token_lab/investigations/*/investigation.toml`
- token server pool summaries with `scam_mechanism`
- source observation rows with first bad block and sell failures
- network analytics timeline features for suspicious graph/fund-flow behavior

The seed label ledger is `labels/scammed_pools.seed.csv`.
