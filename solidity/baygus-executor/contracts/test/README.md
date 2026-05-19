# Tests

`BaygusTradingVault.t.sol` verifies Mode A behavior:

- buy stores bought tokens in the vault;
- buy leaves no sell-router allowance;
- emergency sell approves exact amount and clears allowance;
- ETH proceeds go to treasury;
- non-owner calls fail;
- min-output checks revert;
- owner rescue functions work.

The tests use local mocks instead of mainnet forks so CI and artifact rebuilds
stay deterministic.
