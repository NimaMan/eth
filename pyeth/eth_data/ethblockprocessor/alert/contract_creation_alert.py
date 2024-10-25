from typing import List, Union, Optional
from ethblockprocessor.data_models.txn_models import DetailedTransaction
from ethblockprocessor.data_models.alert_models import ContractCreationAlertData
from ethblockprocessor.data_models.txn_models import TransactionType
from ethblockprocessor.utils.logger import get_logger
from web3 import Web3
from web3.exceptions import BadFunctionCallOutput, ContractLogicError


logger = get_logger("contract_creation_alert")


erc20_abi = [
    {"constant":True,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"totalSupply","outputs":[{"name":"","type":"uint256"}],"type":"function"},
    {"constant":True,"inputs":[{"name":"_owner","type":"address"}],"name":"balanceOf","outputs":[{"name":"balance","type":"uint256"}],"type":"function"},
]


def is_erc20(contract_address: str) -> Optional[dict]:
    web3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    contract = web3.eth.contract(address=contract_address, abi=erc20_abi)
    
    try:
        symbol = contract.functions.symbol().call()
        decimals = contract.functions.decimals().call()
        total_supply = contract.functions.totalSupply().call()
        
        return {
            'contract_address': contract_address,
            'symbol': symbol,
            'decimals': decimals,
            'total_supply': total_supply,
        }
    except (BadFunctionCallOutput, ContractLogicError):
        return None
    except Exception as e:
        if "execution reverted" not in str(e):
            logger.error(f"Unexpected error checking ERC-20 compliance: {str(e)}")
        return None


erc721_abi = [
    {"constant":True,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[{"name":"interfaceId","type":"bytes4"}],"name":"supportsInterface","outputs":[{"name":"","type":"bool"}],"type":"function"},
]


def is_erc721(contract_address: str) -> Optional[dict]:
    web3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    contract = web3.eth.contract(address=contract_address, abi=erc721_abi)
    
    try:
        name = contract.functions.name().call()
        symbol = contract.functions.symbol().call()
        supports_interface = contract.functions.supportsInterface('0x80ac58cd').call()  # ERC721 interface id
        
        return {
            'contract_address': contract_address,
            'name': name,
            'symbol': symbol,
            'supports_interface': supports_interface
        }
    except (BadFunctionCallOutput, ContractLogicError):
        return None
    except Exception as e:
        if "execution reverted" not in str(e):
            logger.error(f"Unexpected error checking ERC-721 compliance: {str(e)}")
        return None


class ContractCreationAlert:
    
    def get_alert(self, transaction: DetailedTransaction) -> List[ContractCreationAlertData]:
        if transaction.txn_type == TransactionType.CONTRACT_CREATION.value:
            alert_data = self.create_alert(transaction)
            self.send_alert(alert_data)
            return [alert_data]
        return []
    
    def create_alert(self, transaction: DetailedTransaction) -> ContractCreationAlertData:
        contract_address = transaction.contract_address
        contract_type = self.classify_contract(contract_address, transaction.input)

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
        logger.info(f"{alert_data}")

    def classify_contract(self, contract_address: str, bytecode: Union[str, bytes]) -> str:
        # First, check if it's an ERC-20 token
        erc20_info = is_erc20(contract_address)
        if erc20_info:
            return f"ERC-20 Token: {erc20_info['symbol']}"

        # Then, check if it's an ERC-721 token
        erc721_info = is_erc721(contract_address)
        if erc721_info and erc721_info.get('supports_interface', False):
            return f"ERC-721 NFT: {erc721_info['name']}"

        # If it's neither ERC-20 nor ERC-721, check for other common interfaces
        if isinstance(bytecode, bytes):
            bytecode = bytecode.hex()
        bytecode = bytecode[2:] if bytecode.startswith('0x') else bytecode

        # Check for ERC-1155 interface
        if '0xd9b67a26' in bytecode:  # ERC-1155 interface id
            return "ERC-1155 Token"

        # Add more checks for other interfaces as needed

        return "Smart Contract"

    
