# DBB Top Winner Scam-After-Exit Review

## Scope

Token: `0xDBb77651dF8A633A4E9D8c858E46878845878B20`

Pool: `0x97cddc3607198b137b37bc68af11c2eba8868f3f`

Backtest run: `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

This is the rank-3 top winner in the corrected 15k baseline.

## Backtest Outcome

- Entry block: `25,077,629`
- Entry time: `2026-05-12 07:45:59 UTC`
- Entry cost: `0.01 ETH`
- Entry amount: `9,674,148.737091200` tokens
- Sell block: `25,077,831`
- Sell time: `2026-05-12 08:26:23 UTC`
- Sell proceeds: `2.006753067216354612 ETH`
- Realized PnL: `+1.996753067216354612 ETH`

The exit was a max-hold style exit around 200 blocks after entry, before the
pool was marked scammed.

## Scam Timing

The source observation stream first marks this pool as scammed at block
`25,077,998`, `2026-05-12 08:59:47 UTC`, 167 blocks after the backtest sell.

Pool state moved from tradeable to drained:

| Block | WETH reserve | Token reserve | can buy | can sell | is scam |
| ---: | ---: | ---: | --- | --- | --- |
| `25,077,962` | `18.27684282640402` | `55,525,327.542106465` | yes | yes | false |
| `25,077,998` | `0.0010138925720517` | `1,006,900.182826085` | no | no | true |

That is a `99.9944525836%` WETH reserve collapse between the last clean
observation and the scam observation.

## Drain Evidence

The decisive block is `25,077,998`. The key drain swap is:

`0xbf27f0a617e80ba2c51540a7861dc6017be5f725540767592e4f979c1d6ce85b`

The transaction came from `0x3fc7A68C936138d1C1bc9DC1403c099f9E626eF5` and
called Uniswap V2 Router `swapExactTokensForETHSupportingFeeOnTransferTokens`.
It sold `55,525,272.016778923` DBB into the pool and withdrew
`18.276824494565733750 WETH`, leaving the pool with near-zero WETH.

The previous transaction in the same block:

`0xf8e6927fd1a9a27f7778de42d0f4ec601f9c25d7e92505a4274ddec2df371a02`

moved the same DBB amount from the pair to `0x3fc7A68C...` through the token
contract before the dump. This is not a normal LP burn in our decoded evidence;
it is a pool-token extraction followed by a dump that drains WETH.

## Simulator Check

At the actual backtest exit block `25,077,831`, selling the recorded full
amount succeeds and the strategy seller receives `2.006753067216354612 ETH`.

At scam block `25,077,998`, selling the same recorded amount fails at every
tested size from 100% down to 1% with:

`Sell transaction failed: TransferHelper: TRANSFER_FROM_FAILED`

## Conclusion

The historical PnL for this position is valid under the current non-mempool
backtest semantics. The strategy exited before the pool became unsellable. This
case is also a useful policy warning: extending max-hold or missing this exit
would turn the same token into failed/zero exposure.
