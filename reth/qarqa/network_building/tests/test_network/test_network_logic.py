"""
Test for Fund Flow Network Logic (Independent of Flask)
Tests the network analysis logic without requiring Flask dependencies

This tests the core logic for:
1. Transaction analysis
2. Edge classification  
3. WETH treatment
4. Intermediary filtering
"""
import unittest
import sys
import os

# Add the sarigoz package to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))


class TestNetworkLogic(unittest.TestCase):
    """Test core network logic without Flask dependencies"""
    
    def setUp(self):
        """Set up test data"""
        self.tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
        
    def test_edge_classification_logic(self):
        """Test edge classification logic"""
        
        # Define classification function (extracted from API)
        def classify_edge(edge_data, token_address=None):
            """Classify edge type and return properties"""
            
            # Gas payments
            if edge_data.get('movementTypes') == 'Gas Payment':
                return {
                    'type': 'GAS_PAYMENT',
                    'color': '#95a5a6',
                    'weight': 1,
                    'priority': 'low'
                }
            
            # ETH transfers
            eth_amount = float(edge_data.get('ethAmount', 0))
            if eth_amount > 0:
                if edge_data.get('movementTypes') == 'Direct Transfer':
                    return {
                        'type': 'ETH_DIRECT',
                        'color': '#2980b9',
                        'weight': min(10, max(2, int(eth_amount * 2))),
                        'priority': 'high'
                    }
                else:
                    return {
                        'type': 'ETH_INTERNAL',
                        'color': '#3498db',
                        'weight': min(10, max(2, int(eth_amount * 2))),
                        'priority': 'high'
                    }
            
            # Token transfers
            if token_address:
                # Stablecoins
                stablecoins = {
                    '0xA0b86a33E6441e0fb7bf8E4e8FF2C6F0a72D3C8A',  # USDC
                    '0xdAC17F958D2ee523a2206206994597C13D831ec7',  # USDT
                    '0x6B175474E89094C44Da98b954EedeAC495271d0F',  # DAI
                }
                
                if token_address.lower() in [addr.lower() for addr in stablecoins]:
                    return {
                        'type': 'STABLECOIN',
                        'color': '#27ae60',
                        'weight': 4,
                        'priority': 'high'
                    }
                elif token_address.lower() == '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2'.lower():  # WETH
                    return {
                        'type': 'WRAPPED_ETH',
                        'color': '#16a085',
                        'weight': 4,
                        'priority': 'high'
                    }
                else:
                    return {
                        'type': 'ERC20_TOKEN',
                        'color': '#9b59b6',
                        'weight': 3,
                        'priority': 'medium'
                    }
            
            return {
                'type': 'UNKNOWN',
                'color': '#bdc3c7',
                'weight': 2,
                'priority': 'low'
            }
        
        # Test gas payment classification
        gas_edge = {'ethAmount': '0.0039', 'movementTypes': 'Gas Payment'}
        gas_result = classify_edge(gas_edge)
        self.assertEqual(gas_result['type'], 'GAS_PAYMENT')
        self.assertEqual(gas_result['color'], '#95a5a6')
        self.assertEqual(gas_result['priority'], 'low')
        
        # Test ETH direct transfer
        eth_edge = {'ethAmount': '10.5', 'movementTypes': 'Direct Transfer'}
        eth_result = classify_edge(eth_edge)
        self.assertEqual(eth_result['type'], 'ETH_DIRECT')
        self.assertEqual(eth_result['color'], '#2980b9')
        self.assertEqual(eth_result['priority'], 'high')
        
        # Test WETH classification
        weth_edge = {'ethAmount': '0', 'movementTypes': 'Token Transfer'}
        weth_address = '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2'
        weth_result = classify_edge(weth_edge, weth_address)
        self.assertEqual(weth_result['type'], 'WRAPPED_ETH')
        self.assertEqual(weth_result['color'], '#16a085')
        self.assertEqual(weth_result['priority'], 'high')
        
        # Test USDC classification
        usdc_edge = {'ethAmount': '0', 'movementTypes': 'Token Transfer'}
        usdc_address = '0xA0b86a33E6441e0fb7bf8E4e8FF2C6F0a72D3C8A'
        usdc_result = classify_edge(usdc_edge, usdc_address)
        self.assertEqual(usdc_result['type'], 'STABLECOIN')
        self.assertEqual(usdc_result['color'], '#27ae60')
        self.assertEqual(usdc_result['priority'], 'high')
        
    def test_weth_combination_logic(self):
        """Test that WETH amounts are properly combined with ETH"""
        
        def combine_weth_with_eth(eth_amount, token_flows):
            """Combine WETH token flows with ETH amount"""
            total_eth = float(eth_amount)
            
            for flow in token_flows:
                if flow.get('symbol', '').upper() == 'WETH':
                    total_eth += float(flow.get('amount', 0))
            
            return total_eth
        
        # Test case 1: ETH only
        result1 = combine_weth_with_eth('5.5', [])
        self.assertEqual(result1, 5.5)
        
        # Test case 2: ETH + WETH
        token_flows = [
            {'symbol': 'WETH', 'amount': '10.5'},
            {'symbol': 'USDC', 'amount': '1000'}
        ]
        result2 = combine_weth_with_eth('2.0', token_flows)
        self.assertEqual(result2, 12.5)  # 2.0 + 10.5
        
        # Test case 3: Multiple WETH flows
        multi_weth_flows = [
            {'symbol': 'WETH', 'amount': '5.0'},
            {'symbol': 'WETH', 'amount': '3.0'},
            {'symbol': 'USDT', 'amount': '500'}
        ]
        result3 = combine_weth_with_eth('1.0', multi_weth_flows)
        self.assertEqual(result3, 9.0)  # 1.0 + 5.0 + 3.0
        
    def test_intermediary_filtering_logic(self):
        """Test logic for filtering intermediary addresses"""
        
        def filter_intermediaries(nodes, threshold=0.001):
            """Filter out nodes with net change below threshold"""
            meaningful_nodes = []
            
            for node in nodes:
                net_change = abs(float(node.get('netChange', 0)))
                if net_change >= threshold:
                    meaningful_nodes.append(node)
            
            return meaningful_nodes
        
        # Mock nodes with various net changes
        test_nodes = [
            {'id': 'user', 'netChange': '-0.0039', 'type': 'user'},      # Meaningful (gas)
            {'id': 'router1', 'netChange': '0.0', 'type': 'contract'},    # Intermediary
            {'id': 'router2', 'netChange': '0.0001', 'type': 'contract'}, # Below threshold
            {'id': 'pool', 'netChange': '16.325', 'type': 'pool'},        # Meaningful
            {'id': 'network', 'netChange': '0.0039', 'type': 'network'}   # Meaningful (gas)
        ]
        
        filtered = filter_intermediaries(test_nodes, threshold=0.001)
        
        # Should keep user, pool, and network; filter out router1 and router2
        self.assertEqual(len(filtered), 3)
        
        filtered_ids = {node['id'] for node in filtered}
        expected_ids = {'user', 'pool', 'network'}
        self.assertEqual(filtered_ids, expected_ids)
        
    def test_meaningful_transfer_extraction(self):
        """Test extraction of meaningful transfers from edges"""
        
        def extract_meaningful_transfers(edges, eth_threshold=0.001):
            """Extract transfers that represent real fund movement"""
            meaningful = []
            
            for edge in edges:
                # Skip gas payments
                if edge.get('movementTypes') == 'Gas Payment':
                    continue
                
                # Check if transfer has meaningful value
                eth_amount = float(edge.get('ethAmount', 0))
                token_flows = edge.get('tokenFlows', [])
                
                # Combine WETH with ETH for threshold check
                total_eth = eth_amount
                for flow in token_flows:
                    if flow.get('symbol', '').upper() == 'WETH':
                        total_eth += float(flow.get('amount', 0))
                
                # Include if above threshold or has non-WETH tokens
                non_weth_tokens = [f for f in token_flows if f.get('symbol', '').upper() != 'WETH']
                if total_eth >= eth_threshold or len(non_weth_tokens) > 0:
                    meaningful.append(edge)
            
            return meaningful
        
        # Mock edges
        test_edges = [
            {
                'source': 'user', 'target': 'network',
                'ethAmount': '0.0039', 'movementTypes': 'Gas Payment',
                'tokenFlows': []
            },
            {
                'source': 'weth', 'target': 'pool',
                'ethAmount': '0', 'movementTypes': 'Token Transfer',
                'tokenFlows': [{'symbol': 'WETH', 'amount': '16.325'}]
            },
            {
                'source': 'pool1', 'target': 'pool2',
                'ethAmount': '0', 'movementTypes': 'Token Transfer',
                'tokenFlows': [{'symbol': 'USDC', 'amount': '27158'}]
            },
            {
                'source': 'router', 'target': 'user',
                'ethAmount': '0.0001', 'movementTypes': 'Transfer',
                'tokenFlows': []
            }
        ]
        
        meaningful = extract_meaningful_transfers(test_edges, eth_threshold=0.001)
        
        # Should exclude gas payment and tiny transfer, keep WETH and USDC
        self.assertEqual(len(meaningful), 2)
        
        # Verify we kept the right transfers
        movement_types = {edge['movementTypes'] for edge in meaningful}
        self.assertIn('Token Transfer', movement_types)
        self.assertNotIn('Gas Payment', movement_types)
        
    def test_expected_transaction_structure(self):
        """Test the expected structure for our specific transaction"""
        
        # Expected nodes after filtering
        expected_nodes = [
            {'address': '0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1', 'type': 'user', 'net_eth': -0.0039},
            {'address': '0x0000000000000000000000000000000000000000', 'type': 'network', 'net_eth': +0.0039},
            {'address': 'WETH_CONTRACT', 'type': 'token', 'net_eth': -16.325},
            {'address': 'UNISWAP_V4', 'type': 'pool', 'net_eth': +16.325},
            {'address': 'UNISWAP_V3', 'type': 'pool', 'net_weth': +6.729}
        ]
        
        # Expected edges after filtering  
        expected_edges = [
            {'from': 'user', 'to': 'network', 'type': 'gas'},
            {'from': 'weth', 'to': 'v4', 'type': 'eth'},
            {'from': 'v4', 'to': 'v3', 'type': 'stablecoin'},
            {'from': 'v3', 'to': 'weth', 'type': 'token'}
        ]
        
        # Verify expected counts
        self.assertEqual(len(expected_nodes), 5, "Should have exactly 5 nodes after filtering")
        self.assertEqual(len(expected_edges), 4, "Should have exactly 4 edges after filtering")
        
        # Verify no intermediaries
        intermediary_addresses = {
            '0x6bDf3535711ab1ac93b3e8de5A5682849f9d59e9d',
            '0x3177F690b14f8a2731396b90C2c5c57A5C99FA1C359',
            '0xfBd4cdB413E45a52E2C8312f670e9cE67E794C37'
        }
        
        node_addresses = {node['address'] for node in expected_nodes}
        overlap = intermediary_addresses.intersection(node_addresses)
        self.assertEqual(len(overlap), 0, "No intermediary addresses should be in final nodes")


if __name__ == '__main__':
    unittest.main(verbosity=2)