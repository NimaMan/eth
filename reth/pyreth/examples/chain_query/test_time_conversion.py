#!/usr/bin/env python3
"""
Test example for block-time conversion through pyreth.

Demonstrates:
- Using ChainQuery's time conversion methods
- Converting between blocks and timestamps
- Getting period boundaries
- Integration with entity flow analysis
"""

import pyreth
from datetime import datetime, timedelta, timezone
import json


def test_block_time_conversion():
    """Test basic block-time conversion."""
    print("🕐 Testing Block-Time Conversion")
    print("=" * 50)
    
    # Initialize PyReth (singleton pattern)
    query = pyreth_chain_query()
    
    # Get latest block
    latest_block = query.get_latest_block()
    print(f"Latest block: {latest_block}")
    
    # Test block to timestamp
    print("\n📊 Block to Timestamp:")
    print("-" * 40)
    
    test_blocks = [
        latest_block,
        latest_block - 100,
        latest_block - 1000,
        latest_block - 7200,  # ~1 day ago
    ]
    
    for block in test_blocks:
        timestamp = query.block_to_timestamp(block)
        print(f"Block {block} -> {timestamp}")
    
    # Test timestamp to block
    print("\n📊 Timestamp to Block:")
    print("-" * 40)
    
    now = datetime.now(timezone.utc)
    test_times = [
        now.isoformat().replace('+00:00', 'Z'),
        (now - timedelta(hours=1)).isoformat().replace('+00:00', 'Z'),
        (now - timedelta(days=1)).isoformat().replace('+00:00', 'Z'),
        (now - timedelta(weeks=1)).isoformat().replace('+00:00', 'Z'),
    ]
    
    for timestamp in test_times:
        block = query.timestamp_to_block(timestamp)
        print(f"{timestamp} -> Block {block}")
    
    # Test period boundaries
    print("\n📊 Period Boundaries:")
    print("-" * 40)
    
    periods = [
        ('hour', 24),   # Last 24 hours
        ('day', 7),      # Last 7 days
        ('week', 4),     # Last 4 weeks
    ]
    
    for period_type, count in periods:
        print(f"\nLast {count} {period_type}s:")
        boundaries = query.get_blocks_for_last_n_periods(count, period_type)
        
        for i, boundary in enumerate(boundaries[:3]):
            print(f"  Period {i+1}: blocks {boundary['start_block']} - {boundary['end_block']}")
            print(f"    Time: {boundary['start_time']} to {boundary['end_time']}")
        
        if len(boundaries) > 3:
            print(f"  ... and {len(boundaries) - 3} more periods")


def test_entity_flows_with_time():
    """Test entity flow analysis using time conversion."""
    print("\n🏦 Testing Entity Flows with Time Conversion")
    print("=" * 50)
    
    # Initialize PyReth
    query = pyreth_chain_query()
    
    # Calculate blocks for last 24 hours
    now = datetime.now(timezone.utc)
    yesterday = now - timedelta(days=1)
    
    start_timestamp = yesterday.isoformat().replace('+00:00', 'Z')
    end_timestamp = now.isoformat().replace('+00:00', 'Z')
    
    start_block, end_block = query.get_blocks_for_time_range(
        start_timestamp, 
        end_timestamp
    )
    
    print(f"Time range: {yesterday:%Y-%m-%d %H:%M} to {now:%Y-%m-%d %H:%M}")
    print(f"Block range: {start_block} to {end_block}")
    print(f"Total blocks: {end_block - start_block}")
    
    # Get ETF flows for this period
    print("\n📊 ETF Flows (Last 24 Hours):")
    print("-" * 40)
    
    try:
        etf_flows = query.calculate_etf_flows_between_blocks(start_block, end_block)
        
        if 'providers' in etf_flows:
            for provider, data in list(etf_flows['providers'].items())[:5]:
                print(f"\n{provider}:")
                print(f"  Inflow: {data.get('inflow_eth', 0):.2f} ETH")
                print(f"  Outflow: {data.get('outflow_eth', 0):.2f} ETH")
                print(f"  Net: {data.get('net_flow_eth', 0):.2f} ETH")
        
        print(f"\nTotal ETF Inflow: {etf_flows.get('total_inflow', 0):.2f} ETH")
        print(f"Total ETF Outflow: {etf_flows.get('total_outflow', 0):.2f} ETH")
        print(f"Net Flow: {etf_flows.get('net_flow', 0):.2f} ETH")
    except Exception as e:
        print(f"Could not get ETF flows: {e}")
    
    # Get CEX flows for this period
    print("\n💱 CEX Flows (Last 24 Hours):")
    print("-" * 40)
    
    try:
        cex_flows = query.calculate_cex_flows_between_blocks(start_block, end_block)
        
        if 'exchanges' in cex_flows:
            for exchange, data in list(cex_flows['exchanges'].items())[:5]:
                print(f"\n{exchange}:")
                print(f"  Inflow: {data.get('inflow_eth', 0):.2f} ETH")
                print(f"  Outflow: {data.get('outflow_eth', 0):.2f} ETH")
                print(f"  Net: {data.get('net_flow_eth', 0):.2f} ETH")
        
        print(f"\nTotal CEX Inflow: {cex_flows.get('total_inflow', 0):.2f} ETH")
        print(f"Total CEX Outflow: {cex_flows.get('total_outflow', 0):.2f} ETH")
        print(f"Net Flow: {cex_flows.get('net_flow', 0):.2f} ETH")
    except Exception as e:
        print(f"Could not get CEX flows: {e}")
    
    # Get stablecoin supply changes
    print("\n💵 Stablecoin Supply Changes (Last 24 Hours):")
    print("-" * 40)
    
    try:
        supply_changes = query.calculate_stablecoin_supply_changes_between_blocks(
            start_block, 
            end_block
        )
        
        if 'token_changes' in supply_changes:
            for token, data in list(supply_changes['token_changes'].items())[:5]:
                if data.get('net_change', 0) != 0:
                    print(f"\n{token}:")
                    print(f"  Minted: ${data.get('minted', 0):,.0f}")
                    print(f"  Burned: ${data.get('burned', 0):,.0f}")
                    print(f"  Net Change: ${data.get('net_change', 0):,.0f}")
                    print(f"  Percent Change: {data.get('percent_change', 0):.2f}%")
        
        print(f"\nTotal Minted: ${supply_changes.get('total_minted', 0):,.0f}")
        print(f"Total Burned: ${supply_changes.get('total_burned', 0):,.0f}")
        print(f"Net Supply Change: ${supply_changes.get('total_net_change', 0):,.0f}")
    except Exception as e:
        print(f"Could not get stablecoin supply changes: {e}")


def test_aggregation_periods():
    """Test different aggregation periods."""
    print("\n📈 Testing Aggregation Periods")
    print("=" * 50)
    
    from datetime import datetime, timedelta, timezone
    
    query = pyreth_chain_query()
    
    # Get blocks for exact time ranges
    # Note: Actual block counts may vary from theoretical values due to
    # Ethereum block time variations (12-14 seconds instead of exactly 12)
    now = datetime.now(timezone.utc)
    periods = [
        (timedelta(hours=1), 'Hourly'),        # Theoretical: 300, Actual: 250-350
        (timedelta(hours=4), '4-Hourly'),      # Theoretical: 1200, Actual: 1000-1400
        (timedelta(days=1), 'Daily'),          # Theoretical: 7200, Actual: 6000-7500
        (timedelta(weeks=1), 'Weekly'),        # Theoretical: 50400, Actual: 45000-52000
        (timedelta(days=30), 'Monthly'),       # Theoretical: 216000, Actual: 200000-225000
    ]
    
    print("Period sizes (in blocks):")
    print("-" * 40)
    
    for delta, period_name in periods:
        # Get blocks for exact time range
        start_time = (now - delta).isoformat().replace('+00:00', 'Z')
        end_time = now.isoformat().replace('+00:00', 'Z')
        
        try:
            start_block, end_block = query.get_blocks_for_time_range(start_time, end_time)
            block_count = end_block - start_block
            avg_block_time = int(delta.total_seconds() / block_count) if block_count > 0 else 0
            print(f"{period_name:10} = {block_count:6} blocks (~{avg_block_time:3}s per block)")
        except Exception as e:
            print(f"{period_name:10} = Error: {e}")


def main():
    """Run all tests."""
    try:
        test_block_time_conversion()
        test_entity_flows_with_time()
        test_aggregation_periods()
        
        print("\n✅ All Time Conversion Tests Complete!")
        
    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()