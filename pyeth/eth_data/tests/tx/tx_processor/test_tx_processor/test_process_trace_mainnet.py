from eth_data.tx_processor.tx_trace_processor import TransactionTraceProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def _collect_internal_transactions(processor, trace, receipt):
    return processor.process_trace(
        trace,
        receipt_contract_address=receipt.get("contractAddress"),
    )


def test_trace_processor_failed_contract_creation(w3, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0x8304000190747e7f8ace1510304ada10d66ec24e05427dc8f994ddbea3b97d6c"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    processor = TransactionTraceProcessor(w3)
    internals = _collect_internal_transactions(processor, tx_data["trace"], tx_data["receipt"])

    assert internals, "Expected internal transactions from failed contract creation"
    assert any(tx.trace_type == "CREATE" and tx.error for tx in internals)


def test_trace_processor_contract_creation_address_resolution(w3, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0x45fbb2326ee70cbaacb56c12b6a14b2ab5efd41635e9d3ba9ff4fed4eee52b89"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    processor = TransactionTraceProcessor(w3)
    internals = _collect_internal_transactions(processor, tx_data["trace"], tx_data["receipt"])

    contract_address = tx_data["receipt"]["contractAddress"]
    assert any(
        tx.trace_type == "CREATE" and tx.to_address == contract_address
        for tx in internals
    )
