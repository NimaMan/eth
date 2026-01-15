#!/usr/bin/env python3
"""
Test script for FundFlowNetwork Python bindings
"""

import fundflownetwork_py as ffn
import json

def test_basic_usage():
    """Test basic fund flow network building."""
    print("🚀 Testing FundFlowNetwork Python bindings...")
    
    # Initialize the builder
    builder = ffn.PyFundFlowNetworkBuilder(
        database_url="postgresql://postgres:postgres@localhost:5432/eth_db"
    )
    
    # Example address (Vitalik's address)
    seed_address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
    
    # Build network from seed address
    print(f"\n📊 Building fund flow network from {seed_address}...")
    network = builder.build_from_address(
        seed_address=seed_address,
        max_depth=2,
        min_value_eth=1.0,
        max_nodes=100,
        include_tokens=True
    )
    
    # Print network statistics
    if 'stats' in network:
        print(f"✅ Network built: {network['stats']['total_nodes']} nodes, {network['stats']['total_edges']} edges")
    
    # Get insights for the address
    print(f"\n🔍 Getting fund flow insights...")
    insights = builder.get_fund_flow_insights(
        address=seed_address,
        lookback_blocks=10000
    )
    
    print(f"  Upstream sources: {len(insights['upstream_sources'])}")
    print(f"  Downstream sinks: {len(insights['downstream_sinks'])}")
    print(f"  Net flow: {insights['net_flow_eth']} ETH")
    
    # Export to different formats
    print(f"\n📤 Exporting network...")
    
    # Cytoscape format (default)
    cytoscape_data = builder.export_network(network, format="cytoscape")
    print(f"  Cytoscape format: {len(str(cytoscape_data))} bytes")
    
    # GraphML format
    graphml_data = builder.export_network(network, format="graphml")
    print(f"  GraphML format: {len(str(graphml_data))} bytes")
    
    print("\n✅ All tests passed!")
    return network

if __name__ == "__main__":
    try:
        network = test_basic_usage()
        
        # Save to file for visualization
        with open("fundflow_network.json", "w") as f:
            json.dump(network, f, indent=2)
        print(f"\n💾 Network saved to fundflow_network.json")
        
    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()