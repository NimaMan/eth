# Interfaces

Minimal interfaces for external contracts the executor calls.

Keep these narrow. Add only the methods used by `BaygusExecutor.sol`, and prefer protocol-specific
interfaces over importing large vendor packages.

`IPermit2.transferFrom` is the AllowanceTransfer pull surface used by
`CMD_PERMIT2_TRANSFER_FROM`.

`IPoolManager.swap` intentionally returns `int256` because Uniswap v4 packs `BalanceDelta` into one
word on mainnet. Do not change this to a two-field struct return; that ABI-decodes incorrectly
against the deployed PoolManager.
