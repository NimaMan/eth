# Scripts

Scripts in this folder make the vault deployment repeatable. They are written
to be run from any working directory and resolve the ETH repo root from their
own path.

The scripts default to checks, local builds, fork rehearsal, calldata
generation, and Kartal dry-runs. The mainnet deployment script refuses to
broadcast unless `CONFIRM_DEPLOY=1` is set and all required environment
variables are present.

## Order

| Script | Purpose |
| --- | --- |
| `00_preflight.sh` | Check repo paths, tools, source files, and public config templates. |
| `01_build_and_hash.sh` | Run Foundry/Rust checks and write bytecode hash evidence. |
| `02_fork_rehearsal.sh` | Run mainnet fork gas tests at a fixed block. |
| `03_generate_calldata.sh` | Generate representative vault buy and emergency-sell calldata. |
| `04_kartal_dry_run.sh` | Submit a prepared direct-raw request through Kartal calibration. |
| `05_deploy.sh` | Deploy the vault to mainnet after explicit confirmation. |
| `06_verify_contract.sh` | Verify the deployed contract and constructor arguments. |
| `07_post_deploy_smoke.sh` | Read deployed immutable values and compare them to expected config. |
| `08_run_simulation_suite.sh` | Run deployed-vault functional and gas-threshold simulations. |

Run evidence belongs under `../runs/<run-id>/`.
