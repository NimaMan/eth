# Uniswap V4 Trading Vault Source

This folder is reserved for a future Uniswap V4 trading vault. It is separate
from `src/UniswapV2TradingVault.sol` because V4 uses a different execution
surface: Universal Router, Permit2, PoolManager, hooks, and pool-key policy.

Current state: assessment scaffold only. The checked-in Solidity file is
deliberately undeployable until the route model, simulator coverage, gas
benchmarks, and Kartal policy are reviewed.

Expected implementation files:

- `UniswapV4TradingVault.sol`: owner-only V4 vault with exact route inputs.
- `../interfaces/uniswap/v4/`: pinned external interfaces.
- `../libraries/v4/`: path encoding and route-policy helpers.

Do not reuse the V2 vault deployment folder for V4 evidence.
