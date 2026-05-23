# Risk Atlas Modeling

Baseline modeling workspace for Risk Atlas runs.

Primary short-horizon run:

- `backdoor_horizon_oot_v1`
- Previous baseline: `risk-atlas-uniswap-v2-20k-short-horizon-20260521`

Current OOT experiment:

- Name: `backdoor_horizon_oot_v1`
- Train: `24974082..25024081` (50K blocks)
- Gap: `25024082..25124081` (100K blocks)
- Test: `25124082..25144081` (20K blocks, includes ULIQ)
- Overlap guard: drop any test `(token_address, pool_address)` also present in train

Modeling targets:

- `scam_within_N_chain_block_delta`, with `N = 1, 2, 3, 4, 10`: whether a
  pool scams within the next raw chain blocks from an observation.

Rules:

- Features must be as-of the observation block.
- Future scam labels, future LP approvals, future mempool arrivals, and future
  removals are labels or evaluation evidence, not model inputs.
- Generated datasets, models, and reports live under `artifacts/<run_id>/`.

Suggested first command flow:

```bash
python scripts/build_dataset.py --config configs/uniswap_v2_short_horizon_lp_approval.toml
python scripts/train_baselines.py --config configs/uniswap_v2_short_horizon_lp_approval.toml
python scripts/evaluate_run.py --config configs/uniswap_v2_short_horizon_lp_approval.toml
```

OOT command flow:

```bash
cargo build --release --bin risk_atlas_headless
RUST_LOG=error target/release/risk_atlas_headless --start 24974082 --end 25024081 --run-id risk-atlas-backdoor-horizon-oot-v1-train-50k-20260521 --progress-every-secs 60
RUST_LOG=error target/release/risk_atlas_headless --start 25124082 --end 25144081 --run-id risk-atlas-backdoor-horizon-oot-v1-test-20k-20260521 --progress-every-secs 60
python scripts/build_oot_dataset.py --config configs/experiments/backdoor_horizon_oot_v1.toml
python scripts/train_baselines.py --config configs/backdoor_horizon_oot_v1.toml
python scripts/evaluate_run.py --config configs/backdoor_horizon_oot_v1.toml
python scripts/write_iteration_notes.py --config configs/experiments/backdoor_horizon_oot_v1.toml
```

Large OOT imports use compact Risk Atlas observation JSON. The compact payload
keeps the observation block, active observation index, active reasons, and
protocol, while omitting full per-observation feature JSON that is already
represented in the modeling dataset.
