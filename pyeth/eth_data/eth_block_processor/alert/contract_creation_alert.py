from typing import List, Optional
from dataclasses import asdict
from eth_block_processor.data_models.txn_models import DetailedTransaction
from eth_block_processor.data_models.alert_models import ContractCreationAlertData
from eth_block_processor.alert.base_alert_class import BaseAlert
from eth_block_processor.utils.logger import get_logger
from eth_block_processor.contracts.contract_type import classify_contract


logger = get_logger("contract_creation_alert", log_folder="alert")


class ContractCreationAlert(BaseAlert):
    
    def _is_alert(self, detailed_txn: DetailedTransaction) -> bool:
        """Check if transaction is a contract creation"""
        return detailed_txn.contract_address is not None and detailed_txn.txn_type == "Contract Creation"

    async def process_txn(self, detailed_txn: DetailedTransaction) -> List[ContractCreationAlertData]:
        """Process a transaction to detect contract creation events"""
        if self._is_alert(detailed_txn):
            alert_data = self.create_alert(detailed_txn)
            self.send_alert(alert_data)
            return [alert_data]
        return []

    def create_alert(self, detailed_txn: DetailedTransaction) -> ContractCreationAlertData:
        """Create a contract creation alert"""
        contract_type = self._classify_contract(detailed_txn.contract_address)
        
        alert_data = ContractCreationAlertData(
            block_number=detailed_txn.block_number,
            transaction_hash=detailed_txn.hash,
            creator_address=detailed_txn.from_address,
            contract_address=detailed_txn.contract_address,
            contract_type=contract_type,
            input=detailed_txn.input
        )
        return alert_data

    def send_alert(self, alert_data: ContractCreationAlertData):
        """Log the alert without the 'input' field"""
       # Create a copy of alert data without input field for logging
        log_data = asdict(alert_data)
        if 'input' in log_data:
            log_data['input'] = '[REDACTED]'
        logger.info(f"Contract Creation Alert: {log_data}")

    def _classify_contract(self, contract_address: str) -> str:
        """Classify the type of contract"""
        return classify_contract(contract_address)