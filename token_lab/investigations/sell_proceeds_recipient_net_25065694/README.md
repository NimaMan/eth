# Sell Proceeds Recipient-Net Parity

## Scope

Historical source range: `25,065,694..25,080,693`

Superseded run: `hist-snipe-all-poolonly-15k-weth-only-20260513-134623`

Corrected run: `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

This investigation covers confirmed V2 sells where gross denomination-token
output from the pool is not equal to the strategy seller's actual proceeds.

## Finding

The previous gross-output baseline fixed undercounted router exits, but it was
too broad. Some fee-on-transfer tokens trigger an internal token-contract
auto-swap in the same sell transaction. That auto-swap can emit WETH from the
pool to another recipient before the strategy seller's swap completes.

For token `0x12a77658112Cf42914cB614D13653ed5852DA1e5`, the old sell proceeds
extractor counted both WETH outputs:

- `5.268043340472091546 ETH` paid to `0xbAd06a3C...`, not the strategy seller;
- `3.762855185725959110 ETH` paid to strategy seller
  `0x0C96...5689`.

The correct strategy proceeds are the seller recipient's net ETH/WETH increase,
`3.762855185725959110 ETH`.

## Baseline Impact

The corrected 15k rerun has the same position and failure counts as the
superseded run, but lower PnL:

| Metric | Superseded gross-output run | Corrected recipient-net run |
| --- | ---: | ---: |
| Total PnL | `+15.137896975732239922 ETH` | `+9.693440469949935980 ETH` |
| Top token PnL | `+9.020898526198050656 ETH` | `+3.752855185725959110 ETH` |
| Top token sell proceeds | `9.030898526198050656 ETH` | `3.762855185725959110 ETH` |

Eight confirmed sell reports changed. The largest delta is the top token above;
the other seven deltas are smaller auto-swap/proceeds attribution cases.

## Fix

`extract_denom_received` now:

- first uses the strategy recipient's net ETH/WETH balance increase;
- falls back to gross pool-output denomination transfers only when recipient
  net proceeds are unavailable or zero.

This preserves the earlier fix for router exits whose seller net balance is not
directly visible, while preventing unrelated same-transaction pool outputs from
being booked as strategy PnL.

## Regression Hooks

- Unit tests in `tx_processor::simulator::sell_swap_simulator::tests` cover
  recipient-net precedence and gross-output fallback.
- `probe_sell_swap --verbose` prints relevant ERC20 transfers, V2 swaps, and
  balance changes for reproducing proceeds attribution.
