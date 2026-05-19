# Route Fixtures

Route fixtures for candidate V4 paths belong here.

Each fixture should include:

- token in/out;
- pool key fields;
- hook address and hook policy;
- Universal Router command bytes;
- Permit2 allowance requirements;
- simulation block;
- expected min output and slippage policy.

## Current Fixtures

`eth-usdc-500-no-hook.json` pins the mainnet native ETH/USDC 0.05% no-hook V4
pool key used by the direct Universal Router baseline. It does not imply that
the future vault is no-hook-only; hook routes must be added as separate named
fixtures with their hook address and policy made explicit.

Run the direct baseline for this fixture with:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -q -p tx_simulator --example uniswap_v4_trading_vault_direct_baseline
```
