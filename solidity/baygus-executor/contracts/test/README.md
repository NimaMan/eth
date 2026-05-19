# Tests

`BaygusTradingVault.t.sol` verifies Mode A behavior:

- buy stores bought tokens in the vault;
- buy leaves no sell-router allowance;
- emergency sell approves exact amount and clears allowance;
- fee-on-transfer buys record the net token amount;
- fee-on-transfer emergency sells clear allowance after router transfer;
- ETH proceeds go to treasury;
- non-owner calls fail;
- min-output checks revert;
- owner rescue functions work.

The main behavior tests use local mocks so CI and artifact rebuilds stay
deterministic. Fork-only gas coverage lives under `bench/` and no-ops unless a
mainnet fork is supplied.
