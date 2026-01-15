eth_price_leverage (Python)

Lightweight Python package to train and evaluate RL agents on the Rust-backed stablecoin trading environment exposed via `pyreth`.

Structure
- envs: Pythonic wrappers around `pyreth` environments
- actions: Helpers to construct valid actions (routes, tokens, sizes)
- policies: Stateless or stateful action selection logic (random/heuristic)
- agents: Glue to run policies against envs and collect trajectories
- examples: Small runnable examples
- config: Typed configs for env/training

Quick start
1) Ensure `pyreth` is installed and your local Reth DB is available.
2) Export `RETH_DATADIR`, `BAYGUS_TEST_EOA`, optionally `START_BLOCK`.
3) Run: `python py/eth_price_leverage/examples/run_basic_agent.py`

Notes
- Rewards are computed inside the Rust env in USD terms (Chainlink ETH/USD), default timing is Next-Block.
- Portfolio balances are U256 in Rust; this wrapper exposes them as Python ints for convenience.

