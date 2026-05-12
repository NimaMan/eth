# ETH Alpha Lab

Offline diagnostics for alpha strategy and position sanity checks.

The lab reads existing `alpha_trading` tables and does not participate in the
live trading path. It currently uses:

- `trader_runs`
- `positions`
- `execution_reports`
- `position_snapshots`
- `strategy_observations`

## Strategy Lab

Run-level checks for PnL, concentration, failures, protocol mix, missing
snapshots, and accounting anomalies.

```bash
eth_alpha_lab --database-url "$MEMPOOL_DATABASE_URL" strategy \
  --run-id hist-poolonly-mtm-v4-20260512-1021
```

Multiple runs can be reported together:

```bash
eth_alpha_lab --database-url "$MEMPOOL_DATABASE_URL" strategy \
  --run-id hist-poolonly-mtm-paper-20260512-1021 \
  --run-id hist-poolonly-mtm-v4-20260512-1021
```

## Token Lab

Single position/token checks for entry report consistency, observation joins,
latest valuation, and first/last mark-to-market trajectory.

```bash
eth_alpha_lab --database-url "$MEMPOOL_DATABASE_URL" position \
  --run-id hist-poolonly-mtm-v4-20260512-1021 \
  --token 0x12a77658112Cf42914cB614D13653ed5852DA1e5
```

If more than one position matches a token, use `--position-id`.

Both commands support `--json` for automation.
