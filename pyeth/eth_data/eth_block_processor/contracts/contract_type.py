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


def is_erc20_contract(contract_address: str) -> Optional[dict]:
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
            raise Exception(f"Unexpected error checking ERC-20 compliance: {str(e)}")
        return None


erc721_abi = [
    {"constant":True,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[{"name":"interfaceId","type":"bytes4"}],"name":"supportsInterface","outputs":[{"name":"","type":"bool"}],"type":"function"},
]


def is_erc721_contract(contract_address: str) -> Optional[dict]:
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
            raise Exception(f"Unexpected error checking ERC-721 compliance: {str(e)}")
        return None
