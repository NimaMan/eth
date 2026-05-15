# Full-Corpus Scam Analytics Goal

## Objective

Extend the token-centric scam analytics dataset from the first 100 reviewed
token/pool cases to the rest of the currently indexed scam-mechanism labels.
Then build category-specific training features, starting with
`direct_lp_liquidity_removal`.

The deep dive is about the token and pool evolution itself.

## Current First Category

Start with direct LP liquidity removals because they are the largest current
cluster. The primary scam-evolution training question is:

```text
P(pool suffers direct LP liquidity removal within the next N active token/pool observations | state up to observation O)
```

Use chain-block horizons as a secondary execution/capital-at-risk question:

```text
P(pool suffers direct LP liquidity removal within the next N chain blocks | state up to block B)
```

Active token/pool observations are blocks where the token or pool has meaningful
state movement or signal, such as swaps, mint/burn/sync/reserve updates, LP
approvals or transfers, token transfers touching the pool, buy/sell
observations, liquidity updates, or price-ratio updates. Idle Ethereum blocks
should not advance the primary scam-evolution horizon.

Use within-horizon labels first. The live question is whether the pool is likely
to become unsafe soon given its current state, not whether the removal happens
on one exact future observation. Exact-next-observation labels can be kept as a
secondary timing target after we inspect the target distributions.

Initial active-observation horizons:

```text
1, 2, 3, 5, 10, 15, 20, 30, 50, 100, 250, 500
```

## Required Label Fields

Each label row should keep:

- token and pool address;
- protocol and symbol when known;
- scam mechanism and confidence;
- pool creation block;
- trading enabled block and evidence source;
- label/removal block;
- blocks and approximate minutes from trading enabled to label;
- price ratio to initial and liquidity at label;
- evidence transaction or source reference.

## Direct-LP Training Attributes

Initial features should include:

- LP approval timing before removal;
- first and last LP approval blocks visible at `as_of_block`;
- blocks from LP approval to removal;
- LP holder and spender counts visible from approval data;
- whether router approval was visible before the prediction point;
- liquidity and price ratio to initial at `as_of_block`;
- liquidity drawdown from the pre-`as_of_block` peak;
- token/pool static attributes such as creator, owner, tax bucket, protocol,
  and creation/trading age;
- token-network graph counts and block-activity features with explicit scope
  columns until fully time-sliced exports exist.

## Leakage Rules

For any row with `as_of_block`, do not use observations after that block.
Chain-block positive rows must have `label_block <= as_of_block + horizon`.
Active-observation positive rows must have the removal inside the next `N`
active token/pool observations. Control rows are only clean after the source
range has completed and the pool remains unlabeled through the required
observation window.

While `run-2` is still running, generated control rows are
observed-negative-so-far examples only.

## Current Completed Export

`run-2` completed the full `24994815..25094814` range. The current export
contains:

- `7626` scam-mechanism label rows;
- `3565` direct-LP feature rows;
- `10736` positive direct-LP horizon rows;
- `2970` completed-range control rows;
- `13706` total direct-LP training-window rows.

Validate the export from the ETH repo root:

```text
python3 token_lab/scam_analytics/tools/validate_direct_lp_features.py
```

The current direct-LP export includes pre-removal LP approval timing, completed
range controls, static token/pool fields, actor-network proxy fields, detailed
liquidity/price histories where token detail exposes them, and block-activity
time-series features.

The current `direct_lp_liquidity_removal_training_windows.csv` uses chain-block
horizons in `prediction_horizon_blocks` and `blocks_before_removal`. It already
contains active block/activity features, so it is useful for a baseline and for
capital-at-risk timing, but it is not yet the final active-observation target.
The next export should add active observation indexes and active-horizon target
columns for `1`, `2`, `3`, `5`, `10`, `15`, `20`, `30`, `50`, `100`, `250`,
and `500` active token/pool observations.
