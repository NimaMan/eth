import lmdb
import json
import os
from eth_block_processor.data_models.txn_models import DetailedTransaction
from typing import List


class ERC20TransactionDB:
    def __init__(self, path=None, map_size=10*1024*1024*1024):
        if path is None:
            path = os.path.join(os.environ.get("ETH_ADDRESS_DIR", ""), "erc20_txn_store.lmdb")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.path = path
        self.map_size = map_size
        self.env = None
        self._open_env()

    def _open_env(self):
        try:
            if self.env:
                self.env.close()
            self.env = lmdb.open(self.path, map_size=self.map_size)
        except lmdb.Error as e:
            print(f"Error opening LMDB environment: {e}")
            raise

    def _ensure_env_open(self):
        try:
            # Try a simple operation to check if the environment is usable
            with self.env.begin() as txn:
                txn.get(b'test')
        except (lmdb.Error, AttributeError):
            # If there's an error, reopen the environment
            self._open_env()

    def add_transaction(self, detailed_txn: DetailedTransaction, max_retries=3):
        for attempt in range(max_retries):
            try:
                self._ensure_env_open()
                with self.env.begin(write=True) as db_txn:
                    # Use the token contract address as the primary key
                    token_addresses = set()
                    token_addresses.update(detailed_txn.erc20_contracts)
                    for approval in detailed_txn.approvals:
                        token_addresses.add(approval.token_address)
                    
                    for token_address in token_addresses:
                        key = token_address.encode('utf-8')
                        existing_data = db_txn.get(key)
                        
                        txn_data = {
                            'txn_hash': detailed_txn.hash.hex(),
                            'block_number': detailed_txn.block_number,
                            'txn_index': detailed_txn.txn_index,
                            'from_address': detailed_txn.from_address,
                            'value': str(detailed_txn.value),  # Convert to string to ensure JSON serialization
                            'txn_type': detailed_txn.txn_type,
                        }
                        
                        if existing_data:
                            data = json.loads(existing_data.decode('utf-8'))
                            # Check if transaction already exists
                            if not any(txn['txn_hash'] == txn_data['txn_hash'] for txn in data):
                                data.append(txn_data)
                        else:
                            data = [txn_data]
                        
                        db_txn.put(key, json.dumps(data).encode('utf-8'))
                return
            except lmdb.Error as e:
                print(f"LMDB Error (attempt {attempt + 1}/{max_retries}): {e}")
                self._open_env()
                if attempt == max_retries - 1:
                    raise

    def get_transactions(self, contract_address):
        self._ensure_env_open()
        try:
            with self.env.begin() as txn:
                data = txn.get(contract_address.encode('utf-8'))
                return json.loads(data.decode('utf-8')) if data else []
        except lmdb.Error as e:
            print(f"LMDB Error in get_transactions: {e}")
            self._open_env()
            return []

    def reset_database(self):
        self.close()
        if os.path.exists(self.path):
            os.remove(self.path)
        if os.path.exists(self.path + '-lock'):
            os.remove(self.path + '-lock')
        self._open_env()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

    def close(self):
        if self.env:
            self.env.close()
            self.env = None

    def add_transactions_batch(self, detailed_txns: List[DetailedTransaction]):
        """Batch insert ERC20 transactions"""
        values = []
        for txn in detailed_txns:
            if self._should_store_transaction(txn):
                values.append(self._prepare_transaction_data(txn))
                
        if values:
            with self.engine.connect() as conn:
                conn.execute(
                    self.table.insert(),
                    values
                )
                conn.commit()
    
    def _should_store_transaction(self, txn: DetailedTransaction) -> bool:
        return (txn.txn_type == 'ERC20_TRANSFER' or
                len(txn.erc20_contracts) > 0 or
                len(txn.approvals) > 0 or
                any(len(getattr(txn, attr)) > 0 for attr in 
                    ['uniswap_v2_syncs', 'uniswap_v2_swaps', 
                     'mints', 'burns', 'deposits', 'withdraws']))
