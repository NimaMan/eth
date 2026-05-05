# Interfaces

Minimal interfaces for external contracts the router calls.

Keep these narrow. Add only the methods used by `BaygusRouter.sol`, and prefer protocol-specific
interfaces over importing large vendor packages.

`IPoolManager.swap` intentionally returns `int256` because Uniswap v4 packs `BalanceDelta` into one
word on mainnet. Do not change this to a two-field struct return; that ABI-decodes incorrectly
against the deployed PoolManager.
