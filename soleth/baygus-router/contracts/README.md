# Contracts

This directory contains the Solidity sources for the Baygus Router project.

## Planned Structure

```
contracts/
  src/
    BaygusRouter.sol         <-- primary entry point (initially Uniswap v4 focused)
    adapters/                <-- venue-specific swap adapters
    libraries/               <-- shared helpers (math, settlement utilities, safety checks)
  test/
    <foundry-or-hardhat suites>
```

We have not generated any source files yet; the first milestone is to translate the behaviour
documented in `references/` into a minimal router contract that:

1. Accepts a `PoolKey` + swap parameters.
2. Manages `PoolManager.lock` lifecycle safely.
3. Performs settlement/take flows for both ERC-20 and native currencies.

As the design stabilises we will add Foundry/Hardhat scaffolding here (e.g. `foundry.toml`,
deployment scripts, interface packs). Every new component should document its assumptions and
threat model directly in code comments and the `docs/` folder.
