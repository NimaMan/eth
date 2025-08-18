from eth_data.tx_processor.tx_processor import TransactionProcessor
import pytest
import json
import asyncio
from web3 import Web3
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_batch_processor import TransactionBatchProcessor
from eth_data.blockchain.block_fetcher import BlockFetcher


@pytest.mark.asyncio
async def test_batch_processor_with_regular_processor_consistency():
    """Test that batch processor with regular processor produces identical results"""
    # Setup
    w3 = Web3(Web3.HTTPProvider("http://localhost:8545"))
    batch_processor = TransactionBatchProcessor(w3)
    regular_processor = TransactionProcessor(w3)
    txn_data_fetcher = TransactionDataFetcher(w3)
    block_number = 21061274  # Use a known test block

    # Get block data
    block = await BlockFetcher(w3.provider.endpoint_uri).fetch_block_by_number(block_number)
    assert block is not None, "Failed to fetch test block"

    # Process with asyncio
    batch_processor_results = await batch_processor.process_block_transactions(
        block_number=block_number,
        transactions=block['transactions'],
    )

    # Process with regular processor
    regular_processor_results = []
    for tx in batch_processor_results:
        tx_hash = tx.hash
        tx_data = txn_data_fetcher.get_transaction_data(tx_hash)
        regular_processor_results.append(regular_processor.process_transaction(tx_data['transaction'], tx_data['receipt'], tx_data['trace']))
    
    for tp_tx, async_tx in zip(batch_processor_results, regular_processor_results):
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

    print(f"Successfully verified consistency between implementations for {len(batch_processor_results)} transactions")


if __name__ == "__main__":
    asyncio.run(test_batch_processor_with_regular_processor_consistency()) 