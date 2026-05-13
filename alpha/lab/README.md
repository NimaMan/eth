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
snapshots, open failed exits, and accounting anomalies.

The lab reads `ALPHA_DATABASE_URL` from `blockchains/eth/config.env` by default.

```bash
eth_alpha_lab strategy \
  --run-id hist-poolonly-mtm-v4-20260512-1021
```

Multiple runs can be reported together:

```bash
eth_alpha_lab strategy \
  --run-id hist-poolonly-mtm-paper-20260512-1021 \
  --run-id hist-poolonly-mtm-v4-20260512-1021
```

## Token Lab

Single position/token checks for entry report consistency, observation joins,
latest valuation, and first/last mark-to-market trajectory.

```bash
eth_alpha_lab position \
  --run-id hist-poolonly-mtm-v4-20260512-1021 \
  --token 0x12a77658112Cf42914cB614D13653ed5852DA1e5
```

If more than one position matches a token, use `--position-id`.

Both commands support `--json` for automation.

## Current Strategy Issue Ledger

Scope: latest clean 15k historical baseline
`hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`, range
`25,065,694..25,080,693`, source replay
`snipe-all-v1-chain-sim-live-v4`.

Run facts from Strategy Lab:

- 449 total positions, 353 open positions, 19 failed positions.
- 11 buy-failed positions and 8 sell-failed positions.
- 554 execution reports: 523 confirmed, 31 failed.
- Snapshots across 438 positions; zero open positions without a snapshot.
- Total PnL `+9.693440469949935980 ETH`.
- PnL excluding top 5 positions is `-0.219822746621256746 ETH`.
- PnL excluding top 10 positions is `-2.606516897707689237 ETH`.
- Earlier May 13 reruns confirmed the same remaining failure buckets; the
  balance-overlay run cleared the 3 sell-only
  `unsupported_balance_storage_layout` infrastructure failures before the
  recipient-net accounting rerun became the current baseline.
- Sell proceeds now prefer the strategy recipient's net ETH/WETH balance
  increase before falling back to gross denomination-token pool output. The
  gross-output baseline fixed zero-fill sells but overcounted auto-swap WETH
  paid to third parties, so the old
  `hist-snipe-all-poolonly-15k-weth-only-20260513-134623` result is
  superseded by the recipient-net run.
- `pancakeswap-v2` protocol strings are now parsed as Pancake V2 instead of
  unsupported. The affected Pancake entry buys successfully, then fails every
  sell probe down to 1% with no token-to-pool sell logs through current head.
- V3 Universal Router sell calldata now matches the observed router variant.
  The remaining V3 failed exit is no longer a route gap: it succeeds at the
  observed sell block, but at the strategy max-hold exit block the pool is
  drained and the simulator returns `V3InvalidSwap`.
- Stable-denom entries are excluded from the executable baseline until we add
  quote conversion and ETH-equivalent PnL. This removed the two USDC/USDT
  empty-router buy failures and the one confirmed USDT-denom position from the
  previous run; the current run has only WETH-denominated confirmed positions.
- Source replay rows for this 15k range have `payload.pool.runtime_state`,
  but only with `last_sync_block` / `last_update_block`; none preserve
  `runtime_state.can_buy` or `runtime_state.can_sell`.
- Strategy Lab now prints `Buy Failed Entries` and `Open Failed Exits` tables
  so failed entries and retryable failed exits are visible without ad hoc SQL.

Keep this ledger short and move fixed rows into tests, focused investigation
folders, or commit history.

| Priority | Status | Issue | Evidence | Fix / Next Check |
| --- | --- | --- | --- | --- |
| P0 | confirmed | Failed sells remain open, retryable exposure | The run has 20 failed sell reports but 8 sell-failed positions. BCB2 failed at block `25,074,899` and then sold successfully at `25,074,918`; the 8 still-open failed exits all have zero-value snapshots and remain `sell_failed`. | Keep this lifecycle behavior; next work is policy, not accounting |
| P1 | investigating | V4 observed-flow-only entries polluted strategy eligibility | 6 V4 Universal Router buy failures were selected from observed third-party flow even though same-route buy probes failed at entry block and `block-1` for `0.01`, `0.001`, and `0.0001 ETH`; all six stored observations say top-level `can_buy=true` / `can_sell=true`, but lack `runtime_state.can_buy/can_sell` | Rebuild the source observations from a token-server response that includes runtime trading flags, then rerun the 15k backtest; these should become skips, not failed buys |
| P1 | confirmed | Sell exits are root-cause bucketed | Current failed-position split: 7 V2 `TRANSFER_FROM_FAILED` exits and 1 V3 `V3InvalidSwap`. DF1A/BE74/2834 are address-specific: observed chain sellers simulate successfully with gross proceeds, while the strategy seller fails at every tested size. 11AE sells in 5%/2%/1% chunks. Pancake V2 fails down to 1% and has no token-to-pool sell logs through current head. BCB2 confirms after a later retry. The V3 case is drained zero-liquidity exposure at exit block. | Choose strategy rules for retry cadence, chunked exits, and address-specific/no-observed-sell exposure |
| P1 | confirmed | Fixed entry size fails on thin WETH pools | The 3 V2 `TRANSFER_FAILED` and 2 V3 `TF` WETH buy failures fail at `0.01 ETH` but representative same-route probes succeed at `0.001 ETH` on both `block-1` and the entry block | Decide whether this strategy remains fixed-size, adds adaptive entry sizing, or skips pools that cannot support the target size |
| P2 | investigating | PnL is too concentrated for strategy conclusions | Total PnL is positive, but excluding the top five positions is `-0.219822746621256746 ETH`; excluding the top ten is `-2.606516897707689237 ETH` | Add concentration-aware reporting to the strategy table and use it as a baseline gate before treating `Snipe All v1` as profitable |
