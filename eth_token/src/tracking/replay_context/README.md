# Replay Context

`replay_context` owns block-local setup replay policy for token and pool simulations.

The token manager applies transactions in block order, while the pool simulator starts from the previous block and only sees the prior transactions it is handed. This module records earlier same-block transactions that must be replayed before synthetic buy/sell checks.

Responsibilities:

- Keep token-control priors keyed by token address, not sender address.
- Keep pool setup priors keyed by pool address.
- Return only priors observed earlier in the current block.
- Leave execution semantics to `tx_processor`; this module only decides which processed transactions are relevant setup.

This is intentionally under `manager/` because replay context is block-application policy, not pool math and not low-level transaction execution.
