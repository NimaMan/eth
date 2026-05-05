# Contracts

This is the Foundry project for Baygus Executor.

- `src/` contains production Solidity.
- `test/` contains deterministic tests and mocks.
- `foundry.toml` writes artifacts to `../out` so Rust can load them from a stable path.
- The router has no owner/admin upgrade path. Adapter addresses and PoolManager are constructor
  state, so changing them requires a new deployment.

Production deployments should stay minimal. Keep route search, pool choice, quoting, slippage math,
gas policy, and bribe sizing off-chain. Add Solidity only for behavior that must happen atomically
inside the transaction, and benchmark every added command against the direct router or pool path.

Use:

```bash
forge fmt --check
forge build
forge test
```
