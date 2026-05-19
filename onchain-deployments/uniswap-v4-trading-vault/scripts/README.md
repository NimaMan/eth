# V4 Vault Scripts

These scripts are placeholders for a future V4 vault deployment flow. They fail
closed by default. Replace each script only when the corresponding checklist
item has evidence.

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
