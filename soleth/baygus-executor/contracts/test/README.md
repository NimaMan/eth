# Tests

The test suite is split by router surface:

- `BaygusExecutorV4.t.sol` verifies v4 single-hop, multi-hop net settlement, hooks, and slippage.
- `BaygusExecutorCommands.t.sol` verifies command execution for token pulls, sweeps, V2 swaps,
  malformed commands, adapter-missing paths, coinbase tips, and native-transfer reentrancy.

The tests use local mocks instead of mainnet forks so CI and artifact rebuilds stay deterministic.
