import unittest
import asyncio
import time
from unittest.mock import Mock, patch
from web3 import Web3
from eth_block_processor.blockchain.block_txn_processor import BlockTxnProcessor
from eth_block_processor.data_models.txn_models import DetailedTransaction, TransactionType


TEST_BLOCK = 21142465  

class TestBlockTxnProcessor(unittest.TestCase):
    def setUp(self):
        self.w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.processor = BlockTxnProcessor(w3=self.w3)

    async def test_process_latest_block(self):
        """Test processing of the most recent block"""
        latest_block = self.w3.eth.block_number
        start_time = time.perf_counter()
        
        results = await self.processor.process_transactions(
            self.w3.eth.get_block(latest_block, full_transactions=True)
        )
        
        processing_time = time.perf_counter() - start_time
        
        self.assertIsNotNone(results)
        print(f"Latest block processed in {processing_time:.4f} seconds")
        
        # Verify transaction analysis
        for tx_hash, analysis in results.items():
            self.assertIsInstance(analysis, DetailedTransaction)
            self.assertTrue(hasattr(analysis, 'txn_type'))
            self.assertTrue(hasattr(analysis, 'block_number'))

    async def test_process_specific_block(self):
        """Test processing of a specific historical block"""
        # Choose a block known to have different types of transactions
        
        block = self.w3.eth.get_block(TEST_BLOCK, full_transactions=True)
        results = await self.processor.process_transactions(block)
        
        self.assertEqual(len(results), len(block['transactions']))
        
        # Verify block details
        for tx_hash, analysis in results.items():
            self.assertEqual(analysis.block_number, TEST_BLOCK)

    async def test_process_block_with_different_txn_types(self):
        """Test processing blocks with various transaction types"""
        # Process a block known to have different transaction types
        block = self.w3.eth.get_block(TEST_BLOCK, full_transactions=True)
        results = await self.processor.process_transactions(block)
        
        # Track different transaction types found
        txn_types = set()
        for tx_hash, analysis in results.items():
            txn_types.add(analysis.txn_type)
        
        # Verify we found different types of transactions
        self.assertTrue(len(txn_types) > 1)
        print(f"Transaction types found: {txn_types}")

    async def test_parallel_block_processing(self):
        """Test processing multiple blocks in parallel"""
        start_block = TEST_BLOCK
        num_blocks = 5
        
        blocks = [
            self.w3.eth.get_block(start_block + i, full_transactions=True)
            for i in range(num_blocks)
        ]
        
        start_time = time.perf_counter()
        tasks = [self.processor.process_transactions(block) for block in blocks]
        results = await asyncio.gather(*tasks)
        processing_time = time.perf_counter() - start_time
        
        print(f"Processed {num_blocks} blocks in {processing_time:.4f} seconds")
        self.assertEqual(len(results), num_blocks)

    async def test_error_handling(self):
        """Test error handling for invalid blocks"""
        with self.assertRaises(Exception):
            await self.processor.process_transactions(None)
            
        with self.assertRaises(Exception):
            # Try to process a non-existent block
            invalid_block = self.w3.eth.get_block(999999999, full_transactions=True)
            await self.processor.process_transactions(invalid_block)

    async def test_transaction_details(self):
        """Test detailed transaction analysis"""
        block = self.w3.eth.get_block('latest', full_transactions=True)
        results = await self.processor.process_transactions(block)
        
        for tx_hash, analysis in results.items():
            # Verify transaction details
            self.assertIsNotNone(analysis.from_address)
            self.assertIsNotNone(analysis.hash)
            self.assertIsNotNone(analysis.block_number)
            self.assertIsNotNone(analysis.txn_index)
            
            # Verify fees
            self.assertTrue(analysis.fees.gas_price > 0)
            self.assertTrue(analysis.fees.gas_used > 0)
            self.assertTrue(analysis.fees.total_fee > 0)

    def test_performance_metrics(self):
        """Test performance metrics for block processing"""
        block_sizes = [1, 10, 50, 100]  # Number of transactions
        metrics = {}
        
        for size in block_sizes:
            block = self.w3.eth.get_block(
                self.w3.eth.block_number - size, 
                full_transactions=True
            )
            
            start_time = time.perf_counter()
            asyncio.run(self.processor.process_transactions(block))
            processing_time = time.perf_counter() - start_time
            
            metrics[size] = processing_time
            
        for size, time_taken in metrics.items():
            print(f"Block with {size} transactions processed in {time_taken:.4f} seconds")

def run_async_tests():
    # Helper function to run async tests
    loop = asyncio.get_event_loop()
    test_cases = [
        TestBlockTxnProcessor('test_process_latest_block'),
        TestBlockTxnProcessor('test_process_specific_block'),
        TestBlockTxnProcessor('test_process_block_with_different_txn_types'),
        TestBlockTxnProcessor('test_parallel_block_processing'),
        TestBlockTxnProcessor('test_error_handling'),
        TestBlockTxnProcessor('test_transaction_details'),
    ]
    
    for test in test_cases:
        loop.run_until_complete(test)


if __name__ == '__main__':        
    unittest.main()
