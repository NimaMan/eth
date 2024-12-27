import os
import pandas as pd
from tqdm import tqdm
from web3 import Web3
from multiprocessing import Pool
import math

from aladdin3.db.connection_tools import get_session
from aladdin3.db.models_query_methods import load_erc20_token_contract_addresses
from aladdin3.db.queries.transaction import load_transactions_by_contract_address


def get_bytecode(w3, address):
    return w3.eth.get_code(address).hex()


def get_contract_related_txn_input_data(w3, transactions):
    txn_input_data = []
    for tx in transactions:
        try:
            full_tx = w3.eth.get_transaction(tx.hash)
            txn_input_data.append(full_tx['input'].hex())
        except Exception as e:
            print(f"Error fetching transaction {tx.hash_hex()}: {e}")
    return txn_input_data


def save_results(results, file_path, mode='w'):
    df = pd.DataFrame(results)
    if mode == 'w' or not os.path.exists(file_path):
        df.to_parquet(file_path, index=False)
    else:
        existing_df = pd.read_parquet(file_path)
        updated_df = pd.concat([existing_df, df], ignore_index=True)
        updated_df.to_parquet(file_path, index=False)


def main():
    results = []
    file_dir = os.path.dirname(os.path.abspath(__file__))
    file_path = os.path.join(file_dir, 'token_code_and_txn_data.parquet')
    
    w3 = Web3(Web3.HTTPProvider(NODE_URL))
    with get_session(db='aladdin') as session:
        token_addresses = load_erc20_token_contract_addresses(session)
        
        for i, address in enumerate(tqdm(token_addresses, desc="Processing tokens"), 1):
            all_transactions = load_transactions_by_contract_address(session, address)
            
            bytecode = get_bytecode(w3, address)
            results.append({
                'address': address,
                'bytecode': bytecode,
                'related_txn_input_data': get_contract_related_txn_input_data(w3, all_transactions),
                'all_transactions': [tx.hash.hex() for tx in all_transactions],
            })
            
            if len(results) % 1000 == 0:
                save_results(results, file_path, mode='a')
                results = []  # Clear the results list after saving
    
    # Save any remaining results
    if results:
        save_results(results, file_path, mode='a')


def process_token_chunk(chunk):
    w3 = Web3(Web3.HTTPProvider(NODE_URL))
    results = []
    with get_session(db='aladdin') as session:
        for address in chunk:
            all_transactions = load_transactions_by_contract_address(session, address)
            bytecode = get_bytecode(w3, address)
            results.append({
                'address': address,
                'bytecode': bytecode,
                'related_txn_input_data': get_contract_related_txn_input_data(w3, all_transactions),
                'all_transactions': [tx.hash.hex() for tx in all_transactions],
            })
    return results


def main_parallel():
    file_dir = os.path.dirname(os.path.abspath(__file__))
    file_path = os.path.join(file_dir, 'token_code_and_txn_data.parquet')
    
    with get_session(db='aladdin') as session:
        token_addresses = load_erc20_token_contract_addresses(session)
    
    # Split token addresses into chunks
    num_processes = 4
    chunk_size = math.ceil(len(token_addresses) / num_processes)
    chunks = [token_addresses[i:i + chunk_size] for i in range(0, len(token_addresses), chunk_size)]
    
    # Create a pool of workers
    with Pool(num_processes) as pool:
        results = list(tqdm(pool.imap(process_token_chunk, chunks), total=len(chunks), desc="Processing chunks"))
    
    # Flatten results and save
    all_results = [item for sublist in results for item in sublist]
    save_results(all_results, file_path)


if __name__ == "__main__":
    NODE_URL = "http://localhost:8545"
    main()