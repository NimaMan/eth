# VYP Burned-LP Reserve-Drain Review

## Scope

This investigation answers why the token page can show the VYP/WETH pool LP
tokens held by a burn address while the pool later behaves as drained.

- backtest run: `live-noncapital-maxhold50-riskbundle-delay1-gas-70k-20260514-codex`
- replay source: `snipe-all-v1-chain-sim-live-v4`
- token-builder run: `run-1` over blocks `25023119..25093118`
- strategy: `snipe-all-v1`
- token: `0x8ceda8619ad186e7c9bca77734e48c8f518f5d01`
- symbol: `vyp`
- pool: `0xd8e654a9b4e861b54adb903ada20e3787dbd0079`
- denom: `WETH`

## Result

This is not a contradiction. The LP position was sent to a burn address, but
the later WETH reserve collapse was caused by a large token-to-WETH swap through
the Uniswap V2 router. A swap can drain the WETH side without spending or
burning any LP tokens.

The more precise scam classification is:

```text
LP-lock trust trap + backdoored token transferFrom against the pair + reserve drain
```

It does not look like a standard hidden mint. `totalSupply()` stayed constant
at `1,000,000,000 vyp`, and the only `Transfer(0x0, ...)` mint log in the
reviewed range is the initial supply mint at token creation. The drain actor
instead used the token contract to call `transferFrom(pool, actor, amount)` even
though the pair-to-actor allowance was zero.

In this case, the better wording is:

- LP ownership is effectively locked after block `25077327`.
- The pool's WETH reserve is later drained by a sell/dump swap at block
  `25077595`.
- The UI/table can infer "liquidity removed" from a reserve collapse, but that
  is not the same as a canonical V2 `removeLiquidity` / LP-token burn event.

## Backtest Position

The delay-1 backtest bought and sold before the later reserve collapse:

| Event | Block | Value |
| --- | ---: | ---: |
| buy submitted | `25077324` | `0.01 ETH` |
| buy confirmed | `25077325` | `6377480.479650131 vyp` |
| sell submitted | `25077376` | - |
| sell confirmed | `25077377` | `0.689385953199905591 ETH` |
| realized PnL after gas | `25077377` | `+0.679321667943697037 ETH` |

There are no risk events for this position in the backtest run. The exit is
consistent with the `max_hold_blocks = 50` policy, not a mempool or liquidity
removal exit.

## LP Token Evidence

The token-builder LP table shows:

- LP total supply: about `1.0`
- holder `0x000000000000000000000000000000000000dead`: about `1.0` LP
- holder `0x8ceda8619ad186e7c9bca77734e48c8f518f5d01`: `0.0` LP, with one router
  approval from block `25077324`

That approval row is not removable LP exposure. The approved owner has zero
current LP balance, so the effective approved LP is `0.00%`.

Chain truth:

| Block | Tx | Evidence |
| ---: | --- | --- |
| `25077324` | `0x7c85c06cdb305532033309190ea98dbca78e89369be4a166176d34073db7f5c7` | pair created, initial VYP/WETH added, LP minted |
| `25077327` | `0x6c07721ee377840bf4c9720c4693b704646ba8ac89efab9c090329e1664a41e3` | deployer transferred `0.999999999999999` LP to `0x...dead` |

Block-state checks:

| Block | Dead LP balance | Deployer LP balance | Total supply |
| ---: | ---: | ---: | ---: |
| `25077325` | `0` | `0.999999999999999` | `1.0` |
| `25077328` | `0.999999999999999` | `0` | `1.0` |
| `25077595` | `0.999999999999999` | `0` | `1.0` |

Note that sending LP tokens to `0xdead` does not reduce ERC-20 `totalSupply`.
The UI labels that holder as `burn` because the holder is a burn address.

## Token Supply Evidence

VYP has 9 decimals.

| Check | Raw amount | Scaled amount |
| --- | ---: | ---: |
| `totalSupply()` at launch block `25077324` | `1000000000000000000` | `1,000,000,000 vyp` |
| `totalSupply()` at drain block `25077595` | `1000000000000000000` | `1,000,000,000 vyp` |
| `totalSupply()` latest checked | `1000000000000000000` | `1,000,000,000 vyp` |

Exact mint/burn log checks over blocks `25077324..25081722`:

- `Transfer(0x0, actor, 1,000,000,000 vyp)` appears once at token creation tx
  `0x4f55f16ce6d60a16155d2cc1264d16e6bca06b3c4a1456deafda31fb46d74652`.
- No `Transfer(anyone, 0x0, ...)` burn logs were found in the checked range.

So this should not be classified as hidden-mint based on total supply or
standard ERC-20 transfer logs.

## Reserve-Drain Evidence

The pool remained sellable after the LP transfer to `0xdead`. The source
observations show healthy WETH reserves through the backtest exit:

| Block | WETH reserve | Token reserve | can sell | is scam |
| ---: | ---: | ---: | --- | --- |
| `25077325` | `1.2457579070108855` | `803247764.7656809` | `true` | `false` |
| `25077376` | `10.862195780772552` | `93825911.46513487` | `true` | `false` |
| `25077493` | `28.284524657374707` | `36976006.19766866` | `true` | `false` |
| `25077595` | `0.000018052161427478` | `588081647.5197332` | `false` | `true` |

The drain transaction is:

```text
0x834fead8ee0ed75b360441ad448351509f897dab5f2be655c3aef8a801f78697
```

It calls the Uniswap V2 router method
`swapExactTokensForETHSupportingFeeOnTransferTokens`:

```text
amountIn: 588075766.703258098 vyp
amountOutMin: 0
path: [vyp, WETH]
recipient: 0x50995f97f63f8ec9a131272269f1388d88a03990
```

The pair emits a `Swap`, not a `Burn`, with:

```text
amount0In:  588075766.703258098 vyp
amount1Out: 1.799782496314326996 WETH
```

After that swap, `getReserves()` at block `25077595` is:

```text
reserve0: 588081647.519733296 vyp
reserve1: 0.000018052161427478 WETH
```

There are no LP `Transfer` or V2 `Burn` logs for the pool at block `25077595`.

## Backdoor Evidence

The actor did not hold the dump-sized token balance before the drain:

| Block | Actor VYP balance | Pair VYP balance | Pair WETH reserve |
| ---: | ---: | ---: | ---: |
| `25077594` | `0` | `588081647.519733296` | `1.799800548475754474` |

The pair-to-actor allowance was also zero:

| Block | `allowance(pair, actor)` |
| ---: | ---: |
| `25077594` | `0` |
| `25077595` | `0` |

Despite that, tx
`0x8cf6ba5c36844df6bd8aae184173038293ca95819747649e01573a623ecdf1a0`
successfully called the token contract with:

```text
transferFrom(
  from:   0xd8e654a9b4e861b54adb903ada20e3787dbd0079,
  to:     0x3b6a5e29d68251265d5ca522f79caad95392b47b,
  amount: 588075766.703258098 vyp
)
```

That amount is about `58.8076%` of total supply. The actor then sold the same
amount through the router in tx
`0x834fead8ee0ed75b360441ad448351509f897dab5f2be655c3aef8a801f78697`,
draining almost all WETH from the pair.

## Mempool Arrival Check

The local mempool arrival index did not record either critical transaction:

| Tx index | Tx | Role | Arrival recorded |
| ---: | --- | --- | --- |
| `158` | `0x8cf6ba5c36844df6bd8aae184173038293ca95819747649e01573a623ecdf1a0` | backdoored `transferFrom(pair, actor, amount)` | no |
| `161` | `0x834fead8ee0ed75b360441ad448351509f897dab5f2be655c3aef8a801f78697` | router sell / WETH drain | no |

The recorder was active for this block: it recorded arrivals for `158` of the
`255` transactions in block `25077595`, including nearby txs at indexes `156`,
`157`, `162`, and `163`. The contiguous unrecorded cluster at indexes
`158..161` is consistent with private/builder-only propagation or local mempool
misses. It should not be treated as proof that no public node saw it, but our
local live mempool pipeline did not see these two tx hashes before inclusion.

## Interpretation

Burning or dead-address-locking LP protects against the LP holder later calling
`removeLiquidity`. It does not protect the pool from malicious token logic. In
this case the malicious behavior is worse than a normal large holder dump:
`transferFrom` allowed the actor to source dump-sized tokens from the pair
despite zero allowance.

For this token, the observed "liquidity removed" behavior is really a reserve
drain / dump:

1. liquidity is added at block `25077324`;
2. LP is transferred to `0xdead` at block `25077327`;
3. the backtest exits at block `25077377`;
4. the deployer/actor uses a backdoored `transferFrom` against the pair at block
   `25077595`;
5. the actor sells that same amount back through the router;
6. WETH reserve falls from a peak near `28.2845 WETH` to about `0.000018 WETH`.

So the current low-liquidity pool state does not invalidate the backtest sell.
The backtest sold before the reserve drain.

## Follow-Up

- UI labels should distinguish `LP burned/locked` from `reserve drained`.
- Reserve-drop derived "liquidity removed" labels should say they are inferred
  from reserve history, not from a V2 LP burn/removeLiquidity event.
- Strategy risk naming should keep `liquidity_removal` for explicit LP removal
  and use a separate `reserve_drain` or `dump_drain` label for this pattern.
- Detector work should add a high-severity token-contract backdoor signal when
  a non-pair caller successfully moves tokens out of a pair with zero observed
  allowance, especially when followed by a same-block router sell.
