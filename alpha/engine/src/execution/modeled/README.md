# Modeled Execution Adapter

Worst-case fill adapter that produces honest PnL using pool snapshot math.

## Behavior

- Keeps a live `HashMap<PoolAddress, PoolSnapshot>` that the trader binary
  updates on every pool poll.
- **Buy**: the full ETH amount is spent. Gas is deducted from the report.
- **Sell**: the expected denom output is reduced by:
  - Configured slippage (bps)
  - Gas cost (wei)
- **Rejection**: orders are rejected if the pool is marked scam or below
  `min_denom_reserve`.

## Configuration

`ModeledExecutionConfig` fields:

| Field | Default | Description |
|-------|---------|-------------|
| `slippage_bps` | `500` | Slippage applied to sells (5%) |
| `gas_cost_wei` | `150_000` | Fixed gas cost deducted per fill |
| `max_price_impact_bps` | `2000` | Max allowed price impact before rejection |
| `min_denom_reserve` | `0.5` ETH | Minimum liquidity floor |

## Use Cases

- Live trader `--mode=modeled` for honest paper PnL.
- Backtest alignment: the backtest `SimulatedExecutionAdapter` should use the
  same fill rules so backtest and live paper are comparable.

## Future Work

- Add sell-tax modeling once `PoolSnapshot` exposes `sell_tax_bps`.
- Add dynamic gas estimation based on pool protocol instead of fixed cost.
- Add price-impact math using reserve ratios rather than just slippage.
