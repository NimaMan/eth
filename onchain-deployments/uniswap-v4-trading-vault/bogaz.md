# Uniswap V4 Trading Vault Gates

This document tracks the gates that must be closed before any V4 vault
deployment.

| Gate | Status | Requirement |
| --- | --- | --- |
| Assessment criteria | evidence-ready | `audit/assessment-criteria.md` defines shared V2-derived gates and V4-specific gates. |
| V4 route model | evidence-ready | ETH/USDC 0.05% no-hook fixture defines token path, pool key, fee tier, tick spacing, hooks, and expected command bytes. |
| Simulator coverage | evidence-ready | tx_simulator compares direct Universal Router execution with candidate vault execution at pinned block 25131251. |
| Solidity implementation | evidence-ready | Candidate is owner-only, exact-route, hook-policy aware, and non-generic. |
| Permit2 lifecycle | evidence-ready | Sell path grants exact ERC20 and Permit2 allowances, clears both after success, and reverts state on failure. |
| Gas review | evidence-ready | Latest fork report: vault route 354,887 gas vs direct full route 382,953 gas; deploy 1,228,234 gas. |
| Deploy calldata | evidence-ready | `runs/20260519-v4-gates-202032Z/calldata.json` records constructor args, init code hash, gas estimate, and representative buy/sell calldata. |
| Kartal policy | partial | Candidate policy has from address and buy/sell selectors; current live Kartal rejects V4 target/selectors until a deployed V4 target is allowlisted. |
| Bribe policy | blocked | Only public EIP-1559 priority fee is allowed until separate bundle support exists. |
| Deployment | fail-closed | `scripts/05_deploy.sh` requires `CONFIRM_DEPLOY=1`; final mainnet broadcast still needs operator signoff. |
