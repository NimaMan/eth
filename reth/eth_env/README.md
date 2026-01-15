# eth_env

`eth_env` provides on-chain simulation environments backed by a local Reth
snapshot. The goal is to keep all heavy lifting (feature engineering, reward
calculation, deterministic block replay) in Rust so that training stacks
(`pyreth`, notebooks, services) can focus on higher-level modelling.

## Current Scenario — ETH 15-minute Forecast

The first scenario targets Polymarket-style ETH prediction markets. Every step
produces an observation containing:

- Chainlink ETH/USD
- Mid-prices for ETH/USDC, ETH/USDT, ETH/DAI
- Relative spreads vs Chainlink (bps)
- Block number + timestamp metadata

The agent emits a price prediction, and once the next observation arrives the
env scores it via negative absolute error (future reward hooks allow binary
payoffs). This lets us plug in classical regressors, supervised learners, or
RL policies that reason about the same signal.

## Layout

```
src/
├── env_core/           # shared traits, action/observation types, reward helpers
├── data/               # feed adapters around eth_prices + feature transforms
├── scenarios/
│   └── eth15m          # concrete environment implementation
├── telemetry/          # tracing + metrics helpers
└── python/bindings.rs  # future PyO3 hooks consumed by pyreth
```

`examples/eth15m/` contains runnable snippets. `cargo run --example eth15m`
fetches the last 100 on-chain blocks, materialises their price snapshots, and
stores the output JSON under `examples/eth15m/data/`.

## Usage

```
export RETH_DATADIR=/home/nima/.local/share/reth/mainnet
cargo run -p eth_env --example eth15m
```

The example materialises historical market snapshots that can be consumed by
follow-on notebooks or the upcoming `pyreth` bindings. Once wired up, Python
agents can call into the same Rust code for reset/step loops.
