# Bench Tests

Foundry gas tests for Mode A.

- `UniswapV2TradingVaultGas.t.sol` uses local mocks for deterministic regression
  snapshots, including a fee-on-transfer token mock.
- `UniswapV2TradingVaultForkGas.t.sol` uses mainnet Uniswap V2 fixtures for USDC
  and RFI when the suite runs with a fork URL.
- `V2GasBenchBase.sol` holds shared helper calls so each measured path stays
  easy to compare.

Regenerate committed snapshots from `../../benchmarks/README.md`.
