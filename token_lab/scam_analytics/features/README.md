# Scam Feature Contract

Features are computed per pool at a declared `as_of_block`.

## Current Feature Exports

| File | Purpose |
| --- | --- |
| `direct_lp_liquidity_removal_features.csv` | Draft category-specific training feature table for pools labeled `direct_lp_liquidity_removal`. |
| `direct_lp_liquidity_removal_window_features.csv` | Positive horizon rows for the same pools at fixed offsets before removal. |
| `direct_lp_liquidity_removal_training_windows.csv` | Positive horizon rows plus observed-negative-so-far control rows for supervised experiments. |

Generate labels and feature exports from one consistent range-view snapshot:

```text
python3 token_lab/scam_analytics/tools/build_current_snapshot.py --run-id run-2
python3 token_lab/scam_analytics/tools/validate_direct_lp_features.py
```

The direct feature builder can still be run alone for development, but the
snapshot builder is preferred while `run-2` is active because the token-server
range view can advance between separate label and feature exports.
The builder can retry transient token-detail 404s with `--detail-retries`, and
merges detailed pool views into summary pool rows so `liquidity_history` and
`price_ratio_history` can populate `*_as_of` window features when detail views
are available. Keep retries low while `run-2` is active; use higher retries for
the final post-completion export if needed.

The current direct-LP export uses `as_of_block = label_block - 1` for the
category feature table.
Block-activity features are filtered to `as_of_block`. LP control fields and
token-network graph counts are currently marked with scope columns because the
token-server summary is not fully time-sliced yet; use those scope columns before
treating a field as training-safe.

The window export currently defaults to `1,10,50,100,500` chain blocks before
removal. Positive rows skip offsets before the token, pool, or trading-enabled
block existed. Training windows include completed-range controls from liquid,
tradeable pools with no current scam mechanism. When the source run is
completed, the exporter marks controls as observed through the completed range
in `control_reason`.

This is not yet the final scam-evolution target. The final target should use
active token/pool observation horizons:

```text
P(direct LP liquidity removal within the next N active token/pool observations | state up to observation O)
```

An active observation is a chain block where the token or pool has meaningful
state or signal movement: pool swap, mint, burn, sync/reserve update, LP
approval or transfer, token transfer touching the pool, buy/sell observation,
liquidity update, or price-ratio update. Idle chain blocks should not advance
the scam-evolution horizon.

Use within-horizon targets as the first supervised target family:

```text
target_N = true if label_active_observation_index - as_of_active_observation_index <= N
```

The first active-horizon set should be:

```text
1, 2, 3, 5, 10, 15, 20, 30, 50, 100, 250, 500
```

This gives detail near the immediate future while still keeping medium-term
survival/rug timing. An exact-next-observation target can be exported as a
secondary timing diagnostic, but it should not replace the within-horizon risk
curve until the data shows it is more useful.

Direct-LP rows include both current LP approval state and pre-removal approval
timing fields. Negative `blocks_trading_enabled_to_*_approval` values mean the
approval was already visible before the selected trading-enabled block.
They also include actor-network proxy fields that can be computed without the
disabled range-run token graph, such as owner/creator equality and whether the
last visible LP approval owner matches the creator or current owner.

Block-activity features include raw counts plus activity density, recent/total
transaction shares, buy/sell volume balance, transfer ratios, and first/last
activity blocks at `as_of_block`. Active-block counts are unique block counts,
not raw activity-row counts.

## Row Identity

Each model row should include:

| Field | Meaning |
| --- | --- |
| `chain` | Chain name. |
| `token` | Token address. |
| `pool` | Pool address or stable pool id. |
| `as_of_block` | Last block included in the features. |
| `horizon_blocks` | Chain-block prediction horizon for execution-risk rows. |
| `as_of_active_observation_index` | Ordered token/pool observation index included in the features for active-horizon rows. |
| `horizon_active_observation_blocks` | Active token/pool observation horizon for scam-evolution rows. |
| `active_observations_before_label` | Number of active observations from `as_of` to the label, for positive rows. |
| `label_block` | First known scam/blocked/drain block, empty for negatives. |
| `target_scam_within_horizon` | `true` for chain-block rows when `label_block <= as_of_block + horizon_blocks`. |
| `target_scam_within_active_horizon` | `true` for active-horizon rows when the label occurs within the next `N` active observations. |

Keep chain-block coordinates on active-horizon rows as metadata, including
`as_of_block`, `label_block`, `chain_blocks_to_label`, and
`chain_blocks_until_active_horizon_end` when the horizon endpoint is observed.

## Feature Families

Early pool features:

- blocks since pool creation;
- quote reserve, token reserve, reserve slope, and liquidity concentration;
- LP holder concentration, dead/burned LP share, approvals by LP holders;
- pool protocol and quote denomination.

Token mechanics:

- ownership/authority state;
- mint/burn activity;
- failed sell route observations;
- tax/transfer restriction observations;
- allowance anomalies such as `transferFrom(pair, actor)` without allowance.

Network features:

- token-network address growth by block;
- token-transfer bursts without denom counterpart;
- many-to-one and one-to-many transfer motifs;
- edge spikes with no new addresses;
- pool-recycling transfer count;
- creator/owner centrality.

Second-order fund-flow features:

- shared non-protocol funder/sink cluster count;
- largest non-protocol cluster size;
- percent of flow edges explained by routers/protocol addresses;
- seed wallets funded by the same connector before launch;
- direct ETH/WETH flow between token actors outside token transfers.

Token market lifecycle observations:

- trading enabled block;
- trading enabled evidence source;
- first successful non-owner buy block;
- first successful non-owner sell block;
- first failed sell block;
- blocks and approximate minutes from trading enabled to first bad block;
- price-to-init ratio before label;
- quote liquidity before label;
- max sell size / chunkability.

## Leakage Guard

For every feature, store the latest evidence block that contributed to it. A
training export should reject rows where `feature_latest_block > as_of_block`.
For active-horizon rows, feature evidence must also be at or before
`as_of_active_observation_index`; the future active observations are used only to
assign the target.
