# How it was scammed

Token **e (0x2CedB62299Ad3422Fc0b0ec57180695501D15fef)** was launched, pumped, and rugged by the operator `0x9d58c75A8749a94C9CF9539B4f625Ed42936105D`.

## The short version

1. The operator **deployed** the token (block 25180893, tx `0x04c678d5ba91f2c07c02343042c6bb3a1fd657601ecf1166b0738bcec784c565`).
2. They **added about 1.0000 ETH of liquidity** to the Uniswap pool `0x5D43262637B4fc4bfaF4164A8974c436D5CFCe8D` (around block 25181124), which sets the starting price.
3. They **opened trading** (block 25181124, tx `0x0ef032ee97e3024e8cd7f9c0a76b99356759aac4f6b8b4e62f00bf4e9490bc66`). As buyers came in, the price was inflated **1.2x** — from a starting price of 1.332e-9 ETH/token up to a peak of 1.574e-9 ETH/token (peak around block 25181138).
4. With the price pumped, the operator **pulled the liquidity** (block 25181165, tx `0xba87aaeaff423a9c312a3432919c6191951639277c5d1835bfa09bf71b65fb33`), taking roughly **1.0643 ETH** back out of the pool.
5. The moment the liquidity was gone, the price **collapsed to ~0** (down to 0.0000% of the peak), so everyone who bought is left holding tokens that can no longer be sold for anything.

## The numbers

- **Mechanism:** Direct LP Liquidity Removal
- **ETH added as liquidity:** 1.000000 ETH
- **ETH removed at the rug:** 1.064254 ETH
- **Net ETH the operator extracted (removed − added):** 0.064254 ETH
- **Price inflation:** 1.2x (from 1.332e-9 to 1.574e-9 ETH/token)
- **Price after the rug:** 0.000e0 ETH/token (0.0000% of peak)
- **Trading observed:** 5 buys, 2 sells, 5 distinct buyers caught in the pump.

## Timeline (how it unfolded)

| Block | Phase | What happened | Tx |
| ---: | --- | --- | --- |
| 25180893 | token_deploy | Operator deployed the token contract | `0x04c678d5...84c565` |
| 25180926 | token_fund | Operator funded the token / seeded ETH | `0xfa786c9b...eb5b16` |
| 25181124 | first_trade | First on-chain trade established a live price | `0x0ef032ee...90bc66` |
| 25181124 | open_trading | Operator opened trading (buys enabled) | `0x0ef032ee...90bc66` |
| 25181138 | peak | Price peaked (top of the pump) | - |
| 25181163 | lp_approval | Operator approved LP tokens to the router (pre-stage the rug) | `0x867dafbf...828d44` |
| 25181165 | collapse | Price collapsed to ~0 after liquidity was pulled | - |
| 25181165 | liquidity_removal | Operator removed the liquidity (the rug pull) | `0xba87aaea...65fb33` |

## Scope and confidence

- Replayed pool `0x5D43262637B4fc4bfaF4164A8974c436D5CFCe8D` over blocks 25180893..=25181215 (12 active blocks).
- Price/liquidity numbers are derived from on-chain reserve snapshots (reth_db_replay_token_state_builder). Trading counts are derived from swap events (tool_derived_from_swap_events).
- Notes:
  - trading counts from pool.swap_events (7 events); price/liquidity from reserve_history deltas

_Machine-readable detail: `artifacts/scam_mechanics.json`._
