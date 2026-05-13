# Historical Backtest PnL Verification: 15K Blocks

Run ID: `hist-snipe-all-poolonly-15k-pnl-verify-20260513-205548`

Replay source: `snipe-all-v1-chain-sim-live-v4`

Block window: `25,073,636..25,088,635`

This is the latest executable 15,000-block window available from the replay source at run time. The live chain head was newer, but `alpha_trading.strategy_observations` for the replay source stopped at block `25,088,635`, so this run verifies the latest fully available replay window rather than inventing missing observations.

## Backtest Command

```bash
RUN_ID=hist-snipe-all-poolonly-15k-pnl-verify-20260513-205548
RUST_LOG=info cargo run --release -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id "$RUN_ID" \
  --replay-run-id snipe-all-v1-chain-sim-live-v4 \
  --from-block 25073636 \
  --to-block 25088635 \
  --skip-primed \
  --buy-amount-wei 10000000000000000 \
  --min-liquidity-eth 0.5 \
  --min-liquidity-usd 1000 \
  --max-hold-blocks 200
```

The source window had `28,291` `pool_update` observations. `3,032` were primed rows with `suppress_events=true`, leaving `25,259` replayed strategy events. No mempool signals were included.

## Result

```text
events processed:      25,259
execution reports:        613
confirmed reports:        581
failed reports:            32
positions:                498
open positions:           393
failed positions:          19
snapshots:             12,444
positions snapshotted:    487
```

Position states:

```text
buy_confirmed:   385
buy_failed:       11
sell_confirmed:   94
sell_failed:       8
```

The completed run metadata currently stores `"positions": 393`; that field was actually the open position count. Future runs now store `"positions"` as total positions and `"open_positions"` separately.

## PnL Verification

The lab report and independent SQL rollups agree:

```text
entry cost ETH:             4.87
latest current value ETH:   0.367133091739369945
realized PnL ETH:          14.692951799834758243
unrealized PnL ETH:        -3.562866908260630055
total PnL ETH:             11.130084891574128188
```

Independent fill accounting:

```text
confirmed buy cost:     4.870000000000000000 ETH
confirmed sell proceeds: 15.632951799834758243 ETH
closed realized PnL:    14.692951799834758243 ETH
open mark value:         0.367133091739369945 ETH
gross total PnL:        11.130084891574128188 ETH
```

The gross total is:

```text
confirmed sell proceeds + latest open value - confirmed buy cost
= 15.632951799834758243 + 0.367133091739369945 - 4.87
= 11.130084891574128188 ETH
```

Invariant checks:

```text
positions missing entry report:             0
confirmed buys without a position:          0
sell_confirmed without confirmed sell:      0
sell_failed without failed sell:            0
open positions without latest snapshot:     0
entry cost vs buy report mismatches:        0
exit proceeds vs sell report mismatches:    0
closed realized PnL mismatches:             0
open unrealized PnL mismatches:             0
state-aware ROI mismatches:                 0
```

## Concentration

```text
total PnL ETH:      11.130084891574128188
excluding top 1:     7.377229705848169078
excluding top 2:     5.057301262370881745
excluding top 5:     1.142209585439411740
excluding top 10:   -1.777050511574842793
```

The run is profitable on gross PnL, but the strategy result is highly concentrated. Excluding the top ten positions turns the run negative.

## Failure Buckets

```text
Sell transaction failed: TransferHelper: TRANSFER_FROM_FAILED   20
v4 universal router buy reverted                                 6
Buy transaction failed: UniswapV2: TRANSFER_FAILED               3
Buy transaction failed: TF                                       2
Universal Router V3 sell transaction failed: V3InvalidSwap       1
```

There are `21` failed sell reports across `10` positions. `8` positions remain in `sell_failed`; `2` later sold successfully. The `8` still-open failed exits are marked to zero value with `-0.01 ETH` unrealized PnL each.

## Accounting Interpretation

This run verifies gross recipient-side swap PnL.

The buy and sell fills come from EVM simulation at each replay block. Transfer taxes, honeypot behavior, max transaction limits, router reverts, and AMM price impact/slippage are reflected in the simulated token amount received on buys and denom amount received on sells.

Gas is not included in PnL. `execution_reports` stores `gas_used`, but it does not store base fee, effective gas price, priority fee, or gas cost. Confirmed reports used `75,469,494` gas units (`62,905,216` buy gas and `12,564,278` sell gas). Failed reports currently have no gas usage stored. Therefore the current `total_pnl_eth` is not net after gas.

To make net PnL verifiable, extend `ExecutionReport` and `position_snapshots` with gas price/cost fields, persist them from the simulation transaction fees, and add `gross_pnl_eth`, `gas_cost_eth`, and `net_pnl_eth` to the lab report.

## Repeatable Checks

Canonical lab report:

```bash
cargo run --release -p eth_alpha_lab --bin eth_alpha_lab -- strategy \
  --run-id hist-snipe-all-poolonly-15k-pnl-verify-20260513-205548 \
  --limit 20
```

Independent PnL rollup:

```bash
psql postgresql://postgres:postgres@localhost:5432/eth_db -Atc "
with latest as (
  select distinct on (run_id, position_id)
    run_id,
    position_id,
    nullif(current_value_eth,'')::numeric current_value_eth,
    nullif(realized_profit_eth,'')::numeric realized_profit_eth,
    nullif(unrealized_profit_eth,'')::numeric unrealized_profit_eth
  from alpha_trading.position_snapshots
  where run_id='hist-snipe-all-poolonly-15k-pnl-verify-20260513-205548'
  order by run_id, position_id, block_number desc nulls last, id desc
),
p as (
  select run_id, position_id, state, (payload->>'entry_cost_basis')::numeric cost
  from alpha_trading.positions
  where run_id='hist-snipe-all-poolonly-15k-pnl-verify-20260513-205548'
)
select
  count(*) positions,
  count(l.*) latest_snapshot_positions,
  sum(cost) entry_cost_eth,
  sum(l.current_value_eth) latest_current_value_eth,
  sum(l.realized_profit_eth) realized_pnl_eth,
  sum(l.unrealized_profit_eth) unrealized_pnl_eth,
  sum(l.realized_profit_eth + l.unrealized_profit_eth) total_pnl_eth
from p
left join latest l using (run_id, position_id);
"
```
