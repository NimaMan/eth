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
- Extend the Foundry project to cover multi-hop scenarios and document integration points for the
  Baygus Rust stack.
