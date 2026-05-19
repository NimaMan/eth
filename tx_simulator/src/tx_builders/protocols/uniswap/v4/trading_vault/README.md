# V4 Trading Vault Tx Builders

Future V4 vault calldata builders belong here, separate from the direct
Universal Router builders in `../universal_router.rs`.

Planned scope:

- encode candidate V4 vault buy calldata;
- encode candidate V4 vault emergency-sell calldata;
- keep exact route and pool-key inputs explicit;
- avoid route discovery, quote selection, and gas-rank policy.

Do not promote a Rust builder until the Solidity ABI is final.
