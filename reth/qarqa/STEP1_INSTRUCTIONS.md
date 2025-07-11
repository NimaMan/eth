# Step 1: Pool Transaction Audit Instructions

## What We Built

A real database query tool that will:

1. **Connect to your eth_db PostgreSQL database**
2. **Query the pool address**: `0x0e9797f0f05a3de8384d76467e98da03874c86a6`
3. **Get all transactions involving this pool**  
4. **Filter to block ≤ 22885510** (the liquidity addition block)
5. **Print the actual transaction hashes and block numbers**

## To Run This

Set your database environment variables:
```bash
export DB_HOST="your_host"
export DB_PORT="5432"
export DB_NAME="eth_db"
export DB_USER="your_user"
export DB_PASSWORD="your_password"

# Or as a single DATABASE_URL:
export DATABASE_URL="postgresql://user:pass@host:port/eth_db"
```

Then run:
```bash
cargo run --example step1_pool_transactions
```

## Expected Output

Should show something like:
```
Step 1: Fetching real pool transactions from database
====================================================
Pool address: 0x0e9797f0f05a3de8384d76467e98da03874c86a6
Liquidity added at block: 22885510

1. Checking if pool exists in database...
✅ Pool found in database:
   Name: Some("Uniswap V2: ByteBond")
   Entity Category: Some("DEX")
   Is Contract: true

2. Fetching all transactions for pool...
✅ Found 15 total transactions for pool
   Retrieved 15 transaction hashes

3. Getting block numbers and filtering by block ≤ 22885510...

4. Results:
   Total pool transactions: 15
   Checked: 15
   Relevant (block ≤ 22885510): 3

5. Relevant transactions (sorted by block):
   1. Block 22885510: 0x5e439aa276849d6ba592b5bce910fefd79c0d26b9d185d95f367848d6a1f6164
      ↑ THIS IS THE LIQUIDITY ADDITION TRANSACTION
   2. Block 22885509: 0x1234567890123456789012345678901234567890...
   3. Block 22885508: 0xabcdef1234567890123456789012345678901234...

6. Checking for MEXC transaction...
✅ MEXC transaction found:
   Block: 22885482
   From: 0x9642b23Ed1E01Df1092B92641051881a322F5D4E
   To: Some("0xC04B517E75907965AD59976c63912C8C8af97D96")
   Value: 1399850000000000000

✅ Step 1 complete. Found 3 relevant transactions.
```

## Key Points

- **NO MOCK DATA**: Everything comes from real PostgreSQL queries
- **Real tx_participants table**: Uses the indexed address→transaction mapping  
- **Real transactions table**: Gets actual block numbers and transaction details
- **Filtered by block**: Only shows transactions ≤ liquidity addition block
- **Finds both transactions**: Pool liquidity + MEXC withdrawal

## What This Proves

1. The pool address exists in your database
2. The tx_participants table correctly maps addresses to transactions
3. We can get the actual transaction hashes and blocks
4. Both our target transactions (MEXC + liquidity) should be found

Run this first, then we'll move to Step 2: processing these transactions with tx_processor.