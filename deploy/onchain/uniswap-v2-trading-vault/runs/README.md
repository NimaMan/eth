# Runs

Each child folder is one deployment rehearsal or deployment attempt. A run
folder should be immutable once it has been signed off.

Use `TEMPLATE/` as the shape for a new run folder. Name real runs with a date,
network, and version, for example:

```text
2026-05-19-mainnet-v1/
```

## Required Run Evidence

| File | Purpose |
| --- | --- |
| `inputs.json` | Resolved deployment, route, gas, and Kartal inputs. |
| `git-revision.txt` | Git commit used for the run. |
| `git-status.txt` | Dirty-state summary at run time. |
| `forge-fmt.txt` | Foundry format check output. |
| `forge-build.txt` | Foundry build output. |
| `forge-test.txt` | Foundry test output. |
| `cargo-test-tx-simulator.txt` | Rust builder test output. |
| `cargo-test-eth-live-trading.txt` | Live tx-prep test output. |
| `artifact-hashes.json` | Bytecode, deployed bytecode, and ABI hashes. |
| `constructor-args.json` | Constructor arguments and signer env name. |
| `fork-rehearsal.json` | Fork block and rehearsal status. |
| `gas-snapshot.txt` | Fork gas snapshot. |
| `calldata.json` | Representative buy and emergency-sell calldata. |
| `kartal-dry-run.json` | Kartal calibration or policy evidence. |
| `deploy-output.json` | Mainnet deployment transaction output. |
| `verification-output.json` | Contract verification result. |
| `post-deploy-smoke.json` | Immutable readback checks. |
| `signoff.json` | Final operator and reviewer approval. |

Deployed-vault functional and performance simulations live under
`../simulations/reports/<run-id>/`, because they can be rerun against current
local Reth state after deployment.
