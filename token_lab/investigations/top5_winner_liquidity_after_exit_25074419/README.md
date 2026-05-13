# Top-5 Winner Liquidity-After-Exit Review

## Scope

Backtest run: `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

This review checks whether the corrected top-five winners are profitable
because the simulator sold before a later liquidity collapse, or because the
backtest incorrectly valued an already-dead pool.

## Result

The four low-liquidity winners were sold before their first source observation
that marks the pool as scammed/cannot-sell. Probing the same recorded sell
amount at the later bad block fails down to 1%, so the current low-liquidity
state would not be counted as sellable if the position were still open.

| Rank | Token | Sell block | Sell proceeds ETH | First bad block | Blocks after sell | Last clean WETH | First bad WETH | First bad result |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 2 | `0x0654750A6012bD42e06a0f1141153Dc8C60F7Fb1` | `25,077,522` | `2.329928443477287333` | `25,077,722` | `200` | `17.21054620646172` | `0.000017262333154253` | sell fails down to 1% |
| 3 | `0xDBb77651dF8A633A4E9D8c858E46878845878B20` | `25,077,831` | `2.006753067216354612` | `25,077,998` | `167` | `18.27684282640402` | `0.0010138925720517` | sell fails down to 1% |
| 4 | `0x8CEDa8619ad186E7C9bCA77734e48c8f518f5d01` | `25,077,529` | `1.055073350228747716` | `25,077,595` | `66` | `1.7998005484757544` | `0.000018052161427478` | sell fails down to 1% |
| 5 | `0x3B0e37179fC806302f4A15f85577a06A6Eb38B4F` | `25,074,622` | `0.808653169922843955` | `25,074,698` | `76` | `3.3927639346942295` | `0.000034029727508584` | sell fails down to 1% |

The rank-1 token's primary backtested pool is different: it remains liquid in
the checked source range and on current chain state. Other pools for the same
token were later marked bad, but the backtested primary pool stayed sellable.

## Current Chain Reserves

Direct `getReserves()` checks confirm the current low-liquidity state for ranks
2 through 5:

| Rank | Pool | Current WETH reserve |
| ---: | --- | ---: |
| 1 | `0x08368cfb1b69f5821ec0e8340bee726568f8d662` | `31.206256149320130295` |
| 2 | `0x25aeb70269c8e9d4c80d2c82dcd74d813ee1efd9` | `0.000017262333154253` |
| 3 | `0x97cddc3607198b137b37bc68af11c2eba8868f3f` | `0.0010138925720517` |
| 4 | `0xd8e654a9b4e861b54adb903ada20e3787dbd0079` | `0.000028052161427478` |
| 5 | `0xd2888a9f952e2dda7676f320322aca8e117f4c5a` | `0.000034029727508584` |

## Drain Transactions

The post-exit drain swaps observed for the low-liquidity winners are:

| Token | Bad block | Drain transaction |
| --- | ---: | --- |
| `0x065475...0F7Fb1` | `25,077,722` | `0xc4fc96637c8814dfbbfeee7b0a3994e3f362cda73d867328342daf68f953afc4` |
| `0xDBb776...878B20` | `25,077,998` | `0xbf27f0a617e80ba2c51540a7861dc6017be5f725540767592e4f979c1d6ce85b` |
| `0x8CEDa8...8f5d01` | `25,077,595` | `0x834fead8ee0ed75b360441ad448351509f897dab5f2be655c3aef8a801f78697` |
| `0x3B0e37...b38B4F` | `25,074,698` | `0xc8b945bc88184ee9fc0723945de5219cb786221c163fd2901d5be9dffe4a717b` |

## Conclusion

The corrected 15k backtest is giving the right answer for these top winners
under the configured policy: buy, then max-hold sell after about 200 blocks,
with risk exits disabled. The profits are not coming from valuing low-liquidity
pools after they died; they are realized exits before the later rug/low-liquidity
state.

This does not validate a never-sell policy. If max-hold is removed, these same
tokens would become open exposure and the later failed-sell/zero-value behavior
must be counted instead.
