# Alpha11 Hold15 Live Deploy Review

This folder is the deployment review workspace for:

- result set: `live-alpha11-live-univ2-lp30-pool-update-block-hold-sweep-chain-sim-20260521-105807`
- strategy: `alpha11-live-univ2-lp30-pool-update-block-hold15`
- source page: `http://127.0.0.1:40019/eth/backtest/live/live-alpha11-live-univ2-lp30-pool-update-block-hold-sweep-chain-sim-20260521-105807/strategies/alpha11-live-univ2-lp30-pool-update-block-hold15/`

The goal is to convert the live chain-sim backtest into deployable live-trading
requirements. In particular, this review checks every position and every
submitted order under the current backtest assumption:

```text
submit in block N -> simulated fill as the last relevant transaction in block N+1
```

For real deployment, the missing piece is the EIP-1559 priority fee. The
generator samples the exact mined confirmation block for each simulated
submission and records the priority fee needed to rank near the tail, top 50,
top 25, top 10, and top 5 of that block.

## Regenerate

```bash
cd /home/nima/code/crypto/blockchains/eth
python3 alpha/lab/reports/alpha11_live_deploy_20260521_hold15/analyze.py
```

Environment overrides:

- `ASENA_URL`, default `http://127.0.0.1:40019`
- `ETH_RPC_URL`, default `http://127.0.0.1:8545`

Generated files land in `artifacts/`:

- `summary.md`: run-level findings and bribe distribution.
- `position_reviews.md`: one section per current trade/position.
- `positions.csv`: compact position metrics.
- `gas_rank_review.csv`: one row per submitted buy/sell order.
- `summary.json`: machine-readable aggregate used by the Markdown reports.
