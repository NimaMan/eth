# Mechanism Classification

Cases that define or refine scam mechanisms, sell-failure buckets, or suspicious
token/pool behavior labels.

Use this category when the primary work is to name the mechanism correctly. Do
not use it merely because a strategy position won or lost money; position and
backtest validation belongs in
`../../../../alpha/lab/strategy_analysis/backtest_validity/`.

## Questions

Ask these in order:

| Question | Why It Matters |
| --- | --- |
| What changed on chain first? | Distinguishes the earliest observable warning from later symptoms. |
| Was the quote-side loss caused by canonical LP removal, a swap/dump, pair-balance mutation, hidden mint/rebase, tax behavior, or route/address restriction? | Prevents unrelated failure modes from sharing one vague label. |
| Did LP ownership actually grant removal power at the time of the event? | Avoids calling a reserve drain "LP removal" when LP was burned, locked, or not spent. |
| Did token supply or pair token balance change without normal ERC-20 or pair events explaining it? | Identifies hidden mint/rebase and pair-balance backdoor families. |
| Was `sync()` the cause, or only the step that published an already-manipulated balance into pair reserves? | `sync()` is a normal V2 method; the suspicious part is the preceding balance manipulation. |
| Who could execute the behavior: creator/control wallet, any holder, a privileged seller, or only a specific route/address? | Determines whether the mechanism is a broad scam signal, a route-specific risk, or an Alpha execution limitation. |
| Could Alpha's actual executable route buy and sell at the same chain coordinate? | Separates avoid-only mechanism evidence from recoverable-exit evidence. |
| Is the signal visible early enough to use for live entry rejection or exit? | Determines whether the mechanism becomes a live detector, historical label, or display-only warning. |
| What durable label should Risk Atlas persist and display? | Converts a one-off case into a reusable behavior catalog entry. |
| What should not be concluded from this case? | Records boundaries, such as "not LP removal" or "not proof our route can exit." |

Promote shared answers to `../../../investigations/behavior_catalog/`.

## Cases

| Status | Case | Current Disposition |
| --- | --- | --- |
| confirmed | [sss_creator_transfer_from_pair_drain_25041123](../../cases/sss_creator_transfer_from_pair_drain_25041123/) | Creator/control `transferFrom(pair, ...)` is the first state-changing warning, `sync()` publishes the manipulated balance, and the WETH drain follows in the same block. |
| explained | [vyp_burned_lp_reserve_drain_25077324](../../cases/vyp_burned_lp_reserve_drain_25077324/) | LP was locked; WETH was drained by backdoored pair-balance transfer plus sell, not normal LP removal. |
| explained | [transfer_from_failed_exit_classification_25065694](../../cases/transfer_from_failed_exit_classification_25065694/) | Failed exits are bucketed into address restriction, chunking, no observed sell, Pancake V2 unsellable, and V3 drained liquidity. |

Shared labels promoted from these cases belong in
`../../../investigations/behavior_catalog/`.
