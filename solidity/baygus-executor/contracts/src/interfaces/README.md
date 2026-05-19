# Interfaces

Minimal interfaces for external contracts the vault calls.

Keep these narrow. Add only the methods used by `BaygusTradingVault.sol`, and
prefer protocol-specific interfaces over importing large vendor packages.

`IUniswapV2Router02` only includes the two fee-on-transfer-supporting methods
used by Mode A buy and emergency sell. Route discovery, quoting, and slippage
selection stay off-chain.
