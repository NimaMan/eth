# Case Study: Mind of Pepe Launch (Block 23606650)

## Objective
Understand how the launch sequence lines up in one block: contract creation, liquidity add, and the flurry of sniper interactions. Establish provenance of the helper contracts and illustrate how the deployer/same-block actors coordinate.

## Actors
- **Deployer / Owner:** `0x539714c8aA548f98e6D564486BFb6658326840A1`
- **Token Address:** `0x2A179Bfab23c71c733864776bEa6e0daB9eCD79e`
- **Liquidity Pool:** `0xFB02Ffd5b26229464C0d41f0fA9B6895CDdDA885`
- **Router:** Uniswap V2 Router (`0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D`)
- **Helper Contract Observed:** `0x3328F7f4A1D1C57c35df56bBf0c9dCAFCA309C49`

## Timeline (Block 23606650)
1. **Nonce 0 — Contract Creation**  
   Tx `0x027399b9a14e8e7c1d86b8f82f205abdef3b5c4ccbc2475cba70dcbbcf187b2b`  
   - Sent by the deployer.  
   - Deploys `Mind of Pepe` (`0x2A179Bfa…`).

2. **Nonce 1 — `openTrading()`**  
   Tx `0x7468044e32eeab38ed8afe6ee17648a8effa6057da9f152be03ff3a2aea7af22`  
   - Calls `openTrading()` on the token contract.  
   - Internally: approves router, adds 0.5 ETH + 980M MIND liquidity via Uniswap.  
   - Emits pair creation / sync logs establishing pool `0xFB02Ffd5…`.

3. **Sniper / Helper Wave**  
   - Immediately after nonce 1, the block includes 40+ router/helper interactions.  
   - Examples:  
     • `0x2b7444bedda25c00470797ae772c2c48b461c3feb198cf6d95243471b2f0456b` (invokes helper `0x3328…`)  
     • `0x73fe4231…`, `0x08c09eb6…`, etc.  
   - Most helpers are EOAs calling the router directly; a subset uses `0x3328…`, possibly a copy-trading bot or bundle aggregator.

## Observations
- **Same-block orchestration:** the deployer’s actions and the snipers all land in block 23606650, implying an MEV bundle or pre-signed set of txs submitted via a private relay.  
- **Helper contract (`0x3328…`):** address cannot be enumerated via `eth_getCode` because it’s self-destructed post-launch (or never deployed in the first place—call returns revert). Likely a short-lived contract used solely in this block.  
- **Router reverts when replayed:** In a sandbox, we only execute the helper transactions in isolation, so they revert — the state from steps 1–2 isn’t applied.  
- **Snipers’ addresses:** include a mix of known copy-trader wallets and opportunistic EOAs.

## Implications for Simulation
- To reproduce post-launch state, we must apply the entire ordered sequence (deploy → openTrading → helper) before running a viability check.  
- The mempool processor should maintain a per-deployer queue of pending txs so that, if multiple txs from the same entity arrive, we can replay them in order even before they’re mined.

## Further Work
- Catalog helper contracts (`0x3328…` and others) across launches to see if they belong to a common bot network.  
- Verify whether these helpers deploy ephemeral contracts via `CREATE2` or depend on pre-existing bytecode.  
- Capture additional metadata (Gas price/priority, bundle hints) to label which transactions are likely in the same MEV package.
