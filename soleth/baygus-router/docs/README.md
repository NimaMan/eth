# Documentation

Use this folder to capture the evolving design of the Baygus Router:

- **Architecture notes** – module boundaries, call flows, settlement semantics.
- **Threat models** – assumptions about hook behaviour, reentrancy guards, and allowance handling.
- **Integration guides** – how off-chain services (simulators, executors) interact with the router.
- **Testing strategy** – coverage targets, fuzzing plans, and cross-protocol regression cases.

When drafting a document, prefer Markdown and link back to source material in `references/` where
relevant. Keep filenames descriptive (e.g. `uniswap-v4-lock-flow.md`, `router-state-machine.md`)
so they are easy to navigate.

---

## Version Roadmap (v0.1 → v0.5)

| Version | Focus                                                                                                       | Acceptance Test                                                                                         |
|---------|-------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| v0.1    | Replay a production Uniswap v4 swap offline to understand PoolManager cash flows.                           | `python sol/baygus-router/tools/replay_moonstr_swap.py`                                                  |
| v0.2    | Implement a minimal `BaygusRouter` (single pool, no hooks) handling lock → swap → settle for ERC20/WETH.    | `forge test --root sol/baygus-router/contracts --match-path test/BaygusRouter.t.sol`                     |
| v0.3    | Introduce hook-aware adapters and safety rails (hook data propagation, sanity checks).                      | `forge test --root sol/baygus-router/contracts --match-path test/BaygusRouter.t.sol`                     |
| v0.4    | Support multi-hop / multi-pool swap paths within a single lock.                                             | Scenario test executing TokenA → WETH → TokenB path with deterministic balances.                         |
| v0.5    | Generalise the router core so additional venues (e.g. Uniswap v2, Balancer) plug in via adapters.           | Single CLI/forge task executing both a v4 hook swap and a v2 swap through the unified router entrypoint. |

We revisit this table after every milestone to delete unnecessary requirements and adjust scope
as needed.

---

## Milestone Audits

### v0.1 – Uniswap v4 Swap Replay

- **Objective:** Understand the execution trace of transaction
  `0x6910a3065065c447ee2791126ddfce55e65ace265e30c05d52e386b37e7233c9` without deploying new
  contracts.
- **Command:** `python sol/baygus-router/tools/replay_moonstr_swap.py`
- **Outcome:** Script prints the PoolManager ETH/token flows (trader proceeds plus two fee
  recipients), confirming that the bespoke router at `0x00000000000044a361Ae3cAc094c9D1b14Eece97`
  acquires the lock and redistributes funds. Missing bytecode for this router explains why the
  generic simulator cannot yet reproduce the trade.
- **Follow-up:** v0.2 introduces our own minimal router to unblock simulations.

### v0.3 – Hook Adapter & Safety Rails

- **Objective:** Enable optional hook adapters and enforce slippage guards so the router can safely
  interact with hook-heavy pools.
- **Command:** `forge test --root sol/baygus-router/contracts --match-path test/BaygusRouter.t.sol`
- **Outcome:** The test suite now covers hook callbacks (via `MockHookAdapter`) and slippage failure
  scenarios. Router exposes `SwapExactInputSingleParams` struct with hook adapter address and
  per-currency minimum deltas. Safety checks revert with a dedicated `SlippageCheckFailed` error.
- **Follow-up:** Design adapter abstraction for multi-hop routes (v0.4) and extend documentation
  with hook-specific threat modelling.
