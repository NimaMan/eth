import asyncio
from dataclasses import asdict
from typing import Dict, Tuple

from web3 import Web3
from pyreth import block_processor

from eth_data.reth_chain_query.reth_index.address_tx_history import RethAddressTxHistory
from eth_token.erc20_token.erc20_token import ERC20Token

NODE_URL = "http://127.0.0.1:8545"
CONTRACT_ADDRESS = "0xeEcd819eab1D44e358EC4aE464678B9956e3ce8E"


async def build_token(contract_address: str) -> Tuple[ERC20Token, Dict[int, str]]:
    """Instantiate an ERC20Token by replaying all related transactions."""
    w3 = Web3(Web3.HTTPProvider(NODE_URL))
    if not w3.is_connected():
        raise RuntimeError(f"Unable to connect to Ethereum node at {NODE_URL}")

    processor = block_processor()
    address_history = RethAddressTxHistory()
    token = ERC20Token(contract_address=contract_address)
    token_block_coverage: Dict[int, str] = {}

    token_tx_records = address_history.get_transactions(contract_address)
    block_numbers = sorted({record.block_number for record in token_tx_records})
    print(f"Found {len(block_numbers)} blocks touching {contract_address}")

    for block_number in block_numbers:
        processed_block = await asyncio.to_thread(processor.process_block, block_number)
        for tx in processed_block.transactions:
            pool_addresses = set(token.pool_addresses)
            unique_addrs = set(tx.unique_addresses)
            if contract_address in unique_addrs or unique_addrs & pool_addresses:
                token_block_coverage[tx.block_number] = tx.hash
                tx_payload = tx.to_dict()
                token.update_from_transaction(tx_payload)

    return token, token_block_coverage


async def main() -> None:
    token, coverage = await build_token(CONTRACT_ADDRESS)
    print(f"Processed {len(coverage)} relevant transactions")

    if not len(token.pools):
        print("No pools discovered for token.")
        return

    for pool in token.pools:
        print(f"Pool {pool.display_address} trading enabled tx: {pool.trading_enabled_tx}")


if __name__ == "__main__":
    asyncio.run(main())
