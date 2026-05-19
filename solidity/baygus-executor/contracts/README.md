# Contracts

This is the Foundry project for Uniswap V2 Trading Vault.

- `src/` contains production Solidity.
- `test/` contains deterministic tests and mocks.
- `foundry.toml` writes artifacts to `../out` so Rust can load them from a stable path.
- The vault has no owner/admin upgrade path. Owner, treasury, WETH, and router
  are constructor state, so changing them requires a new deployment.

Production deployments should stay minimal. Keep route search, pool choice, quoting, slippage math,
gas policy, and bribe sizing off-chain. Add Solidity only for behavior that must happen atomically
inside the transaction, and benchmark every added command against the direct router or pool path.

For Mode A, the atomic behavior is approving the exact sell amount and selling
inside one emergency-exit transaction. The buy path intentionally leaves no
router allowance behind.

Use:

```bash
forge fmt --check
forge build
forge test
```
