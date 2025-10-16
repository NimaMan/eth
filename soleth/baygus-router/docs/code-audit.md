# Baygus Router Code Audit

- **Version:** v0.4 (multi-hop routing)
- **Last updated:** 2025-10-16

## Scope
- Solidity sources under `contracts/src/`
- Foundry test suite under `contracts/test/`
- Tooling + documentation within the Baygus Router repository

## Current Capabilities
1. **Single and multi-hop Uniswap v4 swaps**
   - Executes `lock → swap → settle` for a provided `PoolKey`.
   - Supports sequential hops with per-hop hooks and aggregated slippage checks.
   - Handles ETH/WETH and arbitrary ERC-20 currencies.
2. **Hook adapter pipeline**
   - Optional external adapter invoked via `beforeSwap` / `afterSwap`.
   - Propagates caller-supplied hook data.
3. **Per-leg slippage assertions**
   - Enforces minimum deltas on both pool currencies.
4. **Foundry test coverage**
   - Happy-path swaps (buy/sell).
   - Hook integration & slippage revert scenarios.
5. **Offline replay tooling**
   - `tools/replay_moonstr_swap.py` decodes a real Uniswap v4 trade for reference.

## Known Gaps / Open Questions
| Area | Status | Notes |
|------|--------|-------|
| Multi-hop routing | ✅ v0.4 | Sequential hops supported; follow-up gas profiling and hook threat model still needed. |
| Rust integration | Missing | Rust simulators still call legacy components; adapter layer from Rust → router not implemented. |
| Gas / reentrancy guards | Partial | Router is reentrancy-safe for single call, but adapter contracts are unconstrained. Needs threat model + guardrails. |
| Token approval ergonomics | Barebones | Caller must pre-approve max amount. Consider permit / pull allowances for better UX. |
| Production readiness | Not started | No deployment scripts, on-chain tests, audit, or runtime monitoring hooks. |

## Recommendations
1. **v0.5** – Integrate with the Rust simulator stack (Baygus agent) so buy/approve/sell simulations invoke this router directly. Acceptance should be a combined Rust + Foundry test.
2. **v0.6** – Production hardening:
   - Expand fuzz/property tests.
   - Provide deployment scripts + config management.
   - Run static analysis (slither, mythril) & engage audit review.
   - Document on-chain monitoring / emergency procedures.
3. **Documentation** – Maintain this audit document as milestones land; note resolved items and new findings.

## Outstanding Questions
- Do we require adapter-specific permissions or allow any address? (Current: unrestricted.)
- Should we bake in a native → WETH auto-wrap step, or expect callers to wrap explicitly?
- How do we represent multi-hop routes in calldata (array of `PoolKey`, per-hop params, etc.)?
- What is the minimum quality bar for mainnet deployment (audit vendor, coverage thresholds)?
