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

Cases are not just notes. A complete case `README.md` should explain:

- what the range builder reported;
- what the chain actually did;
- whether the simulator can reproduce it;
- whether a detector or guardrail should exist before trading;
- what code changed because of the investigation.

Case folders should normally contain only:

- `README.md`: the single narrative markdown file;
- `case.toml`: machine-readable metadata;
- `artifacts/README.md`: artifact folder instructions.

Generated artifacts belong under `artifacts/` and are ignored by default.

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
| fixed | multiple V2 tokens | multiple V2 pools | historical range run `run-2` review | observed swap processing could mark `can_buy` before the buy/sell simulator result was applied | trading decisions must trust simulator-backed `can_buy` / `can_sell`; observed swaps remain evidence but should not set execution viability flags | simulator-owned-trading-status |
| fixed | multiple V4 tokens | multiple V4 pools | live warmup and V4-enabled range review | V4 pool status was being probed through a local custom executor path instead of the chain route used by mined swaps | V4 simulation now uses the deployed Universal Router plus Permit2 for the supported pool-buy-sell path; hook-heavy or actual-route-only cases still need explicit route parity checks | v4-route-equivalent-simulator |
| fixed | VIRPEG, SYX, MPAD, and other short-lived V4 launches | multiple V4 pools | historical range run `run-1`, blocks around `25,042,400` through `25,045,000` | dust or drained V4 pools could still show old `can_buy`, `can_sell`, or `trading_enabled` flags after liquidity collapsed | current tradeability must reflect the latest pool liquidity; first-buy metadata can remain historical, but dust/drained/scam pools must not be shown as currently buyable or sellable | v4-drained-pool-stale-trading-flags |
| fixed | `0x000000000000bb1b11e5ac8099e92e366b64c133`, `0xe0b7927c4af23765cb51314a0e0521a9645f0e2a`, `0x38c6a68304cdefb9bec48bbfaaba5c5b47818bb2` | optional V2 pool metadata reads | historical range run `run-2`, blocks `24,986,518`, `24,990,735`, `24,990,736`, `24,993,747` | pool metadata discovery counted token `decimals()` failures as transaction failures | V2 pool metadata is optional; malformed/nonstandard metadata should not make historical block application diverge from chain progress | historical-pool-metadata-decimals-soft-miss |
| investigating | WCT `0x9765b6f6f781006e96d4f2b7be3470bb91754b6a` | `0x71ed1657631bed970b5059c49aed709c5b7e8c7e` | blocks `24,991,564` through `24,991,596` | range builder reports `can_buy=true`, `can_sell=false`; same wallet bought from the pool and later sold successfully on chain | simulator uses the classic V2 router path, while the mined WCT buy/sell path used Universal Router / Permit2; route mismatch can create false `cannot_sell` labels | wct_universal_router_sell_parity_24991564 |
| investigating | SCREAM `0x7e68b4acab2804af5caafebbf8ee18f862a33b91` | `0xfec520dba4e7fd7626022593c23f8d8cb597769a` | blocks `24,995,988` through `24,996,193` | simulator flips to `can_sell=false` after renounce with `TransferHelper: TRANSFER_FROM_FAILED`, but later chain sells succeed | some post-flip sellers bought through a bot/aggregator entrypoint before selling through V2; parity needs an actual-route replay before deciding if this is a simulator false positive or route-dependent honeypot logic | scream_route_dependent_sell_parity_24995988 |
| investigating | BAG `0xf3c425fcf069bc90607976a4401f380af5e91597` | `0x29c081435a7e602fb2b2f8f99f4dc89a274e36a1` | blocks `24,987,948` through `24,987,962` | range builder reports `can_buy=true`, `can_sell=false`; current evidence is observed buys through an external router and no observed sells in the focused range | `TransferHelper: TRANSFER_FROM_FAILED` may mean route-dependent token behavior rather than a generic simulator error; confirm whether any later sells exist before trading against the pool | bag_external_buy_sell_parity_24987948 |
| investigating | Xtrim `0x16a8c532ad7273194ea2bb0de3dd58ff7e882274` | `0x4b2fe3799481001bc70e34e7f8b809753fb627cc` | blocks `24,987,207` through `24,988,451` | simulator reports `can_buy=true`, `can_sell=false` with `UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT` rather than `TransferHelper: TRANSFER_FROM_FAILED` | this is a separate simulator/amount-quality case: determine whether the simulated buy output is dust, transfer-taxed, or incorrectly extracted before treating the pool as buyable | xtrim_dust_sell_input_parity_24987207 |

Status values:

- `new`: captured from the token range builder, UI, or logs.
- `investigating`: chain-truth or simulator parity work has started.
- `confirmed`: the behavior is real and needs a detector, guardrail, or display rule.
- `explained`: the behavior is real but expected, or the display should clarify it.
- `fixed`: code or display logic has been changed and verified.
- `ignored`: not useful after review.
