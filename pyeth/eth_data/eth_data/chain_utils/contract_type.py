from web3 import Web3
from typing import Optional, Union
from web3.exceptions import BadFunctionCallOutput, ContractLogicError, Web3RPCError
from eth_abi.exceptions import DecodingError


erc20_abi = [
    {"constant":True,"inputs":[],"name":"name","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"symbol","outputs":[{"name":"","type":"string"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"decimals","outputs":[{"name":"","type":"uint8"}],"type":"function"},
    {"constant":True,"inputs":[],"name":"totalSupply","outputs":[{"name":"","type":"uint256"}],"type":"function"},
    {"constant":True,"inputs":[{"name":"_owner","type":"address"}],"name":"balanceOf","outputs":[{"name":"balance","type":"uint256"}],"type":"function"},
]

# Some legacy tokens encode name() and symbol() as bytes32 (fixed‑length)
erc20_bytes32_abi = [
    {"constant": True, "inputs": [], "name": "name", "outputs": [{"name": "", "type": "bytes32"}], "type": "function"},
    {"constant": True, "inputs": [], "name": "symbol", "outputs": [{"name": "", "type": "bytes32"}], "type": "function"},
    {"constant": True, "inputs": [], "name": "decimals", "outputs": [{"name": "", "type": "uint8"}], "type": "function"},
    {"constant": True, "inputs": [], "name": "totalSupply", "outputs": [{"name": "", "type": "uint256"}], "type": "function"},
]

def _bytes32_to_text(b: bytes) -> str:
    """Convert bytes32 / padded bytes to cleaned text."""
    return Web3.to_text(b).rstrip('\x00')


def _looks_like_minimal_proxy(bytecode: bytes) -> bool:
    """
    Return True for EIP‑1167 clones, metamorphic stubs, or *any* tiny runtime
    that clearly lacks the two core ERC‑20 selectors (balanceOf / totalSupply).

    Heuristics:
    • length < 64  => definitely a clone placeholder
    • length < 128 and *missing* 0x70a08231 or 0x18160ddd selectors
      => almost certainly a proxy shell or non‑ERC20 helper
    • specific opcode prefixes for OpenZeppelin (45 B) and 32 B meta‑clone
    """
    ln = len(bytecode)
    if ln < 64:
        return True

    if (
        bytecode.startswith(b"\x36\x3d\x3d\x37\x3d\x3d\x3d\x36\x3d\x73")  # OZ clone runtime
        or bytecode.startswith(b"\x60\x20\x36\x03\x80")                   # 32‑B meta clone
    ):
        return True

    # Fast negative test for small contracts that do NOT embed ERC‑20 logic
    if ln < 128 and b"\x70\xa0\x82\x31" not in bytecode and b"\x18\x16\x0d\xdd" not in bytecode:
        return True

    return False


def get_erc20_contract_info_rpc(contract_address: str, w3: Web3 = None, block_identifier: Optional[Union[int, str]] = 'latest') -> Optional[dict]:
    """
    Attempts to identify if a contract is an ERC-20 token and returns its information
    at a specific block identifier.
    
    This function calls standard ERC-20 methods on the contract to determine if it
    implements the ERC-20 interface. It handles various error conditions that might
    occur when interacting with non-ERC-20 contracts.
    
    Args:
        contract_address: The Ethereum contract address to check
        w3: Optional Web3 instance (creates one with local provider if None)
        block_identifier: The block number or block identifier (e.g., 'latest', 'pending') 
                          to query the state at. Defaults to 'latest'.
        
    Returns:
        Dictionary with token info if ERC-20, None otherwise
    """
    if w3 is None:
        w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545")) 
        
    if not w3.is_connected():
        raise ConnectionError("Web3 provider is not connected.")

    checksum_address = w3.to_checksum_address(contract_address)
    code = w3.eth.get_code(checksum_address, block_identifier)
    # If no code, or clearly a minimal‑proxy stub, treat as non‑ERC20
    if not code or _looks_like_minimal_proxy(code):
        return None

    contract = w3.eth.contract(address=checksum_address, abi=erc20_abi)
    
    try:
        try:
            call_params = {"gas": 50_000}
            symbol = contract.functions.symbol().call(call_params, block_identifier=block_identifier)
            name = contract.functions.name().call(call_params, block_identifier=block_identifier)
        except (UnicodeDecodeError, DecodingError, ValueError):
            # Retry with bytes32 ABI
            contract_b32 = w3.eth.contract(address=checksum_address, abi=erc20_bytes32_abi)
            symbol_b = contract_b32.functions.symbol().call(call_params, block_identifier=block_identifier)
            name_b = contract_b32.functions.name().call(call_params, block_identifier=block_identifier)
            symbol = _bytes32_to_text(symbol_b)
            name = _bytes32_to_text(name_b)

        decimals = contract.functions.decimals().call(call_params, block_identifier=block_identifier)
        raw_total_supply = contract.functions.totalSupply().call(call_params, block_identifier=block_identifier)
        
        # Adjust total supply using decimals
        adjusted_total_supply = raw_total_supply / (10**decimals)
        
        return {
            'contract_address': contract_address,
            'name': name,
            'symbol': symbol,
            'decimals': decimals,
            'total_supply': adjusted_total_supply, # Return adjusted supply
            'raw_total_supply': raw_total_supply  # Optionally return raw supply too
        }
    except (BadFunctionCallOutput, ContractLogicError, Web3RPCError, DecodingError, UnicodeDecodeError) as e:
        # Contract is not an ERC20 token or has invalid bytecode at this block
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
            "gas required exceeds allowance",  # Add gas limit error as recognized
            "cannot decode",
            "invalid operand",
            "An RPC error was returned by the node",
            "execution error"
        ]
        
        # If the error is one of the expected EVM/node errors, treat it as "not an ERC-20 token at this block"
        if any(err in error_str for err in evm_errors):
            return None
            
        # For unexpected errors, raise with more context
        raise Exception(f"Unexpected error checking ERC-20 compliance for {contract_address} at block {block_identifier}: {str(e)}")


def is_erc20_contract(contract_address: str, w3: Web3 = None, block_identifier: Optional[Union[int, str]] = 'latest') -> Optional[dict]:
    return get_erc20_contract_info_rpc(contract_address, w3, block_identifier) is not None


# ---------------------------------------------------------------------------
# ERC‑165 & ERC‑721 helpers
# ---------------------------------------------------------------------------
erc165_abi = [
    {
        "constant": True,
        "inputs": [{"name": "interfaceId", "type": "bytes4"}],
        "name": "supportsInterface",
        "outputs": [{"name": "", "type": "bool"}],
        "type": "function",
    }
]

erc721_abi = [
    {
        "constant": True,
        "inputs": [],
        "name": "name",
        "outputs": [{"name": "", "type": "string"}],
        "type": "function",
    },
    {
        "constant": True,
        "inputs": [],
        "name": "symbol",
        "outputs": [{"name": "", "type": "string"}],
        "type": "function",
    },
    # totalSupply is optional
    {
        "constant": True,
        "inputs": [],
        "name": "totalSupply",
        "outputs": [{"name": "", "type": "uint256"}],
        "type": "function",
    },
]

# Fallback for legacy tokens that return bytes32
erc721_bytes32_abi = [
    {
        "constant": True,
        "inputs": [],
        "name": "name",
        "outputs": [{"name": "", "type": "bytes32"}],
        "type": "function",
    },
    {
        "constant": True,
        "inputs": [],
        "name": "symbol",
        "outputs": [{"name": "", "type": "bytes32"}],
        "type": "function",
    },
    {
        "constant": True,
        "inputs": [],
        "name": "totalSupply",
        "outputs": [{"name": "", "type": "uint256"}],
        "type": "function",
    },
]


def _supports_interface(
    w3: Web3,
    address: str,
    interface_id: bytes,
    block_identifier: Union[int, str] = "latest",
) -> bool:
    """
    Safe ERC‑165 supportsInterface check with a low gas cap.
    """
    contract = w3.eth.contract(address=address, abi=erc165_abi)
    try:
        return contract.functions.supportsInterface(interface_id).call(
            {"gas": 50_000}, block_identifier=block_identifier
        )
    except (BadFunctionCallOutput, ContractLogicError, DecodingError, UnicodeDecodeError):
        return False
    except Exception as e:
        # Treat common node/VM errors (out‑of‑gas, invalid opcode) as 'False'
        if "out of gas" in str(e).lower() or "invalid operand" in str(e).lower():
            return False
        raise


def get_erc721_contract_info(
    contract_address: str,
    w3: Web3 = None,
    block_identifier: Optional[Union[int, str]] = "latest",
) -> Optional[dict]:
    """
    Fetch basic metadata (name, symbol, totalSupply) for ERC‑721 tokens.
    Returns None if contract is not ERC‑721‑compliant.
    """
    if w3 is None:
        w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    if not w3.is_connected():
        raise ConnectionError("Web3 provider is not connected.")

    checksum = w3.to_checksum_address(contract_address)

    # Quick ERC‑165 test ― saves RPC calls on non‑NFT contracts
    if not _supports_interface(w3, checksum, b"\x80\xac\x58\xcd", block_identifier):
        return None

    contract = w3.eth.contract(address=checksum, abi=erc721_abi)

    try:
        name = contract.functions.name().call(block_identifier=block_identifier)
        symbol = contract.functions.symbol().call(block_identifier=block_identifier)
    except (UnicodeDecodeError, DecodingError, ValueError):
        # Retry legacy bytes32 format
        legacy = w3.eth.contract(address=checksum, abi=erc721_bytes32_abi)
        name = _bytes32_to_text(
            legacy.functions.name().call(block_identifier=block_identifier)
        )
        symbol = _bytes32_to_text(
            legacy.functions.symbol().call(block_identifier=block_identifier)
        )

    # totalSupply is optional
    total_supply = None
    try:
        total_supply = contract.functions.totalSupply().call(
            block_identifier=block_identifier
        )
    except (BadFunctionCallOutput, ContractLogicError):
        pass

    return {
        "contract_address": contract_address,
        "name": name,
        "symbol": symbol,
        "total_supply": total_supply,
    }


def is_erc721_contract(
    contract_address: str,
    w3: Web3 = None,
    block_identifier: Optional[Union[int, str]] = "latest",
) -> bool:
    """
    True if contract *appears* to implement ERC‑721.
    """
    if w3 is None:
        w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    if not w3.is_connected():
        raise ConnectionError("Web3 provider is not connected.")

    checksum = w3.to_checksum_address(contract_address)

    # 1. ERC‑165 test
    if _supports_interface(w3, checksum, b"\x80\xac\x58\xcd", block_identifier):
        return True

    # 2. Fallback metadata probe (very low‑cost)
    try:
        info = get_erc721_contract_info(checksum, w3, block_identifier)
        return info is not None
    except Exception:
        return False
    

def classify_contract(address: str, w3: Web3, block='latest') -> str:
    checksum = w3.to_checksum_address(address)
    code = w3.eth.get_code(checksum, block)

    # ---------- NEW early exit ----------
    if not code or _looks_like_minimal_proxy(code):
        return "Unknown"
    # ------------------------------------

    # existing ERC‑20 / 721 / 1155 probes follow …
    try:
        if get_erc20_contract_info_rpc(checksum, w3, block):
            return "ERC20"
        if get_erc721_contract_info(checksum, w3, block):
            return "ERC721"
    except (BadFunctionCallOutput,
            ContractLogicError,
            Web3RPCError,
            DecodingError,
            UnicodeDecodeError):
        return "Unknown"