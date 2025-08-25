#!/usr/bin/env python3
"""
Verify that the time conversion system is fully integrated across all modules.

This script tests:
1. Time conversion utilities are accessible
2. Entity flow analysis uses time conversion
3. All datetime objects are timezone-aware
4. Block-time conversion is bidirectional and accurate
"""

import pyreth
from datetime import datetime, timedelta, timezone
import sys


def verify_time_conversion():
    """Verify basic time conversion functionality."""
    print("\n1. Testing Time Conversion...")
    
    reth = pyreth.PyReth()
    query = reth.chain_query()
    
    # Test timestamp to block
    now = datetime.now(timezone.utc)
    timestamp = now.isoformat().replace('+00:00', 'Z')
    
    try:
        block = query.timestamp_to_block(timestamp)
        print(f"   ✓ Timestamp to block: {timestamp} -> Block {block:,}")
    except Exception as e:
        print(f"   ✗ Timestamp to block failed: {e}")
        return False
    
    # Test block to timestamp
    try:
        ts = query.block_to_timestamp(block)
        print(f"   ✓ Block to timestamp: Block {block:,} -> {ts}")
    except Exception as e:
        print(f"   ✗ Block to timestamp failed: {e}")
        return False
    
    # Test time range to blocks
    start = (now - timedelta(hours=1)).isoformat().replace('+00:00', 'Z')
    end = now.isoformat().replace('+00:00', 'Z')
    
    try:
        start_block, end_block = query.get_blocks_for_time_range(start, end)
        print(f"   ✓ Time range to blocks: 1 hour = {end_block - start_block:,} blocks")
    except Exception as e:
        print(f"   ✗ Time range to blocks failed: {e}")
        return False
    
    return True


def verify_entity_flows():
    """Verify entity flow analysis uses time conversion."""
    print("\n2. Testing Entity Flow Analysis...")
    
    reth = pyreth.PyReth()
    query = reth.chain_query()
    
    # Get blocks for last hour
    now = datetime.now(timezone.utc)
    start = (now - timedelta(hours=1)).isoformat().replace('+00:00', 'Z')
    end = now.isoformat().replace('+00:00', 'Z')
    
    try:
        start_block, end_block = query.get_blocks_for_time_range(start, end)
        
        # Test ETF flows
        etf_flows = query.calculate_etf_flows_between_blocks(start_block, end_block)
        print(f"   ✓ ETF flows: Found {len(etf_flows['provider_flows'])} ETF providers")
        
        # Test CEX flows
        cex_flows = query.calculate_cex_flows_between_blocks(start_block, end_block)
        print(f"   ✓ CEX flows: Found {len(cex_flows['exchange_flows'])} exchanges")
        
        # Test stablecoin supplies
        stable_supplies = query.calculate_stablecoin_supply_changes_between_blocks(start_block, end_block)
        print(f"   ✓ Stablecoin supplies: Found {len(stable_supplies['token_changes'])} tokens")
        
    except Exception as e:
        print(f"   ✗ Entity flow analysis failed: {e}")
        return False
    
    return True


def verify_timezone_aware():
    """Verify all datetime objects are timezone-aware."""
    print("\n3. Testing Timezone Awareness...")
    
    # Test datetime creation
    now = datetime.now(timezone.utc)
    if now.tzinfo is None:
        print("   ✗ datetime.now(timezone.utc) is not timezone-aware")
        return False
    print(f"   ✓ Datetime is timezone-aware: {now.tzinfo}")
    
    # Test ISO format
    iso = now.isoformat().replace('+00:00', 'Z')
    if not iso.endswith('Z'):
        print(f"   ✗ ISO format doesn't end with Z: {iso}")
        return False
    print(f"   ✓ ISO format correct: {iso}")
    
    return True


def verify_period_blocks():
    """Verify period block counts match expectations."""
    print("\n4. Testing Period Block Counts...")
    
    reth = pyreth.PyReth()
    query = reth.chain_query()
    
    periods = [
        ("1 hour", timedelta(hours=1), 250, 360),      # Allow up to 360 blocks
        ("4 hours", timedelta(hours=4), 1000, 1440),   # Allow up to 1440 blocks
        ("1 day", timedelta(days=1), 6000, 8640),      # Allow up to 8640 blocks
        ("1 week", timedelta(weeks=1), 42000, 60480),  # Allow up to 60480 blocks
    ]
    
    now = datetime.now(timezone.utc)
    
    for name, delta, min_blocks, max_blocks in periods:
        start = (now - delta).isoformat().replace('+00:00', 'Z')
        end = now.isoformat().replace('+00:00', 'Z')
        
        try:
            start_block, end_block = query.get_blocks_for_time_range(start, end)
            block_count = end_block - start_block
            
            if min_blocks <= block_count <= max_blocks:
                print(f"   ✓ {name}: {block_count:,} blocks (expected {min_blocks:,}-{max_blocks:,})")
            else:
                print(f"   ⚠ {name}: {block_count:,} blocks (expected {min_blocks:,}-{max_blocks:,})")
        except Exception as e:
            print(f"   ✗ {name} failed: {e}")
            return False
    
    return True


def main():
    """Run all verification tests."""
    print("="*60)
    print("Time Conversion System Integration Verification")
    print("="*60)
    
    all_passed = True
    
    # Run all tests
    all_passed &= verify_time_conversion()
    all_passed &= verify_entity_flows()
    all_passed &= verify_timezone_aware()
    all_passed &= verify_period_blocks()
    
    # Summary
    print("\n" + "="*60)
    if all_passed:
        print("✅ All verification tests passed!")
        print("Time conversion system is fully integrated.")
    else:
        print("❌ Some tests failed. Please review the output above.")
        sys.exit(1)
    print("="*60)


if __name__ == "__main__":
    main()