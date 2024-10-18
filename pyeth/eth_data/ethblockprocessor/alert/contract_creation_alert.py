from typing import List
from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.data_models.alert_models import ContractCreationAlertData


class ContractCreationAlert:
    
    def get_alerts(self, transaction: DetailedTransaction) -> List[ContractCreationAlertData]:
        if transaction.txn_type == "create":
            alert_data = self.create_alert(transaction)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, transaction: DetailedTransaction) -> ContractCreationAlertData:
        
        alert_data = ContractCreationAlertData(
            block_number=transaction.block_number,
            transaction_hash=transaction.hash,
            alert_type="Contract Creation",
            details={
                "from_address": transaction.from_address,
                "to_address": transaction.to_address,
                "contract_address": transaction.contract_address,
            }
        )
        return alert_data
    
    def send_alert(self, alert_data: ContractCreationAlertData):
        print(f"Contract creation alert sent: {alert_data}")
