# V4 Vault Libraries

V4-only helper libraries belong here. Keep route encoding, pool-key validation,
and hook policy separate from V2 helpers.

Planned files:

- `V4PathEncoding.sol`
- `V4VaultPolicy.sol`

These helpers must stay deterministic and calldata-focused. Route discovery,
quote selection, bribe selection, and gas-rank decisions stay off-chain.
