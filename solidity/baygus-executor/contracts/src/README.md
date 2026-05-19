# Source

`UniswapV2TradingVault.sol` is the only production entry point.

The vault is intentionally narrow:

- buy ETH -> token through the configured Uniswap V2 router;
- hold bought tokens in the vault with no standing sell-router allowance;
- approve the exact sell amount and sell token -> ETH inside one emergency
  transaction;
- clear sell-router allowance after the emergency sell;
- send ETH proceeds to the configured treasury.

The vault does not infer routes, discover pools, quote output, choose gas, or
pay bribes. Rust must supply the exact token, amount, min-output, deadline, gas
plan, and transaction metadata before signing.
