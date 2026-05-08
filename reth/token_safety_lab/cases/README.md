# Cases

Each case is a concrete token or pool investigation. Cases should be small
enough to reproduce and specific enough to become regression tests.

Use a stable folder name:

```text
<token_symbol_or_name>_<short_context>_<start_block>
```

Examples:

```text
sss_space_services_25041048
biba_primary_v2_pool_25039257
```

Cases are not just notes. A complete case should explain:

- what the range builder reported;
- what the chain actually did;
- whether the simulator can reproduce it;
- whether a detector or guardrail should exist before trading;
- what code changed because of the investigation.

## Candidate Ledger

This is the working triage ledger for behavior that does not make sense yet.
Rows here are not conclusions. They are prompts for chain-truth and simulator
parity investigation.

| Status | Token | Pool | Range / Block | Symptom | Why It Matters | Case |
| --- | --- | --- | --- | --- | --- | --- |
| fixed | `0xb5fcb8a2efefd2840b572d6e7de7afa9c03d1e6e` | pending pool from router add-liquidity | block `24,987,361`, tx `0x597c71bc501882cfada8df254ae9c333874ac585e235a4385ba1ab996056d630` | setup replay failed with `ds-math-sub-underflow` even though the mined tx succeeded | same-block approvals must be replayed before router/pool simulations, otherwise simulator failures can be false positives | same-block tracked-token approval prior |
| fixed | `0x2520c955fd49256e2e182dcac5aac54ab731db05` | `0x71b3b7adc000425f29da1249ed228e87d596cf69` | block `25,050,385`, tx `0x19eb15be312fb29ee8291d3f05759bfdb4dfe0a1d5422d50f465e7934d5d9c96` | live setup replay reported `Ownable: caller is not the owner`, but direct parent-block RPC call succeeds from the same owner | zeroed storage slots from live prestate diffs must clear the tracked live snapshot, otherwise owner/control state can remain stale | live-renounce-replay-state |
| fixed | `0xff426e468c33036700f52a3cd74b11ea40431086` | `0xb80b6c2453b82996950a81c42db446e95a3fa2d5` | blocks `25,050,310`, `25,050,316`, `25,050,322` | live simulation could not find the V2 pool contract at the parent block after applying priors even though chain RPC shows the pool exists | live simulations should prefer refreshed MDBX state when available and live snapshots must apply zeroed storage correctly | live-pool-contract-missing |
| fixed | `0xfbc5c28ea0662205b3d380eecee4ffeb5fda8fb3` | `0x0cdd211fba6ba61dae2a8200606b15acd62908f5` | block `25,050,415`, tx `0xcc57f1f69037a7e961aa9ac23fbcb3f50687b05e315e0abcba30e3c190b202d4` | live simulation could not resolve the token/WETH V2 pool at the parent block even though chain RPC shows the pool exists | stale live state selection or stale factory storage can create false pool-not-found errors | live-pool-not-found |
| fixed | multiple token-control priors | multiple V2 pools | blocks `25,047,390` through `25,048,809` | selected prior replay failed with `lack of funds (0) for max fee` before reaching contract execution | sparse live snapshots can omit EOA ETH balances; mined setup transactions should be funded enough for replay validation so balance gaps do not look like token/pool failures | prior-replay-sender-funding |
| fixed | live-created token metadata | pending deployment replay | live blocks after `25,050,506` | token metadata discovery reported `transaction validation error: lack of funds` while replaying pending creation transactions | metadata replay also runs on sparse live state, so pending deployment senders need the same validation-only balance funding | metadata-replay-sender-funding |
| fixed | multiple token-control and pool-update transactions | multiple V2 pools | warmup blocks around `25,044,213` through `25,044,455` | selected same-block setup priors failed with `TRANSFER_FROM_FAILED`, `Not enough tokens out`, or token-specific reverts even though the mined transactions succeeded | arbitrary same-block liquidity and swap priors can depend on earlier block-local state; updated and token-control-triggered pools should simulate against current-block state instead of replaying fragile selected priors | current-block-pool-simulation |
| fixed | live catchup pool simulations | multiple V2 pools | live transition blocks `25,050,680` through `25,050,741` | current-block pool simulations failed with `missing live block header` while the processed block already had the header and persisted head was near or ahead | disk-cache catchup can process blocks that are no longer retained in the small Redis live-header window and not yet visible through the read-only static-file provider; current-block simulations must use the processed block header hint | live-catchup-header-hint |
| fixed | live metadata discovery | token and V2 pool metadata reads | live transition blocks around `25,050,934` | optional metadata view calls failed with `missing live block header` after warmup entered live catchup | metadata discovery must tolerate the same Redis/static-file header gap as a soft miss, otherwise optional reads become transaction failures | live-metadata-header-soft-miss |
| fixed | live token and V2 pool discovery | optional metadata reads | warmup blocks around `25,046,198` and `25,046,199` | live pool metadata lookup timed out after `2500 ms` and was counted as a transaction failure | metadata is a discovery aid, not chain execution; live timeout should be logged as a soft miss so token application remains deterministic and can rediscover later | live-metadata-timeout-soft-miss |

Status values:

- `new`: captured from the token range builder, UI, or logs.
- `investigating`: chain-truth or simulator parity work has started.
- `confirmed`: the behavior is real and needs a detector, guardrail, or display rule.
- `explained`: the behavior is real but expected, or the display should clarify it.
- `fixed`: code or display logic has been changed and verified.
- `ignored`: not useful after review.
