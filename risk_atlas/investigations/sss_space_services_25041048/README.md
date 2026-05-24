# SSS SpaceX Space Services

Status: display/read-model and durable observation fields implemented. A fresh
token-builder range replay on 2026-05-20 reproduces the reserve collapse,
same-block WETH drain, post-drain dust-token reserve state, and `cannot_sell`
read model. The chain-parity question is settled; the remaining work is to
decide whether to add earlier warning detectors around the direct `sync()`.

This investigation tracks the SSS pool whose displayed price ratio became
extremely high while the pool was also marked `cannot_sell`.

The first objective was parity, not classification:

1. Extract the chain truth for the pool and key transactions.
2. Replay the same range with the token builder.
3. Confirm whether the token builder can reproduce the chain behavior.
4. Decide which odd behavior detectors this investigation should produce.

## Addresses

- token: `0x144742cc48cacb0f2e49dd03af5cfca3fa30e7bc`
- pool: `0x06f53d7c71f91060d7e698218d78ff293f4f2aa5`
- denom: `WETH`
- creator: `0xd962715b742d7a761a09a5141d85679e8472a6b0`

## Range

- start block: `25,041,048`
- end block: `25,041,137`

## Key Transactions

- `initial_liquidity`:
  `0x1d8bcaff714a59ba96aba2db3ac58e7862723382d02394aaac6d541d2bcbbe5e`
- `first_tracked_buy`:
  `0xf398d2afa25ae98b7576382a8e7487070bad7d9954b41e2cdfadcfee80cdbf20`
- `creator_sync_after_reserve_collapse`:
  `0x620827c84f54de392802cbc7b7fb7fdf565591d26481d4eb1ec1e7d47d3bc660`
- `weth_drain_sell`:
  `0x2c7f9398b329df81b5a526ab8f400a8b90c81206e47b4550f6c8b00efcaaac1b`
- `later_buy`:
  `0x43ea4fbb24b326397e5f782c556be4b47f0406107bf263709e2f0c9250bb5c96`

## Chain Truth

Initial observations from the range `25,041,048..25,041,137`.

### Key Reserve Points

| Block | Event | Token Reserve | WETH Reserve | Notes |
| --- | --- | ---: | ---: | --- |
| 25,041,048 | initial LP sync | 1,000,000,000.000000000 | 1.000000000000000000 | First pool sync. |
| 25,041,049 | first tracked buy | 835,138,624.660307288 | 1.198000000000000000 | First price point used by saved build. |
| 25,041,117 | pre-manipulation | 119,838,970.280179918 | 8.594042892376489107 | Pool still has meaningful token and WETH reserves. |
| 25,041,123 | creator sync | 119.838970281 | 8.594042892376489107 | Token reserve collapses by roughly 1,000,000x without WETH moving. |
| 25,041,123 | WETH drain sell | 119,838,970.280179918 | 0.000008619902574299 | A sell-like tx drains WETH. |
| 25,041,137 | later buy | 697.712149384 | 1.485008619902574267 | Pool has WETH but only dust token reserve. |

### Chain Interpretation

- `initial_liquidity` creates the pair, sends `1,000,000,000` SSS and `1 WETH`,
  then syncs.
- `creator_sync_after_reserve_collapse` is a direct pool call from the creator
  that emits a `Sync` after the token reserve collapsed.
- `weth_drain_sell` transfers SSS into the pool and drains about `8.594 WETH`.
- `later_buy` buys after the WETH drain and leaves the pool with only
  `697.712149384` SSS.

The high price ratio is mathematically consistent with pool reserves. It is not
evidence of healthy price appreciation. It is a reserve-quality problem: WETH is
present while token reserve is dust relative to total supply.

## Token-Builder Replay

### Checks Run

- Replayed `initial_liquidity` and verified pair creation, LP state, and
  reserves through the range read model.
- Replayed `first_tracked_buy` and verified buy activity.
- Replayed `creator_sync_after_reserve_collapse` and verified that the token
  builder records the direct creator `sync` with zero token or denom transfers;
  chain truth shows `balanceOf(pool)` collapsed before `sync()`.
- Replayed `weth_drain_sell` and verified the observed sell-like tx succeeds and
  drains WETH.
- Replayed `later_buy` and verified the post-buy reserve state.

### 2026-05-20 Token-Builder Result

Server-backed token builder run `run-1` over `25,041,048..25,041,137`
completed from disk cache with `0` transaction failures and `0` pool simulation
failures. It produced the SSS range-end pool snapshot at block `25,041,137`.
This is not the current/latest market state shown by Dexscreener:

| Field | Value |
| --- | --- |
| `token_reserve` | `697.712149384` |
| `denom_reserve` | `1.4850086199025743 WETH` |
| `raw_price_ratio_to_initial` | `2128397.249802325` |
| `pooled_token_supply_percent` | `0.0000697712149384` |
| `can_buy` / `can_sell` | `true` / `false` |
| `has_observed_buy` / `has_observed_sell` | `true` / `true` |
| `stage` | `CANNOT_SELL` |
| `risk_level` / `risk_label` | `honeypot` / `cannot_sell` |

The key transactions were present in token-builder activity rows:

- `creator_sync_after_reserve_collapse`: zero token transfers and zero denom
  transfers in the creator tx.
- `weth_drain_sell`: `8.594034272473914 WETH` sell volume in the same block.
- `later_buy`: `1.485 WETH` buy volume, ending in the dust-token reserve state.

Artifact:
`artifacts/token_builder_replay_20260520.md`.

### Same-Prestate Simulator Classification

If we run a narrower synthetic same-prestate sell and simulator output differs
from chain truth, classify the mismatch as one of:

- missing prior tx setup;
- missing token control or balance mutation;
- incorrect pool/token metadata;
- incorrect pool orientation;
- incorrect tax or sell simulation path;
- simulator EVM/state bug;
- expected limitation with a documented reason.

### Current Status

Token-builder reproduction is complete. The remaining parity question is narrower:
only run a same-prestate synthetic sell if we need to prove whether
`observed_sell_simulator_fail` is real. The current range replay itself does not
prove that detector, because the run had zero pool simulation failures and the
WETH drain sell is observed from executed chain data.

## Findings

### Confirmed So Far

- The final pool reserves match chain replay.
- The extreme price ratio comes from a tiny token reserve against non-zero WETH.
- The pool has at least one observed sell-like transaction on chain.
- The token builder reproduces the observed sell-like WETH drain and final
  range-end `can_buy=true`, `can_sell=false` state.
- The creator-triggered `sync()` after token reserve collapse is a critical
  suspicious event.

### Mechanism Classification

SSS belongs in the reserve-drain scam family. The closest existing production
label is `pair_balance_backdoor_drain` / `Backdoored Pair-Balance Drain`, but
the exact low-level balance mutation still needs final proof before we mark it
verified under that label.

The confirmed dynamic is:

1. the pool had meaningful token and WETH reserves;
2. the pair's actual SSS token balance collapsed by roughly `1,000,000x`;
3. the creator called `sync()`, which made the pair's stored reserves reflect
   the already-collapsed token balance;
4. WETH was then drained by a same-block sell-like transaction;
5. later buys saw a pool with non-zero WETH but token-reserve dust, making raw
   AMM price ratios misleading.

So the detector is not "any `sync()` is bad". The detector candidate is the
compound pattern:

```text
creator/control sync
  + no matching normal token/denom transfer explanation
  + extreme token reserve discontinuity
  + meaningful WETH still present
  + later or same-block WETH drain / cannot-sell state
```

Until the exact balance mutation is classified, this should be treated as a
probable `pair_balance_backdoor_drain` variant, with `unknown_reserve_drain` as
the fallback label if the backdoor evidence cannot be proven.

### Detector Candidates

- `sync_without_transfer`
- `reserve_discontinuity`
- `token_reserve_dust`
- `price_ratio_extreme_low_supply`
- `pair_balance_backdoor_drain` when the unexplained pair-balance mutation is
  proven
- `unknown_reserve_drain` as the conservative fallback
- `observed_sell_simulator_fail` only if a same-prestate synthetic sell fails

### Open Work

- Decide whether the range builder should flag this pool before the WETH drain,
  at the direct `sync()`, or both.
- Decide whether to run a same-prestate synthetic sell to distinguish privileged
  seller behavior from simulator route/account gaps.

### Display Rule

The displayed price/init ratio should be zeroed when the quote side is liquid
but the pooled token reserve is dust relative to declared token supply. SSS is
the anchor example: the raw `2,128,397x` price ratio is mathematically
consistent with reserves, but it should not be presented as price appreciation
because only `0.0000697712149384%` of supply remains in the pool and
`can_sell=false`.

The read model keeps raw price ratio separately and labels the display reserve
quality as `token_reserve_dust` so the UI can explain why the displayed ratio is
suppressed.

The persisted Risk Atlas observation row now also carries:

- `pooled_token_supply_ratio`
- `reserve_quality_status`
- `price_to_initial_ratio_trustworthy`

These fields let modeling and review queries keep the raw price ratio while
filtering or explaining untrusted price evidence.

## Artifacts

Generated outputs for this investigation belong under `artifacts/`.
