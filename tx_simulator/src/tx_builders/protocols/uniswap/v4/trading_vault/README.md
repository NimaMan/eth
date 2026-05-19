# V4 Trading Vault Tx Builders

V4 vault calldata builders live here, separate from the direct Universal Router
builders in `../universal_router.rs`.

Scope:

- encode candidate V4 vault buy calldata;
- encode candidate V4 vault emergency-sell calldata;
- keep exact route and pool-key inputs explicit;
- avoid route discovery, quote selection, and gas-rank policy.

These builders target the current candidate ABI in
`solidity/baygus-executor/contracts/src/v4/UniswapV4TradingVault.sol`. Treat the
ABI as candidate-only until deployment signoff is complete.
