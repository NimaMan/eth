import os
import pandas as pd
from tqdm import tqdm
from web3 import Web3
from multiprocessing import Pool
import math

from aladdin3.db.connection_tools import get_session
from aladdin3.db.models_query_methods import load_erc20_token_contract_addresses
from aladdin3.data.erc20_token.erc20_token_data import ERC20TokenData
from aladdin3.data.erc20_token.erc20_token_scam_features import ERC20TokenScamFeatures


def get_bytecode(w3, address):
    return w3.eth.get_code(address).hex()


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
    file_path = os.path.join(file_dir, 'token_scam_results.parquet')
    
    w3 = Web3(Web3.HTTPProvider('http://localhost:8545'))
    with get_session(db='aladdin') as session:
        token_addresses = load_erc20_token_contract_addresses(session)
        
        for i, address in enumerate(tqdm(token_addresses, desc="Processing tokens"), 1):
            token_data = ERC20TokenData(session=session, contract_address=address)
            if token_data.has_uni_v2_pair:
                scam_features = ERC20TokenScamFeatures(
                    metadata=token_data.metadata,
                    price_df=token_data.price_df,
                    mint_df=token_data.mint_df,
                    burn_df=token_data.burn_df
                )
                scam_label, scam_block, scam_txn = scam_features.compute_scam_label()
            else:
                continue
            
            bytecode = get_bytecode(w3, address)
            results.append({
                'address': address,
                'scam_label': scam_label,
                'scam_block': scam_block,
                'scam_txn': scam_txn,
                'creation_block': token_data.metadata.creation_block,
                'creation_txn': token_data.metadata.txn_hash_creation,
                'bytecode': bytecode,
            })
            
            if len(results) % 1000 == 0:
                save_results(results, file_path, mode='a')
                results = []  # Clear the results list after saving
    
    # Save any remaining results
    if results:
        save_results(results, file_path, mode='a')


def process_token_chunk(chunk):
    results = []
    with get_session(db='aladdin') as session:
        w3 = Web3(Web3.HTTPProvider('http://localhost:8545'))
        for address in chunk:
            token_data = ERC20TokenData(session=session, contract_address=address)
            if token_data.has_uni_v2_pair:
                scam_features = ERC20TokenScamFeatures(
                    metadata=token_data.metadata,
                    price_df=token_data.price_df,
                    mint_df=token_data.mint_df,
                    burn_df=token_data.burn_df
                )
                scam_label, scam_block, scam_txn = scam_features.compute_scam_label()
            else:
                continue
            
            bytecode = get_bytecode(w3, address)
            results.append({
                'address': address,
                'scam_label': scam_label,
                'scam_block': scam_block,
                'scam_txn': scam_txn.hex() if scam_txn else None,
                'creation_block': token_data.metadata.creation_block,
                'creation_txn': token_data.metadata.txn_hash_creation,
                'bytecode': bytecode,
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
    main()