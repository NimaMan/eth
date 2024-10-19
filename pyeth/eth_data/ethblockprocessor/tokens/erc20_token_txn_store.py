import lmdb
import json
import os
from eth_block_processor.ethblockprocessor.data_models.txn_models import DetailedTransaction
from eth_block_processor.ethblockprocessor.data_models.erc20_token_txn_model import ERC20TokenTxn


class ERC20TransactionDB:
    def __init__(self, path=None, map_size=10*1024*1024*1024):
        if path is None:
            path = os.path.join(os.environ.get("ETH_DATA_DIR", ""), "erc20_txn_store.lmdb")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.env = lmdb.open(path, map_size=map_size)

    def add_transaction(self, txn: DetailedTransaction):
        erc20_txn = ERC20TokenTxn(
            tx_hash=txn.hash,
            block_number=txn.block_number,
            txn_index=txn.txn_index,
            contract_address=txn.to,
            from_address=txn.from_address,
            value=txn.value
        )
        with self.env.begin(write=True) as txn:
            key = erc20_txn.contract_address.encode('utf-8')
            existing_data = txn.get(key)
            data = json.loads(existing_data.decode('utf-8')) if existing_data else []
            data.append(erc20_txn.to_dict())
            txn.put(key, json.dumps(data).encode('utf-8'))

    def get_transactions(self, contract_address):
        with self.env.begin() as txn:
            data = txn.get(contract_address.encode('utf-8'))
            return json.loads(data.decode('utf-8')) if data else []

    def close(self):
        self.env.close()