# Backtest Treatment

Result set:
`live-alpha11-univ2-lp30-pool-update-block-hold-sweep-chain-sim-block-frame-20260529-152040017864493Z`

The live backtest bought the Session pool in all ten sweep strategies and left
every trade in `buy_confirmed`.

| Strategy | Trade | State | Entry | Latest Valuation | Current Value | Total PnL | ROI |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| `alpha11-all-pools-lp30-pool-update-block-hold16` | `trd_mpr6c4zm_8fxz_41` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold100` | `trd_mpr6c50x_8fxz_46` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold12` | `trd_mpr6c4yk_8fxz_3x` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold14` | `trd_mpr6c4yu_8fxz_3y` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold15` | `trd_mpr6c4z3_8fxz_3z` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold16` | `trd_mpr6c4zc_8fxz_40` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold18` | `trd_mpr6c4zv_8fxz_42` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold20` | `trd_mpr6c504_8fxz_43` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold25` | `trd_mpr6c50e_8fxz_44` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |
| `alpha11-univ2-lp30-pool-update-block-hold50` | `trd_mpr6c50n_8fxz_45` | `buy_confirmed` | `25202409` | `25202437` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |

## Hold16 Snapshots

`alpha11-univ2-lp30-pool-update-block-hold16` trade
`trd_mpr6c4zc_8fxz_40` marked value from pool reserve price after the real
vault balance had already been drained at block `25202411`.

| Block | State | Current Value ETH | Total PnL ETH | ROI |
| ---: | --- | ---: | ---: | ---: |
| `25202409` | `buy_confirmed` | `0.004873964726377231` | `-0.000355239526216813` | `-7.105%` |
| `25202414` | `buy_confirmed` | `0.005065688038887566` | `-0.000163516213706478` | `-3.270%` |
| `25202415` | `buy_confirmed` | `0.005460105057925380` | `0.000230900805331336` | `4.618%` |
| `25202422` | `buy_confirmed` | `0.008900893974578217` | `0.003671689721984173` | `73.434%` |
| `25202423` | `buy_confirmed` | `0.011495138558036543` | `0.006265934305442499` | `125.319%` |
| `25202425` | `buy_confirmed` | `0.020380123064957084` | `0.015150918812363040` | `303.018%` |
| `25202436` | `buy_confirmed` | `0.045356795283255376` | `0.040127591030661332` | `802.552%` |
| `25202437` | `buy_confirmed` | `0.023827528402072455` | `0.018598324149478411` | `371.966%` |

## Risk Events

Only one risk event was recorded for the token in the live backtest run:

| Block | Kind | Severity | Count |
| ---: | --- | --- | ---: |
| `25202408` | `lp_approval` | `warning` | `1` |

There was no `balance_drain`, `holder_drain`, `vault_balance_loss`, or
equivalent post-entry risk event for block `25202411`.

## Interpretation

The backtest did not model the real vault balance as an authoritative state
variable after entry. It settled the simulated buy, then valued the position
from pool reserves and the simulated token amount. Because the scam path reduced
the vault's token `balanceOf` without a normal `Transfer` log, the backtest
continued to show profitable open inventory after the inventory was gone.
