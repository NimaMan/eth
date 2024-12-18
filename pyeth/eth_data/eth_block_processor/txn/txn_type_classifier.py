from hexbytes import HexBytes
from typing import Dict, Any, Union
from web3 import Web3
from eth_block_processor.constants.function_signatures import FUNCTION_SIGNATURES

class ContractInteractionClassifier:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.known_functions = FUNCTION_SIGNATURES  
        
        self.known_contracts = {
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D": "Uniswap V2: Router 2",
            "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45": "Uniswap V3: Router 2",
            "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD": "Aave: Lending Pool V3",
            # Add more known contract addresses here
        }

    def classify_interaction(self, transaction: Dict[str, Any]) -> str:
        input_data = transaction.get('input')
        to_address = transaction.get('to', '').lower()

        if isinstance(input_data, HexBytes):
            function_signature = input_data[:4].hex()
        elif isinstance(input_data, str):
            function_signature = input_data[:10]
        
        if function_signature in self.known_functions:
            return self.known_functions[function_signature]
        
        if to_address in self.known_contracts:
            return self.known_contracts[to_address]

        return "Contract Interaction"


class EthTransactionClassifier:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.erc20_transfer_signature = self.w3.keccak(text="transfer(address,uint256)").hex()[:8] # No 0x is returned
        self.erc721_transfer_signature = self.w3.keccak(text="transferFrom(address,address,uint256)").hex()[:8] 
        self.erc1155_transfer_signature = self.w3.keccak(text="safeTransferFrom(address,address,uint256,uint256,bytes)").hex()[:8]
        self.approve_signature = self.w3.keccak(text="approve(address,uint256)").hex()[:8]
        self.contract_interaction_classifier = ContractInteractionClassifier(w3=w3)

    def classify_transaction(self, transaction: Dict[str, Any]) -> str:
        if transaction.get('to') is None:
            return "Contract Creation"
        else:
            input_data = transaction.get('input')
            if input_data.hex() == '0x' or input_data.hex() == '':
                return "Ether Transfer"
            elif self._starts_with(input_data, self.erc20_transfer_signature):
                return "ERC20 Transfer"
            elif self._starts_with(input_data, self.erc721_transfer_signature):
                return "ERC721 Transfer"
            elif self._starts_with(input_data, self.erc1155_transfer_signature):
                return "ERC1155 Transfer"
            elif self._starts_with(input_data, self.approve_signature):
                return "Approval"
            else:
                return self.contract_interaction_classifier.classify_interaction(transaction)

    def _starts_with(self, input_data: Union[HexBytes, str], prefix: str) -> bool:
        if isinstance(input_data, HexBytes):
            return input_data.hex().startswith(prefix)
        elif isinstance(input_data, str):
            return input_data.startswith(prefix)
        else:
            raise TypeError(f"Unsupported input type: {type(input_data)}")