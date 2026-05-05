# Contracts

This is the Foundry project for Baygus Router.

- `src/` contains production Solidity.
- `test/` contains deterministic tests and mocks.
- `foundry.toml` writes artifacts to `../out` so Rust can load them from a stable path.
- The router has no owner/admin upgrade path. Adapter addresses and PoolManager are constructor
  state, so changing them requires a new deployment.

Use:

```bash
forge fmt --check
forge build
forge test
```
