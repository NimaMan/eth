# Baygus Router

## Vision

Create a reusable, well-audited router layer that the Baygus execution agent can use to route swaps
across modern DEX architectures—even when the underlying protocol expects integrators to provide
their own `lock → swap → settle` orchestration. We start with **Uniswap v4**, where every
production trade today routes through bespoke contracts, and gradually generalise the framework so
it can support other venues (v2/v3 style factories, hook-heavy pools, hybrid AMMs, etc.).

**Primary objectives**

1. Deliver deterministic, simulator-friendly routing for Baygus’ Rust tooling.
2. Extend the router into a production-ready contract that can execute on mainnet safely.
3. Keep the architecture adapter-driven so new venues/slippage policies can slot in without
   rewriting the core.

## Gas Optimization Principles

As a high-frequency execution component, the Baygus Router adheres to strict gas optimization standards. All contributors should apply the following principles when modifying the codebase:

1.  **Transient Storage (EIP-1153):**
    *   **Mandate:** Use `tstore`/`tload` for all state that does not need to persist beyond the transaction (e.g., reentrancy guards, native ETH buffers).
    *   **Impact:** Saves ~20,000+ gas per transaction by avoiding expensive `SSTORE` operations.

2.  **Data Location & Types:**
    *   **Calldata:** Prefer `calldata` over `memory` for read-only array arguments to avoid copying costs.
    *   **Custom Errors:** Use `error Name()` instead of `require(cond, "String")`. Strings bloat bytecode and execution cost.
    *   **Storage Packing:** Order storage variables to pack into 32-byte words (e.g., `uint128` next to `uint128`) to minimize slot usage.

3.  **Execution Logic:**
    *   **Cache Reads:** Read storage variables into stack variables once if accessed multiple times.
    *   **Unchecked Arithmetic:** Use `unchecked { ... }` for incrementing loop counters or operations where overflow is logic-impossible.
    *   **Short-Circuiting:** Place cheaper checks before expensive ones in boolean expressions.

4.  **Advanced Techniques:**
    *   **Access Lists (EIP-2930):** Utilize access lists for complex multi-hop transactions to convert "cold" reads (~2100 gas) into "warm" reads (~100 gas).
    *   **Permit2:** Prefer signature-based approvals (Permit2) over separate `approve` transactions to bundle authorization with execution.

## Initial Objectives

1. **Understand existing production routers**  
   Analyse real v4 router deployments (e.g.
   [`0x89110a5a3e01760f88966feefa3f5966c2d1c940`](https://etherscan.io/address/0x89110a5a3e01760f88966feefa3f5966c2d1c940#code))
   to catalogue common patterns: lock acquisition, hook callbacks, settlement semantics, fee
   routing, and safety checks.

2. **Prototype a minimal-yet-flexible v4 router**  
   Implement a contract that executes the canonical flow:
   ```
   PoolManager.lock(...)
       -> swap(PoolKey, SwapParams, hookData)
       -> settle/take per currency
   ```
   The goal is to support buy/approve/sell simulations and real trades while keeping the code easy
   to audit and test.

3. **Design for extensibility**  
   Architect the router so new venues can plug in specialised swap adapters without rewriting the
   core execution pipeline. The docs/ folder will track the abstraction layers we settle on.

## Repository Layout

```
sol/
  baygus-router/
    README.md                <-- this document (project charter + high-level architecture)
    contracts/               <-- Solidity sources and shared libraries
    references/              <-- verified router implementations, ABI snapshots, design notes
    docs/                    <-- deep-dive design documents & threat modelling
    tools/                   <-- offline scripts & utilities (e.g. v0.1 transaction replay)
    uniswap-v4/              <-- protocol-specific research & adapter planning
```

Each subdirectory has its own README describing the expected contents. Start new work by dropping
research artifacts into `references/`, fleshing out design ideas in `docs/`, and iterating on the
router implementation inside `contracts/`. See `docs/README.md` for the milestone plan (v0.1 →
v0.6) and `docs/code-audit.md` for the current feature audit and gaps.

## Immediate Next Steps

- Catalogue behaviours of the router at `0x8911…c940` (permissions, settlement flow, hook data).
- Draft execution and state diagrams describing multi-hop routing & adapter lifecycle.
- Track mainnet Uniswap v4 pool deployments with non-zero liquidity so the PyReth example can be
  re-run against live data.
- Extend the Foundry project to cover multi-hop scenarios and document integration points for the
  Baygus Rust stack.

---

## Current Status (2025-10-20)

- **Solidity** – `contracts/src/BaygusRouter.sol` handles single- and multi-hop swaps, wraps/unwraps
  WETH, and propagates hook data. Foundry tests cover the happy path, slippage reverts, and hook
  adapters (`forge test`).
- **Rust integration** – `tx_processor` now deploys the router inside the `check_can_buy_sell_pool`
  simulation, wraps/approves WETH as needed, and executes buy → approve → sell via the new bytecode
  builders in `reth_chain_query`. PyReth exposes this through
  `PoolBuySellSimulator.set_uniswap_v4_config(...)`.
- **Data availability** – The current Reth snapshot does not include any Uniswap v4 pools that
  return slot0/liquidity, so the example run ends after router deployment with the informative
  failure message “Baygus router deployment succeeded but bytecode not visible in simulation
  state / target has no bytecode”. We keep the simulation in place so that a pool can be dropped in
  as soon as it becomes tradeable.

---

## Development Workflow

1. **Compile the Solidity artifacts**
   ```bash
   cd sol/baygus-router
   forge build
   ```
   This refreshes `out/BaygusRouter.sol/BaygusRouter.json`, which the Rust builder loads at runtime.
   (A placeholder `sol/baygus-router/contracts/uniswap_v4/MinimalV4Router.bin` is still required for legacy tooling.)

2. **Build the Rust crates without running tests**
   ```bash
    cd ../../rust
    cargo test --manifest-path pyreth/Cargo.toml --lib --no-run
   ```
   This recompiles `tx_simulator`, `tx_processor`, and `reth_chain_query` with the latest ABI.

3. **Expose the updated bindings to PyReth**
   ```bash
   cd pyreth
   maturin develop
   ```

4. **Exercise the Uniswap v4 flow (expected failure until live liquidity exists)**
   ```bash
   python examples/pool_buy_sell_simulator/uniswap_v4_pools.py
   ```
   The script prints the router deployment/deposit/approval steps and surfaces why the swap cannot
   complete. Once a slot0-capable pool is available, this example should flip to a successful
   buy/approve/sell trace.
