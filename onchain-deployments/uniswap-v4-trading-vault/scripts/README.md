# V4 Vault Scripts

These scripts implement the V4 vault deployment gate flow. Mainnet broadcast
still fails closed unless `CONFIRM_DEPLOY=1` is set explicitly.

| Script | Purpose |
| --- | --- |
| `00_preflight.sh` | Check source, config, interfaces, and route fixtures. |
| `01_build_and_hash.sh` | Build and record bytecode, ABI, and source hashes. |
| `02_fork_rehearsal.sh` | Run V4 fork tests and gas benchmarks. |
| `03_generate_calldata.sh` | Generate representative V4 vault calldata. |
| `04_kartal_dry_run.sh` | Run candidate direct-raw requests through Kartal. |
| `05_deploy.sh` | Mainnet deploy, blocked until all gates pass. |
| `06_verify_contract.sh` | Verify deployed source and constructor args. |
| `07_post_deploy_smoke.sh` | Read deployed immutable values and smoke-test selectors. |

Typical pre-deploy run:

```bash
RUN_DIR=onchain-deployments/uniswap-v4-trading-vault/runs/<run-id>
./onchain-deployments/uniswap-v4-trading-vault/scripts/00_preflight.sh
RUN_DIR="$RUN_DIR" ./onchain-deployments/uniswap-v4-trading-vault/scripts/01_build_and_hash.sh
RUN_DIR="$RUN_DIR" ./onchain-deployments/uniswap-v4-trading-vault/scripts/02_fork_rehearsal.sh
RUN_DIR="$RUN_DIR" ./onchain-deployments/uniswap-v4-trading-vault/scripts/03_generate_calldata.sh
RUN_DIR="$RUN_DIR" EXPECT=policy-rejected ./onchain-deployments/uniswap-v4-trading-vault/scripts/04_kartal_dry_run.sh
```

The final accepted Kartal dry-run must happen after deployment, with
`VAULT_ADDRESS` set to the deployed contract and both Kartal and signer policies
updated to the V4 target and selectors.
