# SSS Creator TransferFrom Pair-Balance Drain

Status: chain parity and Alpha-route replay complete; detector research open.

This is the second unresolved Risk Atlas issue after the Compass helper-route
V2 vault parity issue. The broad SSS investigation proved token-builder/range
parity and fixed misleading price display. This focused investigation owns the
exact block `25,041,123` mechanism and the policy question that follows from it.

The ordering for this investigation is deliberate:

1. Prove what chain truth did at the exact transaction coordinates.
2. Prove whether our local processor/simulator can replay those coordinates.
3. Only then decide whether the pattern becomes a live scam-avoidance signal.

## Coordinates

| Field | Value |
| --- | --- |
| token | `0x144742cc48cacb0f2e49dd03af5cfca3fa30e7bc` |
| pool | `0x06f53d7c71f91060d7e698218d78ff293f4f2aa5` |
| creator | `0xd962715b742d7a761a09a5141d85679e8472a6b0` |
| drain wallet | `0x0e0dca0fd767b394837fa1c908e8fc2f944aca6c` |
| recipient | `0x36c11106814ef31b14eeb42391c39c95917f6019` |
| range | `25,041,048..25,041,137` |
| creator pool-token transfer tx | `0x487bb861bbd8f86e65e39f3560afa18b8ca99f7b955f0e276f50739f54cbcefb` |
| creator sync tx | `0x620827c84f54de392802cbc7b7fb7fdf565591d26481d4eb1ec1e7d47d3bc660` |
| drain wallet router approval tx | `0xa377b2d316609bec0683da9023ec0db7b1cc6973d301299925dec9428fa6958d` |
| WETH drain sell tx | `0x2c7f9398b329df81b5a526ab8f400a8b90c81206e47b4550f6c8b00efcaaac1b` |

## Chain-Parity Finding

The earlier working hypothesis was "direct sync after unexplained pair balance
collapse." Exact chain truth refines that:

```text
tx 197: creator calls SSS.transferFrom(pool, drain_wallet, 119,838,850.441209632 SSS)
tx 198: creator calls UniswapV2Pair.sync()
tx 199: drain wallet approves router
tx 200: drain wallet sells the received SSS through the V2 router and drains WETH
```

So the first-degree mechanism is not hidden reserve mutation by `sync()`.
`sync()` is still important, but it is the reserve-state publication step after
a creator-controlled pool-token transfer has already changed the pair's actual
SSS balance.

## Exact Block Sequence

SSS has `9` decimals. The pool token orientation is:

| token0 | token1 |
| --- | --- |
| `SSS` | `WETH` |

| Coordinate | Chain Event | Token Balance/Reserve | WETH Balance/Reserve | Interpretation |
| --- | --- | ---: | ---: | --- |
| block `25,041,122` end | pre-drain state | `119,838,970.280179913 SSS` | `8.594042892376488995 WETH` | Meaningful liquidity is still present. |
| block `25,041,123`, tx `197` | creator `transferFrom(pool, drain_wallet, ...)` | pool sends `119,838,850.441209632 SSS`; pool token balance becomes about `119.838970281 SSS` | unchanged | Creator can move pool-held SSS through the token contract. This is the core anomaly. |
| block `25,041,123`, tx `198` | creator direct `sync()` | reserve becomes `119.838970281 SSS` | reserve stays `8.594042892376488995 WETH` | `sync()` copies the manipulated pair balance into stored reserves. |
| block `25,041,123`, tx `199` | drain wallet approval | no pair reserve change | no pair reserve change | Drain wallet prepares router sell. |
| block `25,041,123`, tx `200` | router sell | pool receives `119,838,850.441209632 SSS`; reserve becomes `119,838,970.280179913 SSS` | pool sends `8.594034272473914696 WETH`; reserve becomes `0.000008619902574299 WETH` | WETH is drained after the manipulated reserve state. |

RPC/debug evidence:

- tx `197` input selector is `0x23b872dd` (`transferFrom(address,address,uint256)`).
- tx `197` emits `Transfer(pool -> drain_wallet, 119,838,850.441209632 SSS)`.
- tx `197` also emits `Approval(pool -> creator, 0)`.
- Parent-block `allowance(pool, creator)` reads `0`, so the successful
  `transferFrom` is not explained by a normal standing ERC-20 allowance.
- tx `198` call trace shows the pair calling `balanceOf(pool)` on SSS and WETH;
  SSS returns `119.838970281`, WETH returns `8.594042892376488995`.
- tx `200` call trace shows the router moving SSS from the drain wallet back to
  the pair, then the pair transferring WETH to the router for unwrap/recipient
  payout.

## Local Replay Status

The local `ProcessedTxProvider::process_transaction_by_hash` path initially
failed on these target transactions because it requested default tracing options
from the fused block replay engine. That engine accepts `callTracer` options,
and the provider expects a `CallTracer` result anyway.

Fix applied:

```text
tx_processor/src/processed_tx_provider/provider.rs
```

The provider now requests `GethDebugTracingOptions::call_tracer(...)` for the
targeted partial-block replay.

After that fix, local Reth-backed replay succeeds for the target transactions:

| Tx | Index | Local Result |
| --- | ---: | --- |
| creator pool-token transfer | `197` | processed successfully; call trace captured |
| creator `sync()` | `198` | processed successfully; token/WETH balance calls captured |
| WETH drain sell | `200` | processed successfully; router/pair/WETH call tree captured |

This proves the local transaction replay path can process the exact block
coordinates. Alpha route parity is a separate check because production trading
uses the deployed V2 vault path, not an arbitrary observed sender.

## Alpha Route Parity

Alpha's current executable V2 route is the deployed Uniswap V2 trading vault:

| Field | Value |
| --- | --- |
| vault | `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597` |
| owner/default sender | `0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27` |

That vault did not exist at the historical SSS block, so exact route replay
uses a code overlay. This is not chain truth for the historical address; it is
a route-capability probe that answers: if Alpha's current vault held SSS at
that prestate, could it sell through our production V2 path?

Replay setup:

- state source: `TxSimulator::block_tx_state_session(25_041_123)`;
- state coordinates: before tx `197`, after tx `197`, after tx `198`, and after
  tx `200`;
- code override: latest deployed vault bytecode overlaid onto the vault address;
- token override: synthetic `1 SSS` balance (`1,000,000,000` raw) assigned to
  the vault;
- balance-slot proof: SSS standard balance mapping slot `1`;
- gas override: synthetic owner ETH for gas only;
- min ETH out: `1 wei`.

Result:

| Coordinate | State | Route Result | Revert | Gas Used |
| --- | --- | --- | --- | ---: |
| tx `197` | before creator pair-token transfer | failed | `TransferHelper: TRANSFER_FROM_FAILED` | `123,403` |
| tx `197` | after creator pair-token transfer | failed | `TransferHelper: TRANSFER_FROM_FAILED` | `140,503` |
| tx `198` | after creator `sync()` | failed | `TransferHelper: TRANSFER_FROM_FAILED` | `140,503` |
| tx `200` | after WETH drain sell | failed | `TransferHelper: TRANSFER_FROM_FAILED` | `120,397` |

The pre-tx-197 trace is decisive. The vault can call SSS `approve()` and then
calls the router's `swapExactTokensForETHSupportingFeeOnTransferTokens`, but
the router's `transferFrom(vault, pair, 1 SSS)` fails. The token also makes an
internal creator/control call with the 2300-gas stipend before the router
reports `TransferHelper: TRANSFER_FROM_FAILED`.

Conclusion: for this anchor case, Alpha's current deployed V2 vault route
cannot sell SSS even before the creator drains the pair's token balance. The
`control_transfer_from_pair_before_sync` pattern is therefore an avoid signal
for our route, not a reliable recoverable-exit signal. If a position is already
held, live code must still run exact route replay and gas policy before any
priority exit; this case must not be treated as proof that front-running the
drain is executable.

## Classification

Production family: `pair_balance_backdoor_drain`

Precise subtype: `creator_transfer_from_pair_then_sync_drain`

Fallback family if later replay contradicts this: `unknown_reserve_drain`

This is not direct LP liquidity removal. No LP burn/removal explains the WETH
loss. It is also not merely a generic reserve dump: the dump is enabled by a
creator/control transfer of pool-held SSS before the `sync()`.

The current production family is still appropriate because
`eth_token::BasePool::record_pair_token_transfer` is designed to flag token
movement out of the pair when there is no normal pool event explaining it.

## Why This Matters For Chain Parity

For SSS, a block-level or range-end snapshot is not enough. The simulator must
replay the block prefix in order:

1. parent state at block `25,041,122`;
2. tx `197`, which removes nearly all SSS from the pair;
3. tx `198`, which syncs reserves to the manipulated balance;
4. tx `199`, which approves the router;
5. tx `200`, which drains WETH.

If any replay path skips tx `197`, starts from the wrong prestate, or only looks
at end-of-block reserves, it can misclassify the pool as merely having a strange
price ratio instead of a creator-controlled pair-balance drain.

## Signal Candidate

Primary signal candidate:

```text
control_transfer_from_pair_before_sync
```

Feature components:

| Feature | Condition |
| --- | --- |
| `pair_token_out_transfer` | Token `Transfer` has `from=pair` and no matching pair `Swap`, `Mint`, or `Burn` event. |
| `control_sender` | Tx sender is creator, owner, deployer, or linked control wallet. |
| `transfer_from_pair` | Tx input calls token `transferFrom(pair, recipient, amount)`. |
| `large_pool_share` | Amount is a large fraction of the previous meaningful token reserve. |
| `denom_retained` | WETH balance/reserve remains meaningful immediately after the transfer. |
| `direct_sync_after_transfer` | Same sender or linked actor calls pair `sync()` soon after. |
| `nearby_denom_drain` | WETH drains in the same block or short horizon. |

Suggested severity:

- `critical` immediately in live mempool when a creator/control wallet submits
  `transferFrom(pair, ...)` for a large share of pool tokens while WETH remains.
- `critical` when followed by creator/control direct `sync()`.
- `confirmed scam mechanism` when a same-block or near-horizon WETH drain
  follows.

## Entry And Exit Policy Implications

Entry policy:

- Reject pools that already show this pair-token-out pattern before entry.
- Treat `sync()` alone as insufficient; benign syncs exist.
- Treat a creator/control `transferFrom(pair, ...)` as a stronger signal than
  direct `sync()` because it is the earlier state-changing step.

Exit policy:

- If holding and the mempool shows creator/control `transferFrom(pair, ...)`
  while WETH is still recoverable, attempt a priority exit only if route replay
  and gas-rank policy say the exit is executable and economic.
- If the evidence is only mined after WETH is already dust, do not submit an
  uneconomic sell.
- For the SSS anchor case, exact Alpha-route replay shows the current deployed
  V2 vault cannot sell even before the transfer/sync sequence. Treat this as
  avoid-only unless a later route-specific replay proves a different executable
  path.

## Open Work

1. Decide whether the live mempool detector should key on tx input
   `transferFrom(pair, ...)`, the token `Transfer(pool -> actor)` log, or both.
2. Run a corpus query for creator/control `transferFrom(pair, ...)` events with
   large previous reserve share and retained WETH.
3. Measure false positives for migrations, rescue operations, fee/reflection
   mechanics, and manual pool maintenance.
4. If false positives are acceptable, implement a Risk Atlas detector and add a
   regression fixture anchored on this coordinate.

## Related Files

- broad SSS investigation:
  `risk_atlas/investigations/sss_space_services_25041048/`
- behavior catalog:
  `risk_atlas/behavior_catalog/README.md`
- parity tools:
  `risk_atlas/tools/parity/README.md`
- production labels:
  `eth_token/src/pools/scam_mechanism.rs`
