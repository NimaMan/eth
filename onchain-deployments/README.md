# On-chain Deployments

This folder is the Ethereum on-chain deployment ledger. It is for contracts
that are deployed to Ethereum mainnet or rehearsed exactly as mainnet
deployments.

It is not for service deployment, Docker, systemd, or strategy runtime config.
It is also not the Solidity source tree. Contract source, tests, and Foundry
artifacts stay in `../solidity/`; this folder records the chain-specific path
from reviewed source to deployed address.

## Folder Contract

Each deployed on-chain surface gets one child folder:

```text
onchain-deployments/
  <contract-or-system-name>/
    README.md
    config/
    scripts/
    audit/
    runs/
```

The child folder owns the runbook, deployment inputs, policy files, evidence
paths, and signoff. A deployment is not considered reproducible unless a run
folder records:

- exact git revision and dirty-state summary;
- constructor arguments and deployment signer address;
- contract artifact hash and bytecode hash;
- Foundry and Rust test outputs;
- fork rehearsal block and gas snapshot;
- generated buy/sell calldata examples;
- Kartal dry-run or policy-reject evidence;
- deployment transaction hash and receipt;
- verification result;
- post-deploy smoke result;
- final signoff.

## Safety Rules

- Do not commit private keys, RPC credentials, bearer tokens, or secret env
  files.
- Keep configs as templates unless the value is safe public chain metadata.
- Keep one folder-level `README.md` per folder; put detailed process notes in
  the relevant child README.
- Deployment scripts must default to dry-run or checks. Any mainnet broadcast
  must require explicit confirmation environment variables.

## Current Surfaces

| Folder | Source | Status |
| --- | --- | --- |
| `uniswap-v2-trading-vault/` | `../solidity/baygus-executor/contracts/src/UniswapV2TradingVault.sol` | Deployment runbook and evidence templates. |
