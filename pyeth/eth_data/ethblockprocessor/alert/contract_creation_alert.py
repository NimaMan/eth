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

    def classify_contract(self, bytecode: str) -> str:
        # ERC20 function signatures
        erc20_signatures = [
            "0x18160ddd",  # totalSupply()
            "0x70a08231",  # balanceOf(address)
            "0xa9059cbb",  # transfer(address,uint256)
            "0x23b872dd",  # transferFrom(address,address,uint256)
            "0x095ea7b3",  # approve(address,uint256)
            "0xdd62ed3e",  # allowance(address,address)
        ]

        # ERC721 function signatures
        erc721_signatures = [
            "0x70a08231",  # balanceOf(address)
            "0x6352211e",  # ownerOf(uint256)
            "0x42842e0e",  # safeTransferFrom(address,address,uint256)
            "0xb88d4fde",  # safeTransferFrom(address,address,uint256,bytes)
            "0x23b872dd",  # transferFrom(address,address,uint256)
            "0x095ea7b3",  # approve(address,uint256)
            "0xa22cb465",  # setApprovalForAll(address,bool)
            "0xe985e9c5",  # isApprovedForAll(address,address)
        ]

        # Check for ERC20
        if all(sig in bytecode for sig in erc20_signatures):
            return "ERC-20 Token"

        # Check for ERC721
        if all(sig in bytecode for sig in erc721_signatures):
            return "ERC-721 NFT"

        # If no specific type is identified, it's a generic smart contract
        return "Smart Contract"
