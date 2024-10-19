from typing import List
from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.data_models.alert_models import ContractCreationAlertData
from ethblockprocessor.data_models.txn_models import TransactionType


class ContractCreationAlert:
    
    def get_alert(self, transaction: DetailedTransaction) -> List[ContractCreationAlertData]:
        if transaction.txn_type == TransactionType.CONTRACT_CREATION.value:
            alert_data = self.create_alert(transaction)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, transaction: DetailedTransaction) -> ContractCreationAlertData:
        contract_address = transaction.erc20_transfers[0].token_address
        pair_address = None
        owner_address = None
        if transaction.pair_events:
            pair_address = transaction.pair_events[0].pair_address
        if transaction.owner_events:
            owner_address = transaction.owner_events[-1].new_owner 

        alert_data = ContractCreationAlertData(
            block_number=transaction.block_number,
            transaction_hash=transaction.hash.hex(),
            creator_address=transaction.from_address,
            contract_address=contract_address,
            details={
                "pair_address": pair_address,
                "owner_address": owner_address,
            }
        )
        return alert_data
    
    def send_alert(self, alert_data: ContractCreationAlertData):
        print(f"Contract creation alert sent: {alert_data}")
