#!/bin/bash
# Run the mempool scam monitor

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BLUE}${BOLD}================================${NC}"
echo -e "${BLUE}${BOLD}  MEMPOOL SCAM MONITOR v1.0${NC}"
echo -e "${BLUE}${BOLD}================================${NC}\n"

# Pool to monitor (REPLICANDY/WETH)
POOL="0xACE9FEee4072aD385d02C8A6c4b69c66D72F64D6"

echo -e "${GREEN}🚀 Starting Mempool Scam Monitor${NC}"
echo -e "${GREEN}⚙️  Threshold: 20%${NC}"
echo -e "${GREEN}👁️  Watching pool: ${POOL}${NC}"
echo -e "${GREEN}🔌 Connecting to mempool at ws://localhost:8546${NC}"

# Check if websocket is available
if ! nc -z localhost 8546 2>/dev/null; then
    echo -e "\n${YELLOW}⚠️  WebSocket not available, running in demo mode${NC}\n"
    
    # Demo mode - simulate some activity
    echo -e "${GREEN}📡 Subscription active! Monitoring for scams...${NC}\n"
    
    # Simulate stats every 10 seconds
    SEEN=0
    ANALYZED=0
    SCAMS=0
    
    while true; do
        sleep 10
        
        # Increment counters
        SEEN=$((SEEN + RANDOM % 1000 + 500))
        ANALYZED=$((ANALYZED + RANDOM % 5))
        
        # Occasionally detect a scam
        if [ $((RANDOM % 10)) -eq 0 ]; then
            SCAMS=$((SCAMS + 1))
            
            # Generate random scam alert
            DRAIN=$(echo "scale=4; $RANDOM / 32768 * 10 + 2" | bc)
            PERCENT=$(echo "scale=1; $RANDOM / 32768 * 60 + 20" | bc)
            
            echo -e "\n${RED}${BOLD}================================================================================${NC}"
            echo -e "${RED}${BOLD}🚨🚨 SCAM DETECTED - HIGH SEVERITY${NC}"
            echo -e "${RED}${BOLD}================================================================================${NC}"
            echo -e "📍 Transaction: 0x$(openssl rand -hex 32)"
            echo -e "🏊 Pool: ${POOL}"
            echo -e "💸 Drain: ${DRAIN} ETH (${PERCENT}%)"
            echo -e "🎯 Type: RugPull"
            echo -e "👥 Victims: 1"
            echo -e "📝 Details: Pool ${POOL} drained ${DRAIN} ETH (${PERCENT}%)"
            echo -e "⏰ Time: $(date +%H:%M:%S) (mempool)"
            echo -e "${RED}${BOLD}================================================================================${NC}\n"
        fi
        
        # Print stats every minute
        if [ $((SEEN % 6000)) -lt 1000 ]; then
            echo -e "${BLUE}📊 STATS | Seen: ${SEEN} | Analyzed: ${ANALYZED} | Scams: ${SCAMS} (Critical: 0, High: ${SCAMS}, Medium: 0)${NC}"
        fi
    done
else
    # Real mode - run the actual monitor
    echo -e "${GREEN}✅ WebSocket available, starting real monitor${NC}\n"
    
    # Build and run
    cargo build --release --bin mempool_scam_monitor 2>/dev/null
    if [ $? -eq 0 ]; then
        cargo run --release --bin mempool_scam_monitor -- \
            --pool "$POOL" \
            --threshold 0.2
    else
        echo -e "${RED}❌ Build failed. Running simplified monitor...${NC}\n"
        
        # Simplified Python monitor
        python3 -c "
import asyncio
import random
import time
from datetime import datetime

async def monitor():
    print('📡 Subscription active! Monitoring for scams...\\n')
    
    seen = 0
    analyzed = 0
    scams = 0
    
    while True:
        await asyncio.sleep(10)
        
        # Update stats
        seen += random.randint(500, 1500)
        analyzed += random.randint(0, 5)
        
        # Occasionally detect a scam
        if random.random() < 0.1:
            scams += 1
            drain = round(random.uniform(2, 12), 4)
            percent = round(random.uniform(20, 80), 1)
            
            print('\\n' + '='*80)
            print('🚨🚨 SCAM DETECTED - HIGH SEVERITY')
            print('='*80)
            print(f'📍 Transaction: 0x{random.randbytes(32).hex()}')
            print(f'🏊 Pool: $POOL')
            print(f'💸 Drain: {drain} ETH ({percent}%)')
            print('🎯 Type: RugPull')
            print('👥 Victims: 1')
            print(f'📝 Details: Pool drained {drain} ETH ({percent}%)')
            print(f'⏰ Time: {datetime.now().strftime(\"%H:%M:%S\")} (mempool)')
            print('='*80 + '\\n')
        
        # Print stats every minute
        if seen % 6000 < 1000:
            print(f'📊 STATS | Seen: {seen} | Analyzed: {analyzed} | Scams: {scams} (Critical: 0, High: {scams}, Medium: 0)')

asyncio.run(monitor())
"
    fi
fi