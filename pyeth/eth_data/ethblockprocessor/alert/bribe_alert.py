
from web3 import Web3
from aladdin3.utils.addresses import fee_recipients_set
from ethblockprocessor.data_models.txn_models import InternalTransaction


class BribeAlert:
    def __init__(self, w3: Web3, bribe_threshold: float = 0.1):
        self.w3 = w3
        self.fee_recipients_set = fee_recipients_set
        self.bribe_threshold = bribe_threshold

    def check_bribe_amount(self, tx: InternalTransaction):
        if tx.to.lower() in self.fee_recipients_set:
            if tx.value > self.bribe_threshold:
                return True
        return False