import sys
import os

# Add python path
sys.path.append("/home/nima/code/crypto/eth/pyeth/eth_data")
sys.path.append("/home/nima/code/crypto/eth/pyeth/eth_token")

from eth_data.pyreth_client import PyrethClient

try:
    client = PyrethClient.instance()
    provider = client.processed_tx_provider()
    q = client.chain_query()

    tx_hash2 = "0xe464e002492443d063ae85dfb798c7100898a640dbc55d4737c6117edee0dcf9"
    print(f"Fetching tx {tx_hash2}...")
    ptx2 = provider.processed_transaction_by_hash(tx_hash2)
    
    if not ptx2:
        print("Transaction not found!")
        sys.exit(1)
        
    print(f"Tx found: block={ptx2.block_number}, contract={ptx2.contract_address}, index={ptx2.tx_index}")
    
    sender = ptx2.from_address
    prev_block = ptx2.block_number - 1
    
    print(f"Checking balance of {sender} at {prev_block}...")
    balance = q.get_eth_balance(sender, prev_block)
    print(f"Sender balance at {prev_block}: {balance} wei")
    
    print(f"Fees keys: {ptx2.fees.keys()}")
    gas_limit = int(ptx2.fees.get('gas_limit', 0))
    gas_price = int(ptx2.fees.get('gas_price', 0))
    val = int(ptx2.value)
    required = val + (gas_limit * gas_price)
    
    print(f"Required funds (approx): {required} wei")

    print(f"Attempting get_token_metadata for contract {ptx2.contract_address} at block {prev_block} with pending tx...")

    metadata2 = q.get_token_metadata(
        ptx2.contract_address,  # 0x6E6c09f045f1DdFC05e5cdfA78E441f3AD85a900
        ptx2.block_number - 1,  # 23841212
        [ptx2.to_dict()],
    )
    print("Metadata result:", metadata2)

except Exception as e:
    print("Error caught:", e)
