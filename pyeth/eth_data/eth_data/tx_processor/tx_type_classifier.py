from hexbytes import HexBytes
from typing import Dict, Any, Union, Optional
from web3 import Web3
from eth_data.chain_utils.function_signatures import FUNCTION_SIGNATURES, UNISWAP_CONTRACTS


class ContractInteractionClassifier:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.known_functions = FUNCTION_SIGNATURES      
        self.known_contracts = UNISWAP_CONTRACTS

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
        
        input_data = transaction.get('input')
        if self.is_eth_transfer(input_data):
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

    def is_eth_transfer(self, input_data: Union[HexBytes, str]) -> bool:
        if isinstance(input_data, HexBytes):
            if input_data.hex() == '0x' or input_data.hex() == '':
                return True
        elif isinstance(input_data, str):
            if input_data == '0x' or input_data == '':
                return True
        return False
    
    def _starts_with(self, input_data: Union[HexBytes, str], prefix: str) -> bool:
        if isinstance(input_data, HexBytes):
            return input_data.hex().startswith(prefix)
        elif isinstance(input_data, str):
            return input_data.startswith(prefix)
        else:
            raise TypeError(f"Unsupported input type: {type(input_data)}")


class EthProtocolTypeClassifier:
    """
    Classifies Ethereum transactions by their protocol type (Legacy, EIP-1559, etc.)
    This is crucial for gas ranking as different types have different fee structures.
    """
    
    def classify_protocol_type(self, transaction: Dict[str, Any]) -> str:
        """
        Determines the Ethereum protocol type of a transaction.
        
        Returns:
            - "legacy": Traditional transactions with single gas price
            - "eip1559": EIP-1559 transactions with base fee + priority fee
            - "eip2930": Access list transactions (rarely used)
            - "unknown": Cannot determine type
        """
        tx_type = transaction.get('type')
        
        # Type can be hex string or int
        if isinstance(tx_type, str):
            if tx_type.startswith('0x'):
                tx_type = int(tx_type, 16)
            else:
                tx_type = int(tx_type)
        
        # EIP-2718 transaction types
        if tx_type == 0:
            return "legacy"
        elif tx_type == 1:
            return "eip2930"  # Access list transactions
        elif tx_type == 2:
            return "eip1559"  # Dynamic fee transactions
        
        # Fallback: check for presence of EIP-1559 fields
        if 'maxFeePerGas' in transaction and 'maxPriorityFeePerGas' in transaction:
            return "eip1559"
        elif 'gasPrice' in transaction:
            return "legacy"
        
        return "unknown"
    
    def get_gas_fields(self, transaction: Dict[str, Any], receipt: Dict[str, Any]) -> Dict[str, Optional[int]]:
        """
        Extracts all gas-related fields needed for ranking.
        
        Args:
            transaction: Transaction object from eth_getTransactionByHash
            receipt: Receipt object from eth_getTransactionReceipt
            
        Returns:
            Dictionary with:
            - protocol_type: Transaction protocol type
            - gas_limit: Gas limit set by user
            - gas_used: Actual gas consumed
            - effective_gas_price: Price per gas actually paid
            - max_fee_per_gas: User's max willingness (if EIP-1559)
            - max_priority_fee: User's max tip (if EIP-1559)
        """
        protocol_type = self.classify_protocol_type(transaction)
        
        # Common fields
        gas_fields = {
            'protocol_type': protocol_type,
            'gas_limit': int(transaction.get('gas', 0), 16) if isinstance(transaction.get('gas'), str) else transaction.get('gas', 0),
            'gas_used': int(receipt.get('gasUsed', 0), 16) if isinstance(receipt.get('gasUsed'), str) else receipt.get('gasUsed', 0),
            'effective_gas_price': int(receipt.get('effectiveGasPrice', 0), 16) if isinstance(receipt.get('effectiveGasPrice'), str) else receipt.get('effectiveGasPrice', 0),
        }
        
        # Protocol-specific fields
        if protocol_type == "eip1559":
            # EIP-1559 transaction
            max_fee = transaction.get('maxFeePerGas')
            max_priority = transaction.get('maxPriorityFeePerGas')
            
            gas_fields['max_fee_per_gas'] = int(max_fee, 16) if isinstance(max_fee, str) else max_fee
            gas_fields['max_priority_fee'] = int(max_priority, 16) if isinstance(max_priority, str) else max_priority
                
        elif protocol_type == "legacy":
            # Legacy transaction
            gas_price = transaction.get('gasPrice')
            gas_fields['gas_price'] = int(gas_price, 16) if isinstance(gas_price, str) else gas_price
            gas_fields['max_fee_per_gas'] = gas_fields['gas_price']  # Same as gas price for legacy
            gas_fields['max_priority_fee'] = None
        
        return gas_fields