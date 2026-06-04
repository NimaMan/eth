# Uniswap V4 Trading Vault Source

This folder holds the candidate Uniswap V4 trading vault. It is separate from
`src/UniswapV2TradingVault.sol` because V4 uses a different execution
surface: Universal Router, Permit2, PoolManager, hooks, and pool-key policy.

Current state: candidate source with unit tests, fork gas tests, and
tx_simulator direct-vs-vault rehearsal evidence. Mainnet broadcast remains
blocked by the deployment folder until calldata generation, ETH tx executor
dry-run, and operator signoff are recorded.

Expected implementation files:

- `UniswapV4TradingVault.sol`: owner-only V4 vault with exact route inputs.
- `../interfaces/uniswap/v4/`: pinned external interfaces.
- `../libraries/v4/`: path encoding and route-policy helpers.

Do not reuse the V2 vault deployment folder for V4 evidence.
