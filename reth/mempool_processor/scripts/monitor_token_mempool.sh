#!/bin/bash
# Monitor mempool transactions for specific token and its pool

# Token and pool addresses
export TOKEN_ADDRESS="0x2e32f96a4FbB9cD7CDC751971c015E282414B956"
export POOL_ADDRESS="0xACE9FEee4072aD385d02C8A6c4b69c66D72F64D6"
export WATCHED_TOKENS="${TOKEN_ADDRESS},${POOL_ADDRESS}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Mempool Monitor for Token ${TOKEN_ADDRESS} ===${NC}"
echo -e "${GREEN}Token: ${TOKEN_ADDRESS}${NC}"
echo -e "${GREEN}Pool:  ${POOL_ADDRESS} (Uniswap V2 WETH pair)${NC}"
echo -e "${YELLOW}Starting mempool processor with enhanced logging...${NC}\n"

# Create a custom log format with timestamps
export RUST_LOG="mempool_processor=debug,info"
export RUST_BACKTRACE=1

# Run the mempool processor with token tracking
echo -e "${PURPLE}Monitoring for:${NC}"
echo -e "  • Direct token transfers (transfer, transferFrom)"
echo -e "  • Token approvals"
echo -e "  • Pool swaps (buy/sell transactions)"
echo -e "  • Liquidity additions/removals"
echo -e "  • Any transaction involving the token or pool\n"

# Start monitoring
cargo run --bin mempool_processor 2>&1 | while IFS= read -r line; do
    # Highlight token-related transactions
    if echo "$line" | grep -qi "$TOKEN_ADDRESS\|$POOL_ADDRESS"; then
        echo -e "${GREEN}[TOKEN TX DETECTED]${NC} $line"
    elif echo "$line" | grep -qi "transfer\|approve\|swap\|mint\|burn\|sync"; then
        echo -e "${YELLOW}[RELEVANT METHOD]${NC} $line"
    elif echo "$line" | grep -qi "alert\|scam\|drain"; then
        echo -e "${RED}[ALERT]${NC} $line"
    else
        echo "$line"
    fi
done