# Transfer From Failed Exit Classification

Status: explained policy backlog. The root causes are bucketed; remaining work
is strategy policy for retries, chunked exits, and address-specific/no-observed
sell exposure.

## Scope

Historical run:
`hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

Range: `25,065,694..25,080,693`

The goal of this investigation is to classify every failed sell in the current
15k historical baseline as either a chain-executable outcome, a strategy policy
gap, or a simulator/accounting gap.

## Current Run Facts

- 449 positions.
- 353 open positions.
- 11 buy-failed positions.
- 8 sell-failed positions.
- 554 execution reports: 523 confirmed, 31 failed.
- 20 failed sell reports.
- 19 failed sell reports are `TransferHelper: TRANSFER_FROM_FAILED`.
- 1 failed sell report is `Universal Router V3 sell transaction failed:
  V3InvalidSwap`.
- Confirmed sell proceeds now prefer recipient net ETH/WETH and only fall back
  to gross pool output when recipient net proceeds are unavailable.

## Open Failed Exits

| Token | Pool | Entry block | Failed reports | First fail | Last fail | Classification | Evidence |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| `0x4fD80A...130325` | `0xb4a5...21cc` | 25,074,185 | 1 | 25,075,260 | 25,075,260 | no observed sell evidence | Strategy seller fails down to 1%; no token-to-pool sell logs through current head. |
| `0xbfb08D...e212fd` | `0x5f77...52ee` | 25,075,851 | 1 | 25,076,700 | 25,076,700 | V3 drained zero-liquidity pool | Universal Router route parity is fixed. Strategy seller succeeds at observed sell block 25,075,896, but the max-hold exit block has `liquidity() = 0`, token balance `8`, WETH balance `2`, and returns `V3InvalidSwap`. |
| `0x2a6F60...e73D20` | `0xaa54...50c3` | 25,076,050 | 1 | 25,077,111 | 25,077,111 | no observed sell evidence | Strategy seller fails down to 1%; no token-to-pool sell logs through current head. |
| `0xdf1a39...30C90b` | `0x2200...13d8` | 25,077,548 | 1 | 25,077,981 | 25,077,981 | address-specific sell restriction | Observed chain seller `0xe84f...0131` sells successfully at the same block; strategy seller fails down to 1%. |
| `0x11aEE4...E80F3F` | `0xecce...e7d` | 25,077,762 | 1 | 25,077,989 | 25,077,989 | chunkable max-transfer / anti-whale | Full sell fails through 10%; 5%, 2%, and 1% chunks simulate successfully with nonzero proceeds. |
| `0x5E30BC...11cD06` | `0xfaf2...c4ea` | 25,078,348 | 12 | 25,078,589 | 25,079,812 | Pancake V2 unsellable for strategy route/address | Pancake protocol parsing is fixed; buy now confirms, every sell probe down to 1% fails, and no token-to-pool sell logs exist through current head. |
| `0xbe74B2...01Af21` | `0x2c02...c3c` | 25,077,634 | 1 | 25,079,186 | 25,079,186 | address-specific sell restriction | Observed chain seller `0x92d0...19e` sells successfully at the same block; strategy seller fails down to 1%. |
| `0x283431...c5B56C` | `0x035a...4024` | 25,080,333 | 1 | 25,080,664 | 25,080,664 | address-specific sell restriction | Observed chain seller `0x5bbe...5bb` sells successfully at block 25,080,768; strategy seller fails down to 1% at that later block too. |

## Closed Retry Evidence

`0xBCB2...962bd` is not an open failed exit in the current baseline. It failed
at block `25,074,899` and later sold successfully at block `25,074,918`.

This confirms the accounting/lifecycle rule: a failed sell must keep the
position open and retryable. It must not be counted as closed until a later
sell confirms.

## Strategy Implications

- Address-specific restrictions should remain zero-valued open exposure unless
  we can prove the strategy address can sell later.
- Chunkable failures need a partial-exit policy. The 11AE case shows that a
  full-size exit can fail while smaller chunks are executable.
- Pancake V2 is now protocol-covered. Its current failure is not unsupported
  infrastructure; it is an unsellable-position outcome for this strategy route.
- The V3 failed exit is no longer a route gap. It is a real drained-pool
  outcome at the strategy's exit block, and the existing zero-value snapshot is
  the correct PnL treatment.

## Regression Hooks

- Strategy Lab now prints an `Open Failed Exits` table for this run.
- V2 sell proceeds extraction is covered by focused unit tests in
  `tx_processor::simulator::sell_swap_simulator::tests`.
- Pancake V2 protocol parsing is covered by
  `wire::tests::parse_protocol_accepts_pancakeswap_v2_spellings`.
- V3 Universal Router sell encoding is covered by
  `tx_builders::uniswap_v3::tests`, including the unwrap command sequence.
- Universal Router custom error decoding is covered by
  `revert::reason::tests::test_decode_universal_router_v3_errors`.
