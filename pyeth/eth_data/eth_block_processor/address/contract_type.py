from web3 import Web3
from typing import Optional
from web3.exceptions import BadFunctionCallOutput, ContractLogicError


erc20_abi = [
    {"constant":True,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"totalSupply","outputs":[{"name":"","type":"uint256"}],"type":"function"},
    {"constant":True,"inputs":[{"name":"_owner","type":"address"}],"name":"balanceOf","outputs":[{"name":"balance","type":"uint256"}],"type":"function"},
]


def get_erc20_contract_info(contract_address: str, w3: Web3 = None) -> Optional[dict]:
    """
    Attempts to identify if a contract is an ERC-20 token and returns its information.
    
    This function calls standard ERC-20 methods on the contract to determine if it
    implements the ERC-20 interface. It handles various error conditions that might
    occur when interacting with non-ERC-20 contracts.
    
    Args:
        contract_address: The Ethereum contract address to check
        w3: Optional Web3 instance (creates one with local provider if None)
        
    Returns:
        Dictionary with token info if ERC-20, None otherwise
    """
    if w3 is None:
        w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    contract = w3.eth.contract(address=w3.to_checksum_address(contract_address), abi=erc20_abi)
    try:
        # Set explicit gas limit to prevent out-of-gas errors
        call_params = {'gas': 100000}  # 100k gas should be more than enough for view functions
        
        symbol = contract.functions.symbol().call(call_params)
        decimals = contract.functions.decimals().call(call_params)
        total_supply = contract.functions.totalSupply().call(call_params)/10**decimals
        name = contract.functions.name().call(call_params)
        
        return {
            'contract_address': contract_address,
            'name': name,
            'symbol': symbol,
            'decimals': decimals,
            'total_supply': total_supply,
        }
    except (BadFunctionCallOutput, ContractLogicError) as e:
        # Contract is not an ERC20 token or has invalid bytecode
        return None
    except Exception as e:
        error_str = str(e)
        # Common EVM errors that indicate the contract is not an ERC-20 token
        evm_errors = [
            "execution reverted", 
            "InvalidFEOpcode", 
            "InvalidJump",
            "EVM error: InvalidJump",
            "StackUnderflow",
            "EVM error: StackUnderflow",
            "out of gas",  # Add out of gas as a recognized error
            "gas required exceeds allowance"  # Add gas limit error as recognized
        ]
        
        # If the error is one of the expected EVM errors, treat it as "not an ERC-20 token"
        if any(err in error_str for err in evm_errors):
            return None
            
        # For unexpected errors, raise with more context
        raise Exception(f"Unexpected error checking ERC-20 compliance: {error_str}")


def is_erc20_contract(contract_address: str, w3: Web3 = None) -> Optional[dict]:
    return get_erc20_contract_info(contract_address, w3) is not None


erc721_abi = [
    {"constant":True,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[{"name":"interfaceId","type":"bytes4"}],"name":"supportsInterface","outputs":[{"name":"","type":"bool"}],"type":"function"},
]


def is_erc721_contract(contract_address: str, w3: Web3 = None) -> Optional[dict]:
    """
    Attempts to identify if a contract is an ERC-721 token and returns its information.
    
    Args:
        contract_address: The Ethereum contract address to check
        
    Returns:
        Dictionary with token info if ERC-721, None otherwise
    """
    if w3 is None:
        w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    contract = w3.eth.contract(address=w3.to_checksum_address(contract_address), abi=erc721_abi)
    
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
        error_str = str(e)
        # Common EVM errors that indicate the contract is not an ERC-721 token
        evm_errors = [
            "execution reverted",
            "InvalidFEOpcode",
            "InvalidJump",
            "EVM error: InvalidJump",
            "StackUnderflow",
            "EVM error: StackUnderflow"
        ]
        
        # If the error is one of the expected EVM errors, treat it as "not an ERC-721 token"
        if any(err in error_str for err in evm_errors):
            return None
            
        # For unexpected errors, raise with more context
        raise Exception(f"Unexpected error checking ERC-721 compliance: {str(e)}")


def classify_contract(contract_address: str, w3: Web3 = None) -> str:
    """Classify the type of contract"""
    if is_erc20_contract(contract_address, w3):
        return "ERC20"
    elif is_erc721_contract(contract_address, w3):
        return "ERC721"
    return "Unknown"  