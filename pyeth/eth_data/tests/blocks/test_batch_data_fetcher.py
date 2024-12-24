import pytest
import json
import asyncio
from web3 import Web3
from eth_block_processor.txn.txn_data_fetcher import BatchTransactionDataFetcher
from eth_block_processor.txn.txn_batch_analyzer import TransactionBatchAnalyzer
from eth_block_processor.blockchain.block_fetcher import BlockFetcher


@pytest.mark.asyncio
async def test_thread_pool_vs_asyncio_consistency():
    """Test that thread pool and asyncio implementations produce identical results"""
    # Setup
    w3 = Web3(Web3.HTTPProvider("http://localhost:8545"))
    analyzer = TransactionBatchAnalyzer(w3)
    block_number = 21061274  # Use a known test block

    # Get block data
    block = await BlockFetcher(w3.provider.endpoint_uri).fetch_block_by_number(block_number)
    assert block is not None, "Failed to fetch test block"

    # Process with thread pool
    thread_pool_results = await analyzer.analyze_block_transactions(
        block_number=block_number,
        transactions=block['transactions'],
        use_asyncio=False
    )

    # Process with asyncio
    asyncio_results = await analyzer.analyze_block_transactions(
        block_number=block_number,
        transactions=block['transactions'],
        use_asyncio=True
    )

    # Compare results
    assert len(thread_pool_results) == len(asyncio_results), \
        f"Result count mismatch: thread pool={len(thread_pool_results)}, asyncio={len(asyncio_results)}"

    for tp_tx, async_tx in zip(thread_pool_results, asyncio_results):
        # Compare key fields
        assert tp_tx.hash == async_tx.hash, f"Hash mismatch for transaction {tp_tx.hash}"
        assert tp_tx.txn_type == async_tx.txn_type, f"Type mismatch for transaction {tp_tx.hash}"
        assert tp_tx.from_address == async_tx.from_address, f"From address mismatch for transaction {tp_tx.hash}"
        assert tp_tx.to_address == async_tx.to_address, f"To address mismatch for transaction {tp_tx.hash}"
        assert tp_tx.value == async_tx.value, f"Value mismatch for transaction {tp_tx.hash}"
        
        # Compare detailed data
        assert len(tp_tx.erc20_transfers) == len(async_tx.erc20_transfers), \
            f"ERC20 transfer count mismatch for transaction {tp_tx.hash}"
        assert len(tp_tx.internal_transactions) == len(async_tx.internal_transactions), \
            f"Internal transaction count mismatch for transaction {tp_tx.hash}"
        assert tp_tx.fees.txn_fee == async_tx.fees.txn_fee, \
            f"Fee mismatch for transaction {tp_tx.hash}"

    print(f"Successfully verified consistency between implementations for {len(thread_pool_results)} transactions")

if __name__ == "__main__":
    asyncio.run(test_thread_pool_vs_asyncio_consistency()) 