# Baygus Trading Vault

Baygus Trading Vault is the minimal Solidity execution layer for live scam-exit
testing. It is no longer a broad command router. The v1 contract supports Mode A:

- buy through the vault and hold the bought token in the vault;
- keep no sell-router allowance after buy;
- on emergency exit, approve the exact sell amount and sell in the same
  top-level transaction;
- send ETH proceeds to the configured treasury.
- tolerate V2 fee-on-transfer token paths by using the router's supporting
  fee-on-transfer methods and measuring net received balances.

## Design goal

Baygus should be cheap on-chain. The off-chain simulator and search pipeline
should do every piece of work that can be done before signing: route discovery,
pool selection, calldata construction, price checks, slippage bounds, block
bounds, gas estimates, profit checks, and bribe sizing. The deployed contract
should only contain primitives that must execute atomically on-chain.

For this Mode A vault, the atomic primitive we need is approve+sell during the
emergency exit. We avoid paying approval gas on every buy, but the emergency
sell gas estimate must include the exact approval and allowance-clear steps.

The Rust tx builders encode the vault ABI directly and tests verify the
selectors. Rebuild Foundry artifacts after contract changes:

```bash
cd contracts
forge build
forge test
```

The vault intentionally keeps calldata explicit. If we need another hot path,
add a typed Solidity test first, benchmark it against direct router calldata,
then add the matching Rust builder.

## Current Surface

- `buyV2ExactEthForTokens(token, minTokensOut, deadline)`: swaps ETH to token
  through the configured Uniswap V2 router and keeps the token in the vault.
- `emergencySellV2ExactTokensForEth(token, amountIn, minEthOut, deadline)`:
  approves the exact token amount, sells to ETH, clears allowance, and sends ETH
  to treasury.
- `rescueToken(token, to, amount)` and `rescueEth(to, amount)`: owner-only
  recovery functions.

## Pre-deployment gate

Do not deploy a new executor until all of these are true:

- `forge fmt --check`, `forge build`, and `forge test` pass from `contracts/`.
- The Rust tx builders compile and selector tests match `BaygusTradingVault`.
- Constructor arguments are fixed: owner, treasury, WETH, and Uniswap V2 router.
- The bytecode hash and ABI diff are recorded next to the deployment note.
- The exact buy and emergency-sell calldata are simulated against target block
  state before signing.
- A gas benchmark proves Mode A is acceptable versus pre-approving on buy.
- Fee-on-transfer token coverage passes locally and on a representative mainnet
  fork fixture.
