# Alpha11 Hold3 Live Mined Validation - 2026-05-21

## Verdict

Passed the one-position mined-validation gate for the happy path:

- buy request accepted by Kartal policy;
- buy accepted and signed by the local signer;
- buy broadcast to public mempool;
- buy mined successfully and reconciled as `buy_confirmed`;
- hold3 exit emitted from the confirmed position;
- sell accepted, signed, and broadcast;
- sell mined successfully and reconciled as `sell_confirmed`;
- Kartal was returned to `dry_run` after the run.

This does not clear multi-position, replacement, reorg, delayed inclusion, or
live gas-rank production-readiness gates.

## Run

| Field | Value |
| --- | --- |
| Run id | `alpha11-hold3-public-validation-20260521-192640Z` |
| Result set | `live-alpha11-hold3-public-validation-20260521-192640Z` |
| Strategy | `alpha11-live-univ2-lp30-pool-update-block-hold3-validation` |
| Trade id | `trd_mpfvtkp7_1bn2n_1` |
| Token | `0xE5C7B9e4d20032D5D7d137C2e5d7633FE7268aC8` |
| Pool | `0xe5c7b9e4d20032d5d7d137c2e5d7633fe7268ac8:0xbfaced0714af69f2cd097c4844cdb121066b7087` |
| Final state | `sell_confirmed` |
| Asena | `http://127.0.0.1:40019/eth/backtest/live/live-alpha11-hold3-public-validation-20260521-192640Z/strategies/alpha11-live-univ2-lp30-pool-update-block-hold3-validation/` |

## Transactions

| Side | Order id | Tx hash | Mined block | Tx index | Gas used | Effective gas price | Gas cost |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Buy | `trd_mpfvtkp7_1bn2n_1-entry-25145709` | `0x5352907ec78da608fb10142794a7e71ba95022af924d8f0de527ecb471cd62c4` | `25145710` | `3` | `176908` | `40.112736704 gwei` | `0.007096264024831232 ETH` |
| Sell | `trd_mpfvtkp7_1bn2n_1-priority-exit-25145716` | `0x8edd9c243263e57519d74fda6042093f163a53871c7bbc9839b66948d405182d` | `25145717` | `5` | `190897` | `40.106058212 gwei` | `0.007656126194496164 ETH` |

## Economics

| Field | Value |
| --- | ---: |
| Entry cost | `0.01 ETH` |
| Exit value | `0.008087566815837533 ETH` |
| Total gas cost | `0.014752390219327396 ETH` |
| Realized PnL | `-0.016664823403489863 ETH` |
| ROI | `-166.64823403489863%` |

The negative result is dominated by the temporary validation gas envelope:
`40 gwei` priority fee on both buy and sell. The gate was a pipeline proof, not
a profitability test.

## Kartal State After Run

Kartal was returned to `broadcast_mode = dry_run`.

Daily spend ledger after the real buy and sell:

```text
spent_wei = 40000000000000000
remaining_daily_cost_wei = 20000000000000000
max_daily_cost_wei = 60000000000000000
```

The ledger kept the real broadcast reservations and released dry-run/signing
probe reservations.

## Notes

- The earlier readiness probes exposed a stale Docker build context; the
  order-server image was rebuilt from current source before this run.
- Restarting the signer recreated `/run/kartal`; the order-server container had
  to be recreated so it could see the current signer socket.
- Public broadcast was enabled only for this validation run and disabled
  immediately afterward.
