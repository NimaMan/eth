#!/usr/bin/env python3
"""
EVM Mining Data Collector

This module collects mining timestamps and block inclusion data for transactions
to analyze the EVM as a priority queue system. It processes exported transaction
data from the Rust service and correlates it with actual mining outcomes.

OBJECTIVES:
1. Collect mining timestamps for EVM queue time calculation
2. Analyze gas price effectiveness in priority queue
3. Measure block inclusion patterns and ordering
4. Calculate EVM queue performance metrics

ALGORITHM:
1. Monitor for exported JSON batches from Rust service
2. For each transaction, query Ethereum node for mining data
3. Calculate gas price percentiles and priority effectiveness
4. Generate EVM queue analytics and insights
5. Export enhanced data with mining correlation
"""

import asyncio
import json
import pandas as pd
import numpy as np
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from datetime import datetime, timedelta
import aiohttp
import time
import logging
from concurrent.futures import ThreadPoolExecutor

# Set up logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class MiningData:
    """Mining outcome data for a transaction."""
    tx_hash: str
    mining_timestamp_ms: Optional[int]
    block_number: Optional[int] 
    transaction_index: Optional[int]
    gas_used: Optional[int]
    effective_gas_price_wei: Optional[int]
    status: str  # "success", "failed", "pending", "dropped"
    
@dataclass 
class BlockAnalysis:
    """Gas price analysis for a block."""
    block_number: int
    transaction_count: int
    gas_prices: List[int]  # All gas prices in block, sorted descending
    gas_price_percentiles: Dict[int, float]  # gas_price -> percentile
    average_gas_price: float
    median_gas_price: float
    min_gas_price: int
    max_gas_price: int

@dataclass
class EVMQueueMetrics:
    """EVM queue performance metrics."""
    total_transactions: int
    mined_transactions: int
    pending_transactions: int
    dropped_transactions: int
    average_queue_time_seconds: float
    median_queue_time_seconds: float
    gas_price_correlation: float  # Correlation between gas price and queue time
    priority_queue_effectiveness: float  # How well gas price predicts mining speed

class EVMMiningCollector:
    """Collects and analyzes EVM mining data for queue analysis."""
    
    def __init__(self, eth_rpc_url: str = "http://localhost:8545"):
        self.eth_rpc_url = eth_rpc_url
        self.session = None
        self.batch_export_dir = Path("/home/nima/code/crypto/logs/mempool/")
        self.output_dir = Path("/home/nima/code/crypto/logs/mempool/evm_analysis/")
        self.output_dir.mkdir(exist_ok=True)
        
        # Processing state
        self.processed_batches = set()
        self.mining_cache = {}  # tx_hash -> MiningData
        self.block_cache = {}   # block_number -> BlockAnalysis
        
        # Statistics
        self.total_processed = 0
        self.successful_mining_lookups = 0
        self.failed_mining_lookups = 0
        
    async def __aenter__(self):
        """Async context manager entry."""
        self.session = aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=30),
            connector=aiohttp.TCPConnector(limit=100)
        )
        return self
        
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        if self.session:
            await self.session.close()
            
    async def monitor_and_process(self, check_interval: float = 30.0):
        """
        Main monitoring loop that processes exported batches from Rust service.
        
        Args:
            check_interval: How often to check for new batches (seconds)
        """
        logger.info("🔍 Starting EVM mining data collection monitoring...")
        logger.info(f"📂 Monitoring directory: {self.batch_export_dir}")
        logger.info(f"📊 Output directory: {self.output_dir}")
        
        while True:
            try:
                await self.process_new_batches()
                await asyncio.sleep(check_interval)
            except KeyboardInterrupt:
                logger.info("⏹️  Stopping monitoring due to user interrupt")
                break
            except Exception as e:
                logger.error(f"❌ Error in monitoring loop: {e}")
                await asyncio.sleep(check_interval)
                
    async def process_new_batches(self):
        """Find and process new transaction batches exported by Rust service."""
        # Look for new JSON export files
        pattern = "evm_batch_export_*.json"
        batch_files = list(self.batch_export_dir.glob(pattern))
        
        new_batches = [f for f in batch_files if f.name not in self.processed_batches]
        
        if new_batches:
            logger.info(f"📥 Found {len(new_batches)} new batch(es) to process")
            
            for batch_file in sorted(new_batches):
                try:
                    await self.process_batch_file(batch_file)
                    self.processed_batches.add(batch_file.name)
                except Exception as e:
                    logger.error(f"❌ Failed to process batch {batch_file.name}: {e}")
                    
    async def process_batch_file(self, batch_file: Path):
        """Process a single batch file of transactions."""
        logger.info(f"🔄 Processing batch: {batch_file.name}")
        
        # Load transaction data
        try:
            with open(batch_file, 'r') as f:
                transactions = json.load(f)
        except json.JSONDecodeError as e:
            logger.error(f"❌ Invalid JSON in {batch_file.name}: {e}")
            return
            
        if not transactions:
            logger.warning(f"⚠️  Empty batch file: {batch_file.name}")
            return
            
        logger.info(f"📊 Processing {len(transactions)} transactions from batch")
        
        # Collect mining data for all transactions
        mining_tasks = [
            self.get_mining_data(tx['tx_hash']) 
            for tx in transactions
        ]
        
        # Process in batches to avoid overwhelming the RPC
        batch_size = 50
        mining_results = []
        
        for i in range(0, len(mining_tasks), batch_size):
            batch_tasks = mining_tasks[i:i+batch_size]
            batch_results = await asyncio.gather(*batch_tasks, return_exceptions=True)
            
            for result in batch_results:
                if isinstance(result, Exception):
                    logger.warning(f"⚠️  Mining lookup failed: {result}")
                    mining_results.append(None)
                else:
                    mining_results.append(result)
                    
            # Small delay between batches
            await asyncio.sleep(0.1)
            
        # Enhance transaction data with mining information
        enhanced_transactions = []
        for tx, mining_data in zip(transactions, mining_results):
            enhanced_tx = tx.copy()
            
            if mining_data:
                enhanced_tx.update({
                    'mining_timestamp_ms': mining_data.mining_timestamp_ms,
                    'block_number': mining_data.block_number,
                    'transaction_index_in_block': mining_data.transaction_index,
                    'gas_used': mining_data.gas_used,
                    'effective_gas_price_wei': str(mining_data.effective_gas_price_wei) if mining_data.effective_gas_price_wei else None,
                    'mining_status': mining_data.status,
                })
                
                # Calculate EVM queue time
                if mining_data.mining_timestamp_ms and enhanced_tx.get('estimated_mempool_entry_ms'):
                    evm_queue_time = mining_data.mining_timestamp_ms - enhanced_tx['estimated_mempool_entry_ms']
                    enhanced_tx['evm_queue_time_ms'] = evm_queue_time
                    
            enhanced_transactions.append(enhanced_tx)
            
        # Analyze gas price effectiveness for mined transactions
        await self.analyze_gas_price_effectiveness(enhanced_transactions)
        
        # Export enhanced data
        await self.export_enhanced_data(enhanced_transactions, batch_file.stem)
        
        self.total_processed += len(transactions)
        logger.info(f"✅ Processed batch {batch_file.name}: {len(transactions)} transactions")
        
    async def get_mining_data(self, tx_hash: str) -> Optional[MiningData]:
        """Get mining data for a specific transaction."""
        # Check cache first
        if tx_hash in self.mining_cache:
            return self.mining_cache[tx_hash]
            
        try:
            # Get transaction receipt
            receipt_payload = {
                "jsonrpc": "2.0",
                "method": "eth_getTransactionReceipt", 
                "params": [tx_hash],
                "id": 1
            }
            
            async with self.session.post(self.eth_rpc_url, json=receipt_payload) as response:
                receipt_data = await response.json()
                
            receipt = receipt_data.get('result')
            if not receipt:
                # Transaction not mined yet or dropped
                mining_data = MiningData(
                    tx_hash=tx_hash,
                    mining_timestamp_ms=None,
                    block_number=None,
                    transaction_index=None,
                    gas_used=None,
                    effective_gas_price_wei=None,
                    status="pending"
                )
                self.failed_mining_lookups += 1
                return mining_data
                
            # Get block timestamp
            block_number = int(receipt['blockNumber'], 16)
            block_payload = {
                "jsonrpc": "2.0",
                "method": "eth_getBlockByNumber",
                "params": [hex(block_number), False],
                "id": 2
            }
            
            async with self.session.post(self.eth_rpc_url, json=block_payload) as response:
                block_data = await response.json()
                
            block = block_data.get('result')
            if not block:
                logger.warning(f"⚠️  Could not get block data for block {block_number}")
                return None
                
            # Extract mining data
            mining_timestamp_ms = int(block['timestamp'], 16) * 1000  # Convert to milliseconds
            status = "success" if receipt.get('status') == '0x1' else "failed"
            
            mining_data = MiningData(
                tx_hash=tx_hash,
                mining_timestamp_ms=mining_timestamp_ms,
                block_number=block_number,
                transaction_index=int(receipt['transactionIndex'], 16),
                gas_used=int(receipt['gasUsed'], 16),
                effective_gas_price_wei=int(receipt.get('effectiveGasPrice', '0x0'), 16),
                status=status
            )
            
            # Cache the result
            self.mining_cache[tx_hash] = mining_data
            self.successful_mining_lookups += 1
            
            return mining_data
            
        except Exception as e:
            logger.warning(f"⚠️  Failed to get mining data for {tx_hash}: {e}")
            self.failed_mining_lookups += 1
            return None
            
    async def analyze_gas_price_effectiveness(self, transactions: List[Dict]):
        """Analyze how effective gas price is for priority in the queue."""
        # Group transactions by block for gas price analysis
        blocks = {}
        for tx in transactions:
            if tx.get('block_number') and tx.get('mining_status') == 'success':
                block_num = tx['block_number']
                if block_num not in blocks:
                    blocks[block_num] = []
                blocks[block_num].append(tx)
                
        # Analyze each block's gas price ordering
        for block_number, block_txs in blocks.items():
            if len(block_txs) < 2:
                continue  # Need multiple transactions for ordering analysis
                
            # Sort by transaction index (actual mining order)
            block_txs.sort(key=lambda x: x.get('transaction_index_in_block', 0))
            
            # Extract gas prices 
            gas_prices = []
            for tx in block_txs:
                try:
                    gas_price = int(tx.get('gas_price_wei', '0'))
                    gas_prices.append(gas_price)
                except (ValueError, TypeError):
                    gas_prices.append(0)
                    
            # Calculate gas price percentiles within block
            for i, tx in enumerate(block_txs):
                our_gas_price = gas_prices[i]
                higher_prices = [p for p in gas_prices if p > our_gas_price]
                percentile = (len(gas_prices) - len(higher_prices)) / len(gas_prices) * 100
                tx['gas_price_percentile_in_block'] = percentile
                
            # Store block analysis
            if gas_prices:
                self.block_cache[block_number] = BlockAnalysis(
                    block_number=block_number,
                    transaction_count=len(block_txs),
                    gas_prices=sorted(gas_prices, reverse=True),
                    gas_price_percentiles={},  # Will be filled if needed
                    average_gas_price=np.mean(gas_prices),
                    median_gas_price=np.median(gas_prices),
                    min_gas_price=min(gas_prices),
                    max_gas_price=max(gas_prices)
                )
                
    async def export_enhanced_data(self, transactions: List[Dict], batch_name: str):
        """Export enhanced transaction data with mining correlation."""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_file = self.output_dir / f"evm_enhanced_{batch_name}_{timestamp}.json"
        
        # Calculate summary statistics
        mined_txs = [tx for tx in transactions if tx.get('mining_status') == 'success']
        pending_txs = [tx for tx in transactions if tx.get('mining_status') == 'pending']
        failed_txs = [tx for tx in transactions if tx.get('mining_status') == 'failed']
        
        summary = {
            'batch_info': {
                'batch_name': batch_name,
                'processing_timestamp': timestamp,
                'total_transactions': len(transactions),
                'mined_transactions': len(mined_txs),
                'pending_transactions': len(pending_txs),
                'failed_transactions': len(failed_txs),
            },
            'evm_queue_metrics': self.calculate_evm_queue_metrics(mined_txs),
            'transactions': transactions
        }
        
        # Export to JSON
        with open(output_file, 'w') as f:
            json.dump(summary, f, indent=2)
            
        logger.info(f"📤 Exported enhanced data: {output_file}")
        
        # Also export CSV for analysis
        csv_file = output_file.with_suffix('.csv')
        self.export_csv_analysis(transactions, csv_file)
        
    def calculate_evm_queue_metrics(self, mined_transactions: List[Dict]) -> Dict:
        """Calculate EVM queue performance metrics."""
        if not mined_transactions:
            return {}
            
        # Extract queue times (mempool entry to mining)
        queue_times = []
        gas_prices = []
        
        for tx in mined_transactions:
            if tx.get('evm_queue_time_ms') is not None:
                queue_time_seconds = tx['evm_queue_time_ms'] / 1000.0
                queue_times.append(queue_time_seconds)
                
                try:
                    gas_price = int(tx.get('gas_price_wei', '0'))
                    gas_prices.append(gas_price)
                except (ValueError, TypeError):
                    gas_prices.append(0)
                    
        if not queue_times:
            return {}
            
        # Calculate correlation between gas price and queue time
        gas_price_correlation = 0.0
        if len(queue_times) > 1 and len(gas_prices) == len(queue_times):
            correlation_matrix = np.corrcoef(gas_prices, queue_times)
            gas_price_correlation = correlation_matrix[0, 1] if not np.isnan(correlation_matrix[0, 1]) else 0.0
            
        # Calculate priority queue effectiveness
        # Higher gas price should correlate with lower queue time (negative correlation)
        priority_effectiveness = max(0, -gas_price_correlation * 100)  # Convert to percentage
        
        return {
            'average_queue_time_seconds': float(np.mean(queue_times)),
            'median_queue_time_seconds': float(np.median(queue_times)),
            'min_queue_time_seconds': float(min(queue_times)),
            'max_queue_time_seconds': float(max(queue_times)),
            'gas_price_correlation': float(gas_price_correlation),
            'priority_queue_effectiveness_pct': float(priority_effectiveness),
            'total_analyzed_transactions': len(queue_times)
        }
        
    def export_csv_analysis(self, transactions: List[Dict], csv_file: Path):
        """Export transaction data as CSV for spreadsheet analysis."""
        # Flatten transaction data for CSV
        csv_data = []
        for tx in transactions:
            row = {
                'tx_hash': tx.get('tx_hash', ''),
                'estimated_mempool_entry_ms': tx.get('estimated_mempool_entry_ms', ''),
                'mining_timestamp_ms': tx.get('mining_timestamp_ms', ''),
                'evm_queue_time_ms': tx.get('evm_queue_time_ms', ''),
                'evm_queue_time_seconds': tx.get('evm_queue_time_ms', 0) / 1000.0 if tx.get('evm_queue_time_ms') else '',
                'gas_price_wei': tx.get('gas_price_wei', ''),
                'gas_price_gwei': int(tx.get('gas_price_wei', '0')) / 1e9 if tx.get('gas_price_wei') else '',
                'gas_limit': tx.get('gas_limit', ''),
                'from_address': tx.get('from_address', ''),
                'to_address': tx.get('to_address', ''),
                'tx_value_wei': tx.get('tx_value_wei', ''),
                'block_number': tx.get('block_number', ''),
                'transaction_index_in_block': tx.get('transaction_index_in_block', ''),
                'gas_used': tx.get('gas_used', ''),
                'effective_gas_price_wei': tx.get('effective_gas_price_wei', ''),
                'mining_status': tx.get('mining_status', ''),
                'gas_price_percentile_in_block': tx.get('gas_price_percentile_in_block', ''),
                'our_processing_time_us': tx.get('our_processing_time_us', ''),
                'our_queue_time_us': tx.get('our_queue_time_us', ''),
            }
            csv_data.append(row)
            
        # Export to CSV
        df = pd.DataFrame(csv_data)
        df.to_csv(csv_file, index=False)
        logger.info(f"📊 Exported CSV analysis: {csv_file}")
        
    def get_statistics(self) -> Dict:
        """Get collection statistics."""
        return {
            'total_processed': self.total_processed,
            'successful_mining_lookups': self.successful_mining_lookups,
            'failed_mining_lookups': self.failed_mining_lookups,
            'mining_cache_size': len(self.mining_cache),
            'block_cache_size': len(self.block_cache),
            'success_rate_pct': (
                self.successful_mining_lookups / 
                (self.successful_mining_lookups + self.failed_mining_lookups) * 100
                if (self.successful_mining_lookups + self.failed_mining_lookups) > 0 else 0
            )
        }

async def main():
    """Main execution function."""
    logger.info("🚀 Starting EVM Mining Data Collector")
    
    async with EVMMiningCollector() as collector:
        try:
            await collector.monitor_and_process(check_interval=30.0)
        except KeyboardInterrupt:
            logger.info("⏹️  Stopping due to user interrupt")
        finally:
            stats = collector.get_statistics()
            logger.info("📊 Final Statistics:")
            for key, value in stats.items():
                logger.info(f"   {key}: {value}")

if __name__ == "__main__":
    asyncio.run(main())