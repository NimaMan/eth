# Worst-Loser Position Checks

## Scope

Backtest run: `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

This review checks representative `-0.01 ETH` losers from the corrected 15k
baseline. Many worst positions tie at full entry loss, so the useful check is
by failure mode rather than row order.

## Cases Checked

| Case | Token | Type | Entry block | Latest / sell block | Result |
| --- | --- | --- | ---: | ---: | --- |
| V4 dead pool | `0x0000001897b916600Df47f6fB695853E68AC846E` | open buy-confirmed | `25,080,028` | `25,080,077` | latest pool state is zero reserve / not sellable |
| V2 rug before exit | `0x022bC1A0560D88Dd537AD51522B1cF63CD46E0c7` | open buy-confirmed | `25,075,629` | `25,075,665` | same recorded amount sells before collapse, fails after collapse |
| Full-size sell restriction | `0x11aEE4BE385E1f9437f9AaC38287A3052BE80F3F` | sell failed | `25,077,762` | `25,077,989` | full exit fails; 5%, 2%, 1% chunks sell at exit block |

## V4 Dead Pool

Token `0x0000001897b916600Df47f6fB695853E68AC846E` entered a Uniswap V4 pool
with `2.0 WETH` equivalent denomination reserve and `can_buy/can_sell=true`.
The position remained open, and the latest snapshot is zero because the source
observation at block `25,080,077` has:

- denomination reserve `0.0`;
- token reserve `0.0`;
- `can_buy=false`;
- `can_sell=false`;
- `is_scam=true`.

Independent V4 probe at block `25,080,076` still buys and sells. The same probe
at block `25,080,077` is no longer tradeable and reports zero target tokens.

## V2 Rug Before Exit

Token `0x022bC1A0560D88Dd537AD51522B1cF63CD46E0c7` entered at block
`25,075,629`. It was still sellable at block `25,075,661`:

- probe full recorded amount succeeds;
- proceeds are `0.329266293275335387 ETH`.

At block `25,075,665`, the pool reserve collapsed:

- WETH reserve `0.000006008039952332`;
- `can_buy=false`;
- `can_sell=false`;
- `is_scam=true`.

The same recorded sell amount fails from 100% down to 1% with
`TRANSFER_FROM_FAILED`. The zero latest value is therefore a real post-rug
valuation, not an accounting artifact.

The drain swap observed at the bad block is:

`0x68a3e22b47cccc6e830adb1f04f3d4863efa0b2d9a3d0b72f3eb1cb7190f72c3`

## Full-Size Sell Restriction

Token `0x11aEE4BE385E1f9437f9AaC38287A3052BE80F3F` is different. The pool still
looks tradeable by source observation at the max-hold exit block:

- WETH reserve `3.155109014258318`;
- `can_buy=true`;
- `can_sell=true`;
- `is_scam=false`.

But the strategy's full recorded sell amount fails with
`TransferHelper: TRANSFER_FROM_FAILED`, so the position remains `sell_failed`
with a zero-value snapshot.

Chunk probes at block `25,077,989` show the policy gap:

| Chunk | Result |
| ---: | --- |
| 100% through 10% | fail |
| 5% | succeeds, `0.001549705725883525 ETH` |
| 2% | succeeds, `0.000620065026033905 ETH` |
| 1% | succeeds, `0.000310062980935728 ETH` |

The current baseline has no partial-exit policy, so treating this as zero
exposure is conservative for the current strategy. A chunked-exit strategy
would produce a different result and needs separate accounting.

## Conclusion

These representative worst losers support the current accounting:

- open positions in dead pools are marked to zero;
- failed full-size exits remain open and zero-valued;
- chunkable failures are strategy-policy losses, not simulator infrastructure
failures.
