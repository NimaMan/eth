# V4 Vault Gate Run

This run closes the local pre-deploy gates for the Uniswap V4 trading vault
candidate without broadcasting a mainnet deployment.

## Evidence

- `artifact-hashes.json`: bytecode, deployed bytecode, ABI, artifact, and source hashes.
- `fork-rehearsal.json`: pinned fork gas test plus tx_simulator direct-vs-vault comparison.
- `constructor-args.json`: owner, treasury, Universal Router, Permit2, hooks policy, and deployer.
- `calldata.json`: constructor init code, deploy gas estimate, nonce-bound predicted address, and representative buy/sell calldata.
- `kartal-dry-run.json`: current Kartal dry-run evidence.
- `deploy-fail-closed.txt`: deploy script refusal without `CONFIRM_DEPLOY=1`.

## Current Result

The candidate deployer is `0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27`.
At nonce `55`, the predicted CREATE address is
`0x352879E8BA5B02e0A9839ec86AB4A94238e8dbc9`.

That prediction is valid only while the deployer nonce remains `55`.

Kartal is healthy and in `dry_run` mode, but the current live policy still
allowlists the deployed V2 vault target/selectors. The V4 buy and sell dry-run
requests were intentionally rejected by policy. Do not switch Kartal to the V4
target until the V4 contract is actually deployed and the operator signs off.
