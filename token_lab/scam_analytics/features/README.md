# Scam Model-Row Contract

This folder documents the model-row contract and leakage rules. It no longer
stores generated flat-file training rows.

Source feature construction belongs in `eth_token::token_analytics`, where
active token/pool observations and as-of feature families are defined in Rust.
Rows built for modeling should join those source features with scam labels and
targets owned by `token_lab/scam_analytics`.

Each model row needs a declared `as_of_block`. Active-observation rows also need
`as_of_active_observation_index`.

The first scam-evolution target uses active token/pool observation horizons:

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

The first active-horizon set is:

```text
1, 2, 3, 5, 10
```

This keeps the first model focused on immediate risk. An exact-next-observation
target can be exported later as a secondary timing diagnostic, but it should not
replace the within-horizon risk curve until the data shows it is more useful.

Direct-LP rows should include both as-of LP approval state and pre-label
approval timing fields. Negative `blocks_trading_enabled_to_*_approval` values
mean the approval was already visible before the selected trading-enabled block.

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
| `horizon_active_observations` | Active token/pool observation horizon for scam-evolution rows. |
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
