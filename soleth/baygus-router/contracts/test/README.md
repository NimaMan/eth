# Tests

The test suite is split by router surface:

- `BaygusRouterV4.t.sol` verifies v4 single-hop, multi-hop net settlement, hooks, and slippage.
- `BaygusRouterCommands.t.sol` verifies command execution for token pulls, sweeps, and V2 swaps.

The tests use local mocks instead of mainnet forks so CI and artifact rebuilds stay deterministic.
