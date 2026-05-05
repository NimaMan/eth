from eth_token.erc20_token.token_state.token_transfer_tracker import TokenTransferTracker


def test_token_transfer_tracker_accepts_hex_transfer_amounts():
    token_address = "0x0B2E1a4cCF71B1879E5a81ed31211220263E36f0"
    tracker = TokenTransferTracker(
        contract_address=token_address,
        decimals=0,
        history_limit=10,
    )
    tx_hash = "0x95ed837256ba6af23fa6a543e71a6f5e6c8447731ccd096b38f9759c602bca12"

    tracker.update_from_transaction(
        {
            "hash": tx_hash,
            "block_number": 25000000,
            "tx_index": 1,
            "unique_addresses": [],
            "bribe_amount": "0x2",
            "from_address": "0x2222222222222222222222222222222222222222",
            "internal_transactions": [
                {
                    "depth": 0,
                    "from_address": "0x2222222222222222222222222222222222222222",
                    "to_address": "0x1111111111111111111111111111111111111111",
                    "value": "0x10",
                }
            ],
            "approvals": [],
            "erc20_transfers": [
                {
                    "token_address": token_address,
                    "from_address": "0x0000000000000000000000000000000000000000",
                    "to_address": "0x1111111111111111111111111111111111111111",
                    "amount": "0x87b5",
                    "log_index": 0,
                }
            ],
        }
    )

    assert tracker.erc20_transfers[tx_hash][0]["amount"] == 0x87B5
    assert tracker.eth_transfers[tx_hash][0]["amount"] == 0x10
    assert tracker.total_bribe_amount == 0x2
    assert tracker.total_supply_from_transfers == 0x87B5
