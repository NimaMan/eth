#!/usr/bin/env python3
"""
Integration test for pool level communication between Python and Rust.

This script demonstrates the complete data flow from Python to Rust:
1. Simulates token updates with pool data
2. Publishes via PoolLevelPublisher
3. Verifies Rust PoolSubscriber receives the data

Run this alongside the Rust pool subscriber test.
"""

import asyncio
import sys
import os
import time
import json
from typing import Dict, Any

# Add the Python module to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '../../../../py'))

from eth_portfolio_manager.pool_level import PoolLevelExtractor, PoolLevelPublisher
from eth_portfolio_manager.utils.logger import get_logger

# Mock token data class
class MockTokenData:
    def __init__(self, address: str, symbol: str, pools: Dict[str, Dict[str, Any]]):
        self.address = address
        self.symbol = symbol
        self.name = f"{symbol} Token"
        self.is_scam = False
        self.trading_enabled = True
        self.pool_addresses = list(pools.keys())
        self._pools = pools
        
    def get_pool_reserve(self, pool_address: str) -> float:
        """Get ETH reserve for a pool."""
        pool = self._pools.get(pool_address, {})
        return pool.get('eth_reserve', 0.0)
    
    def get_pool_token_reserve(self, pool_address: str) -> float:
        """Get token reserve for a pool."""
        pool = self._pools.get(pool_address, {})
        return pool.get('token_reserve', 0.0)
    
    def get_pool_info_dict(self) -> Dict[str, Dict[str, Any]]:
        """Get pool information dictionary."""
        result = {}
        for pool_addr, pool_data in self._pools.items():
            result[pool_addr] = {
                'pool_type': pool_data.get('pool_type', 'V2'),
                'fee': pool_data.get('fee', 3000),
                'denom_currency': 'WETH',
                'denom_address': '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
                'decimals': 18,
                'token1_is_denom': True,
            }
            # Add V4 specific fields
            if pool_data.get('pool_type') == 'V4':
                result[pool_addr]['pool_id'] = pool_data.get('pool_id', '')
                result[pool_addr]['pool_manager'] = pool_data.get('pool_manager', '')
        return result

class MockToken:
    def __init__(self, token_data: MockTokenData):
        self.token_data = token_data

async def simulate_pool_updates():
    """Simulate pool level updates from blockchain processing."""
    logger = get_logger("integration_test")
    
    # Initialize components
    extractor = PoolLevelExtractor(logger=logger)
    publisher = PoolLevelPublisher(logger=logger)
    
    try:
        # Start the publisher
        await publisher.start()
        logger.info("Pool level publisher started")
        
        # Wait a bit for connections
        await asyncio.sleep(1)
        
        # Simulate initial token updates
        logger.info("Simulating initial pool updates...")
        
        # Create mock tokens with various pool configurations
        tokens = {
            # High liquidity token with V2 pool
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": MockToken(
                MockTokenData(
                    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                    "USDC",
                    {
                        "0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f": {
                            "eth_reserve": 125.67,
                            "token_reserve": 250000.0,
                            "pool_type": "V2"
                        }
                    }
                )
            ),
            
            # Medium liquidity token with V3 pool
            "0x6B175474E89094C44Da98b954EedeAC495271d0F": MockToken(
                MockTokenData(
                    "0x6B175474E89094C44Da98b954EedeAC495271d0F",
                    "DAI",
                    {
                        "0xC2e9F25Be6257c210d7Adf0D4Cd6E3E881ba2580": {
                            "eth_reserve": 45.32,
                            "token_reserve": 90000.0,
                            "pool_type": "V3",
                            "fee": 500
                        }
                    }
                )
            ),
            
            # Low liquidity scam token
            "0xScamToken111111111111111111111111111111": MockToken(
                MockTokenData(
                    "0xScamToken111111111111111111111111111111",
                    "SCAM",
                    {
                        "0xScamPool111111111111111111111111111111": {
                            "eth_reserve": 0.5,
                            "token_reserve": 1000000000.0,
                            "pool_type": "V2"
                        }
                    }
                )
            ),
            
            # V4 pool example
            "0xV4Token11111111111111111111111111111111": MockToken(
                MockTokenData(
                    "0xV4Token11111111111111111111111111111111",
                    "V4TK",
                    {
                        "0x000000000004444c5dc75cb358380d2e3de08a90#0x1234": {
                            "eth_reserve": 20.0,
                            "token_reserve": 40000.0,
                            "pool_type": "V4",
                            "pool_id": "0x1234",
                            "pool_manager": "0x000000000004444c5dc75cb358380d2e3de08a90"
                        }
                    }
                )
            )
        }
        
        # Send initial update
        block_number = 19000000
        updated_pools = await extractor.update_pool_levels(tokens, block_number)
        await publisher.update_pool_levels(updated_pools)
        
        logger.info(f"Published {len(updated_pools)} initial pool updates")
        for pool_addr, pool_data in updated_pools.items():
            logger.info(f"  - {pool_addr}: {pool_data['eth_reserve']:.2f} ETH ({pool_data['pool_type']})")
        
        # Wait for processing
        await asyncio.sleep(2)
        
        # Simulate liquidity changes
        logger.info("\nSimulating liquidity changes...")
        
        # Increase USDC pool liquidity
        tokens["0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"].token_data._pools[
            "0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f"
        ]["eth_reserve"] = 150.0
        
        # Decrease DAI pool liquidity
        tokens["0x6B175474E89094C44Da98b954EedeAC495271d0F"].token_data._pools[
            "0xC2e9F25Be6257c210d7Adf0D4Cd6E3E881ba2580"
        ]["eth_reserve"] = 30.0
        
        # Rug pull on scam token
        tokens["0xScamToken111111111111111111111111111111"].token_data._pools[
            "0xScamPool111111111111111111111111111111"
        ]["eth_reserve"] = 0.01
        
        block_number += 1
        updated_pools = await extractor.update_pool_levels(tokens, block_number)
        await publisher.update_pool_levels(updated_pools)
        
        logger.info(f"Published {len(updated_pools)} pool updates after changes")
        
        # Test REQ/REP interface
        logger.info("\nTesting request/reply interface...")
        
        # Let the Rust side make some requests
        await asyncio.sleep(5)
        
        # Add more pools to test capacity
        logger.info("\nAdding multiple pools to test capacity...")
        
        for i in range(10):
            token_addr = f"0xToken{i:040x}"
            pool_addr = f"0xPool{i:040x}"
            
            tokens[token_addr] = MockToken(
                MockTokenData(
                    token_addr,
                    f"TK{i}",
                    {
                        pool_addr: {
                            "eth_reserve": 10.0 + i,
                            "token_reserve": 20000.0 + i * 1000,
                            "pool_type": "V2"
                        }
                    }
                )
            )
        
        block_number += 1
        updated_pools = await extractor.update_pool_levels(tokens, block_number)
        await publisher.update_pool_levels(updated_pools)
        
        logger.info(f"Published {len(updated_pools)} additional pools")
        
        # Keep running for a while to allow testing
        logger.info("\nIntegration test running. Press Ctrl+C to stop...")
        
        while True:
            # Periodically update a random pool
            await asyncio.sleep(5)
            
            # Simulate a small change
            if "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" in tokens:
                current = tokens["0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"].token_data._pools[
                    "0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f"
                ]["eth_reserve"]
                new_value = current * (1 + (0.01 if current < 200 else -0.01))
                tokens["0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"].token_data._pools[
                    "0x3041CbD36888bECc7bbCBc0045E3B1f144466f5f"
                ]["eth_reserve"] = new_value
                
                block_number += 1
                updated = await extractor.update_pool_levels(
                    {"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": tokens["0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"]},
                    block_number
                )
                await publisher.update_pool_levels(updated)
                logger.info(f"Updated USDC pool: {new_value:.2f} ETH")
                
    except KeyboardInterrupt:
        logger.info("\nShutting down...")
    except Exception as e:
        logger.error(f"Error in integration test: {e}")
        raise
    finally:
        await publisher.stop()
        logger.info("Integration test completed")

if __name__ == "__main__":
    # Set up logging
    import logging
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    )
    
    # Run the test
    asyncio.run(simulate_pool_updates())