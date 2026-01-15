from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.data_models.trace_models import InternalTransaction


def test_extend_unique_addresses_normalizes_and_filters(w3):
    processor = TransactionProcessor(w3=w3)

    from_address = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"  # USDC, lowercase
    to_address = "0xdac17f958d2ee523a2206206994597c13d831ec7"  # USDT, lowercase
    erc20_contracts = {
        "0x6b175474e89094c44da98b954eedeac495271d0f",  # DAI, lowercase
        "0xc02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",  # WETH, mixed case
        "notanaddress",
    }

    internal_transactions = [
        InternalTransaction(
            from_address=w3.to_checksum_address("0x111111125434b319222cdbf8c261674adb56f3ae"),
            to_address=w3.to_checksum_address("0x11111112542d85b3ef69ae05771c2dccff4faa26"),
            value=1,
            gas=0,
            gas_used=0,
            depth=0,
            trace_type="call",
            call_type=None,
            error=None,
        )
    ]

    unique_addresses = {None}
    normalized_contracts, participants = processor.extend_unique_addresses(
        from_address=from_address,
        to_address=to_address,
        internal_transactions=internal_transactions,
        unique_addresses=unique_addresses,
        erc20_contracts=erc20_contracts,
        contract_address="0x00000000219ab540356cbb839cbe05303d7705fa",
    )

    assert None not in participants
    assert all(w3.is_checksum_address(addr) for addr in participants)
    assert w3.to_checksum_address(from_address) in participants
    assert w3.to_checksum_address(to_address) in participants
    assert w3.to_checksum_address("0x6b175474e89094c44da98b954eedeac495271d0f") in normalized_contracts
    assert w3.to_checksum_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2") not in normalized_contracts
    assert "notanaddress" not in participants
