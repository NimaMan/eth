# Baygus Router

Baygus Router is the Solidity execution layer used by the Ethereum simulation stack. Its job is to
turn a typed off-chain route into one on-chain transaction surface that can:

- execute Uniswap v4 `unlock -> swap -> settle/take` flows;
- net intermediate currencies across v4 paths;
- run simple command sequences such as pull, V2/Sushi/V3 swap, Curve swap, Balancer swap, flash
  loan, and sweep;
- keep protocol adapter addresses configurable at deployment.

The Rust builders load Foundry artifacts from `out/`. Rebuild artifacts after contract changes:

```bash
cd contracts
forge build
forge test
```

The router intentionally keeps calldata formats explicit. If a command needs a new behavior, add a
typed Solidity test first, then add the matching Rust builder.
