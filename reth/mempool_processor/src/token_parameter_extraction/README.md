# Token Parameter Extraction Module

## Tax Calculation Algorithm

The tax calculator uses a **sequential buy and sell simulation** to measure token taxes:

### Core Algorithm:

1. **Buy Simulation**: 
   - Buyer address `0x0C96c602b1b332B8AB2093E5d72D804a24bd5689` buys tokens with 0.01 ETH
   - Track how many tokens leave the pool
   - Track how many tokens the buyer receives
   - **Buy Tax** = `(1 - tokens_received / tokens_left_pool) × 100`

2. **Sell Simulation** (using tokens from buy):
   - Seller sells ALL tokens received from the buy
   - Track how much ETH leaves the pool  
   - Track how much ETH the seller receives
   - **Sell Tax** = `(1 - eth_received / eth_left_pool) × 100`

### Two Modes:

**Mode 1: Measure Current Taxes**
```
Buy → Sell
```
Simply measures the current tax rates.

**Mode 2: Measure Tax Impact of Creator Transaction**
```
Creator TX → Buy → Sell
```
1. First simulate a creator's transaction (e.g., setTaxes)
2. Then run buy → sell to measure the new tax rates
3. Compare before/after to see the tax change impact

### Key Points:

- Uses sequential simulation: the sell uses tokens from the buy
- Fixed buyer address: `0x0C96c602b1b332B8AB2093E5d72D804a24bd5689`
- Tracks actual asset movements via state changes
- Works with any tax implementation (doesn't need to understand the contract)

### Example:

If a token has 5% buy tax and 90% sell tax:
- Buy: Pool sends 100 tokens, buyer receives 95 tokens → 5% buy tax
- Sell: Seller sells 95 tokens, pool sends 1 ETH, seller receives 0.1 ETH → 90% sell tax