# Mechanism Classification

Cases that define or refine scam mechanisms, sell-failure buckets, or suspicious
token/pool behavior labels.

## Cases

| Status | Case | Current Disposition |
| --- | --- | --- |
| confirmed | [sss_creator_transfer_from_pair_drain_25041123](../../cases/sss_creator_transfer_from_pair_drain_25041123/) | Creator/control `transferFrom(pair, ...)` is the first state-changing warning, `sync()` publishes the manipulated balance, and the WETH drain follows in the same block. |
| explained | [vyp_burned_lp_reserve_drain_25077324](../../cases/vyp_burned_lp_reserve_drain_25077324/) | LP was locked; WETH was drained by backdoored pair-balance transfer plus sell, not normal LP removal. |
| explained | [transfer_from_failed_exit_classification_25065694](../../cases/transfer_from_failed_exit_classification_25065694/) | Failed exits are bucketed into address restriction, chunking, no observed sell, Pancake V2 unsellable, and V3 drained liquidity. |

Shared labels promoted from these cases belong in
`../../../investigations/behavior_catalog/`.
