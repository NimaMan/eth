# Config

This folder holds public, non-secret deployment configuration templates for the
Uniswap V2 trading vault.

Use these files as deployment inputs and copy the resolved values into the
active run folder. Do not commit private keys, RPC tokens, bearer tokens, or
machine-local secret paths here.

## Files

| File | Purpose |
| --- | --- |
| `mainnet.toml` | Chain constants, source paths, constructor arguments, and signer identities. |
| `alpha-route-policy.mainnet.toml` | How alpha should target the deployed vault for buy and emergency-sell routes. |
| `gas-policy.mainnet.toml` | Priority-fee, value-cap, and gas-rank policy for the vault route. |
| `kartal-policy.mainnet.json` | Intended Kartal allowlist and request-policy shape for this deployed address. |

The resolved run inputs belong under `../runs/<run-id>/inputs.json`.
