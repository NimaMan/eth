# Uniswap V4 Trading Vault Gates

This document tracks the gates that must be closed before any V4 vault
deployment.

| Gate | Status | Requirement |
| --- | --- | --- |
| V4 route model | blocked | Route fixtures must define token path, pool key, fee tier, tick spacing, hooks, and expected command bytes. |
| Simulator coverage | blocked | tx_simulator must compare direct Universal Router execution with candidate vault execution. |
| Solidity implementation | blocked | Contract must be owner-only, exact-route, hook-policy aware, and non-generic. |
| Permit2 lifecycle | blocked | Allowance behavior must be explicit and benchmarked. |
| Gas review | blocked | Fork gas report must compare direct and vault paths. |
| Kartal policy | blocked | Target and selector allowlist must be specific to V4 vault selectors. |
| Bribe policy | blocked | Only public EIP-1559 priority fee is allowed until separate bundle support exists. |
| Deployment | blocked | `scripts/05_deploy.sh` must fail closed until all prior gates are signed off. |
