#!/usr/bin/env python3
"""
Simple demonstration of time conversion utilities in ChainQuery.

This example shows:
- Converting blocks to timestamps
- Converting timestamps to blocks
- Working with different time periods
- Getting block ranges for time windows
"""

from pyreth import chain_query as pyreth_chain_query
from datetime import datetime, timedelta, timezone


def demo_basic_conversion():
    """Demonstrate basic block-time conversions."""
    print("🕐 Basic Block-Time Conversion")
    print("=" * 50)
    
    # Initialize PyReth
    query = pyreth_chain_query()
    
    # Get current block
    latest_block = query.get_latest_block()
    print(f"\nLatest block: {latest_block:,}")
    
    # Convert block to timestamp
    timestamp = query.block_to_timestamp(latest_block)
    print(f"Timestamp: {timestamp}")
    
    # Convert back to block
    block_again = query.timestamp_to_block(timestamp)
    print(f"Back to block: {block_again:,}")
    
    # Show some historical blocks
    print("\n📅 Historical Block Times:")
    print("-" * 40)
    
    historical_blocks = [
        (latest_block - 300, "~1 hour ago"),
        (latest_block - 7200, "~1 day ago"),
        (latest_block - 50400, "~1 week ago"),
        (20_000_000, "Block 20M"),
        (15_000_000, "Block 15M"),
    ]
    
    for block, description in historical_blocks:
        timestamp = query.block_to_timestamp(block)
        print(f"Block {block:,} ({description})")
        print(f"  → {timestamp}")


def demo_time_to_blocks():
    """Demonstrate converting time ranges to block ranges."""
    print("\n\n⏰ Time Ranges to Block Ranges")
    print("=" * 50)
    
    query = pyreth_chain_query()
    
    # Define some time ranges
    now = datetime.now(timezone.utc)
    time_ranges = [
        ("Last hour", now - timedelta(hours=1), now),
        ("Yesterday", now - timedelta(days=1), now - timedelta(hours=23)),
        ("Last week", now - timedelta(weeks=1), now),
        ("Specific date", datetime(2024, 1, 1), datetime(2024, 1, 2)),
    ]
    
    print("\nConverting time ranges to blocks:")
    print("-" * 40)
    
    for description, start_time, end_time in time_ranges:
        start_iso = start_time.isoformat().replace('+00:00', 'Z')
        end_iso = end_time.isoformat().replace('+00:00', 'Z')
        
        try:
            start_block, end_block = query.get_blocks_for_time_range(
                start_iso, end_iso
            )
            
            duration = end_time - start_time
            block_count = end_block - start_block
            
            print(f"\n{description}:")
            print(f"  Time: {start_time:%Y-%m-%d %H:%M} to {end_time:%Y-%m-%d %H:%M}")
            print(f"  Duration: {duration}")
            print(f"  Blocks: {start_block:,} to {end_block:,}")
            print(f"  Total blocks: {block_count:,}")
            
        except Exception as e:
            print(f"\n{description}: Error - {e}")


def demo_period_boundaries():
    """Demonstrate getting period boundaries."""
    print("\n\n📊 Period Boundaries")
    print("=" * 50)
    
    query = pyreth_chain_query()
    
    # Different period types
    periods = [
        ('hour', 6, "Last 6 hours"),
        ('day', 7, "Last 7 days"),
        ('week', 4, "Last 4 weeks"),
    ]
    
    for period_type, count, description in periods:
        print(f"\n{description} (in {period_type}ly periods):")
        print("-" * 40)
        
        boundaries = query.get_blocks_for_last_n_periods(count, period_type)
        
        for i, boundary in enumerate(boundaries):
            block_count = boundary['end_block'] - boundary['start_block']
            
            # Parse timestamps for display
            start_time = datetime.fromisoformat(
                boundary['start_time'].replace('Z', '+00:00')
            )
            end_time = datetime.fromisoformat(
                boundary['end_time'].replace('Z', '+00:00')
            )
            
            print(f"\nPeriod {i + 1}:")
            print(f"  Time: {start_time:%m-%d %H:%M} to {end_time:%m-%d %H:%M}")
            print(f"  Blocks: {boundary['start_block']:,} to {boundary['end_block']:,}")
            print(f"  Count: {block_count:,} blocks")


def demo_block_time_estimation():
    """Demonstrate block time estimation without DB access."""
    print("\n\n🔮 Block Time Estimation")
    print("=" * 50)
    
    print("\nEstimating timestamps (using 12s average block time):")
    print("-" * 40)
    
    # These use the static estimation functions
    test_blocks = [1_000_000, 10_000_000, 20_000_000]
    
    for block in test_blocks:
        # This would use the estimation formula
        # timestamp = GENESIS_TIMESTAMP + (block * 12 seconds)
        print(f"Block {block:,}: ~July 2015 + {block * 12:,} seconds")
    
    print("\nNote: Actual timestamps from chain are more accurate!")


def demo_practical_use_case():
    """Demonstrate a practical use case."""
    print("\n\n💡 Practical Use Case: Daily Analysis Window")
    print("=" * 50)
    
    query = pyreth_chain_query()
    
    # Get blocks for different analysis windows
    windows = [
        ("Real-time (1 hour)", 1, 'hour'),
        ("Daily report", 1, 'day'),
        ("Weekly summary", 1, 'week'),
    ]
    
    print("\nAnalysis windows for different use cases:")
    print("-" * 40)
    
    for use_case, count, period in windows:
        boundaries = query.get_blocks_for_last_n_periods(count, period)
        
        if boundaries:
            boundary = boundaries[0]
            start_block = boundary['start_block']
            end_block = boundary['end_block']
            block_count = end_block - start_block
            
            # Estimate data volume (assuming ~200 txs per block average)
            estimated_txs = block_count * 200
            
            print(f"\n{use_case}:")
            print(f"  Block range: {start_block:,} - {end_block:,}")
            print(f"  Total blocks: {block_count:,}")
            print(f"  Est. transactions: {estimated_txs:,}")
            print(f"  Query load: {'Light' if block_count < 1000 else 'Medium' if block_count < 10000 else 'Heavy'}")


def main():
    """Run all demonstrations."""
    try:
        demo_basic_conversion()
        demo_time_to_blocks()
        demo_period_boundaries()
        demo_block_time_estimation()
        demo_practical_use_case()
        
        print("\n\n✅ All Time Conversion Demos Complete!")
        
    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()