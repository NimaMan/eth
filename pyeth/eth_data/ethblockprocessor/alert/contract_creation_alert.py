from typing import List, Union
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
        contract_address = transaction.contract_address
        contract_type = self.classify_contract(transaction.input)

        alert_data = ContractCreationAlertData(
            block_number=transaction.block_number,
            transaction_hash=transaction.hash.hex(),
            creator_address=transaction.from_address,
            contract_address=contract_address,
            details={
                "contract_type": contract_type,
            }
        )
        return alert_data
    
    def send_alert(self, alert_data: ContractCreationAlertData):
        print(f"Contract creation alert sent: {alert_data}")

    def classify_contract(self, bytecode: Union[str, bytes]) -> str:
        # Convert bytecode to string if it's bytes
        if isinstance(bytecode, bytes):
            bytecode = bytecode.hex()

        # Remove '0x' prefix if present
        bytecode = bytecode[2:] if bytecode.startswith('0x') else bytecode

        # ERC20 function signatures (common subset)
        erc20_signatures = [
            "18160ddd",  # totalSupply()
            "70a08231",  # balanceOf(address)
            "a9059cbb",  # transfer(address,uint256)
            "23b872dd",  # transferFrom(address,address,uint256)
        ]

        # ERC721 function signatures (common subset)
        erc721_signatures = [
            "70a08231",  # balanceOf(address)
            "6352211e",  # ownerOf(uint256)
            "42842e0e",  # safeTransferFrom(address,address,uint256)
        ]

        # Count the number of ERC20 and ERC721 signatures present
        erc20_count = sum(1 for sig in erc20_signatures if sig in bytecode)
        erc721_count = sum(1 for sig in erc721_signatures if sig in bytecode)

        # Classify based on the number of signatures found
        if erc20_count >= 3:  # At least 3 out of 4 ERC20 signatures
            return "ERC-20 Token"
        elif erc721_count >= 2:  # At least 2 out of 3 ERC721 signatures
            return "ERC-721 NFT"
        else:
            return "Smart Contract"
