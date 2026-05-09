# SSS SpaceX Space Services

This investigation tracks the SSS pool whose displayed price ratio became
extremely high while the pool was also marked `cannot_sell`.

The first objective is parity, not classification:

1. Extract the chain truth for the pool and key transactions.
2. Replay the same transactions with our simulator.
3. Confirm whether the simulator can reproduce the chain behavior.
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

## Simulator Parity

### Checks To Run

- Replay `initial_liquidity` and verify pair creation, LP state, and reserves.
- Replay `first_tracked_buy` and verify buy output and post-reserves.
- Replay `creator_sync_after_reserve_collapse` and verify whether token
  `balanceOf(pool)` collapses before `sync()`.
- Replay `weth_drain_sell` and verify the observed sell-like tx succeeds and
  drains WETH.
- Replay `later_buy` and verify the post-buy reserve state.

### Failure Classification

If simulator output differs from chain truth, classify the mismatch as one of:

- missing prior tx setup;
- missing token control or balance mutation;
- incorrect pool/token metadata;
- incorrect pool orientation;
- incorrect tax or sell simulation path;
- simulator EVM/state bug;
- expected limitation with a documented reason.

### Current Status

Pending. The investigation has chain observations, but transaction-level
simulator parity still needs to be run.

## Findings

### Confirmed So Far

- The final pool reserves match chain replay.
- The extreme price ratio comes from a tiny token reserve against non-zero WETH.
- The pool has at least one observed sell-like transaction on chain.
- The creator-triggered `sync()` after token reserve collapse is a critical
  suspicious event.

### Detector Candidates

- `sync_without_transfer`
- `reserve_discontinuity`
- `token_reserve_dust`
- `price_ratio_extreme_low_supply`
- `observed_sell_simulator_fail`

### Open Work

- Confirm simulator replay parity for the key transactions.
- Determine whether the observed sell can be reproduced by the simulator at the
  same pre-state.
- Decide whether the range builder should flag this pool before the WETH drain,
  at the direct `sync()`, or both.
- Decide how Asena should display price ratio when reserve quality is unsafe.

## Artifacts

Generated outputs for this investigation belong under `artifacts/`.
