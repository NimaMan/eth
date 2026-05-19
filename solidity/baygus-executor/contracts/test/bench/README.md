# Bench Tests

Foundry gas tests for Mode A.

- `UniswapV2TradingVaultGas.t.sol` uses local mocks for deterministic regression
  snapshots, including a fee-on-transfer token mock.
- `UniswapV2TradingVaultForkGas.t.sol` uses mainnet Uniswap V2 fixtures for USDC
  and RFI when the suite runs with a fork URL.
- `V2GasBenchBase.sol` holds shared helper calls so each measured path stays
  easy to compare.
- `v4/UniswapV4TradingVaultGas.t.sol` compares the candidate V4 vault against a
  direct Universal Router path with the same ETH/USDC 0.05% no-hook route shape
  used by `onchain-deployments/uniswap-v4-trading-vault/simulations/route-fixtures/eth-usdc-500-no-hook.json`.
- `v4/UniswapV4TradingVaultForkGas.t.sol` runs the same comparison against the
  real mainnet Universal Router, Permit2, and USDC contracts when a fork URL is
  supplied.

Regenerate committed snapshots from `../../benchmarks/README.md`.

Pinned V4 fork check:

```bash
forge test \
  --match-contract UniswapV4TradingVaultForkGasTest \
  --fork-url http://127.0.0.1:8545 \
  --fork-block-number 25131251 \
  --gas-report -vv
```
