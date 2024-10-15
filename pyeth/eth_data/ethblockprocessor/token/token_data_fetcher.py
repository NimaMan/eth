from typing import Dict, Any, List
from web3 import Web3
from functools import cached_property
from ethblockprocessor.txn.txn_log_analyzer import TransactionLogAnalyzer
from ethblockprocessor.txn.txn_analyzer import TransactionAnalyzer
from web3.exceptions import ContractLogicError
from decimal import Decimal


class ERC20TokenDataRetriever:
    """
    A class to retrieve comprehensive data related to a specific ERC20 token contract.

    This class connects to a local Ethereum node and provides methods to:
    - Retrieve token metadata (name, symbol, decimals, total supply).
    - Fetch recent transfer events involving the token.
    - Estimate the number of unique token holders.
    - Obtain contract creation information.
    - Leverage existing transaction analysis tools for efficiency.

    Attributes:
    -----------
    w3 : Web3
        An instance of the Web3 client connected to an Ethereum node.
    erc20_abi : list
        The ABI definition for standard ERC20 functions.
    """

    def __init__(self, rpc_url: str = "http://127.0.0.1:8545"):
        """
        Initializes the ERC20TokenDataRetriever with a connection to the Ethereum node.

        Parameters:
        -----------
        rpc_url : str
            The RPC URL of the Ethereum node. Defaults to "http://127.0.0.1:8545".
        """
        self.w3 = Web3(Web3.HTTPProvider(rpc_url))
        self.block_range = 100000

        # Define the standard ERC20 ABI with necessary functions
        self.erc20_abi = [
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
            {
                "constant": True,
                "inputs": [],
                "name": "decimals",
                "outputs": [{"name": "", "type": "uint8"}],
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

    def get_token_data(self, token_address: str, from_block: int = None, to_block: int = None) -> Dict[str, Any]:
        """
        Retrieves comprehensive data about the token.

        Parameters:
        -----------
        token_address : str
            The contract address of the token.

        Returns:
        --------
        Dict[str, Any]
            A dictionary containing token metadata, transfer events, holder count, and creation info.
        """
        # Get recent transfer events
        if to_block is None:
            latest_block = self.w3.eth.get_block('latest')
            to_block = latest_block['number']
        if from_block is None:
            from_block = to_block - self.block_range
        
        token_address = self.w3.to_checksum_address(token_address)
        contract = self.w3.eth.contract(address=token_address, abi=self.erc20_abi)

        # Retrieve token metadata
        name = self._get_contract_property(contract.functions.name)
        symbol = self._get_contract_property(contract.functions.symbol)
        decimals = self._get_contract_property(contract.functions.decimals, default=18)
        total_supply = self._get_contract_property(contract.functions.totalSupply)
        total_supply_formatted = Decimal(total_supply) / Decimal(10 ** decimals) if total_supply else None

        return {
            "address": token_address,
            "name": name,
            "symbol": symbol,
            "decimals": decimals,
            "total_supply": str(total_supply_formatted) if total_supply_formatted else None,
        }

    @property
    def contract_code_length(self):
        return len(self.contract_bytecode)

    @property
    def contract_balance_eth(self):
        return self.w3.from_wei(self.contract_balance, 'ether')
    
    @property
    def latest_transfer_events(self):
        return self.get_transfer_events(self.token_address, self.from_block, self.to_block)[:10]

    @property
    def contract_creation_info(self):
        return self.get_contract_creation_info(self.token_address)

    @property
    def creator(self):
        return self.contract_creation_info['creator']
    
    @property
    def creation_block(self):
        return self.contract_creation_info['creation_block']
    
    @property
    
    def _get_contract_property(self, function_call, default=None):
        """
        Helper method to call a contract function with error handling.

        Parameters:
        -----------
        function_call : ContractFunction
            The contract function to call.
        default : Any
            The default value to return in case of failure.

        Returns:
        --------
        Any
            The result of the contract function or the default value.
        """
        try:
            return function_call().call()
        except (ContractLogicError, ValueError):
            return default

    def get_transfer_events(self, token_address: str, from_block: int, to_block: int) -> List[Dict[str, Any]]:
        """
        Retrieves and decodes Transfer events from the token contract within a block range.

        Parameters:
        -----------
        token_address : str
            The contract address of the token.
        from_block : int
            The starting block number.
        to_block : int
            The ending block number.

        Returns:
        --------
        List[Dict[str, Any]]
            A list of decoded Transfer events.
        """
        transfer_event_signature_hash = self.w3.keccak(text='Transfer(address,address,uint256)').hex()
        event_filter_params = {
            'fromBlock': from_block,
            'toBlock': to_block,
            'address': token_address,
            'topics': [transfer_event_signature_hash],
        }
        logs = self.w3.eth.get_logs(event_filter_params)

        # Use the TransactionLogAnalyzer to process logs
        txn_log_analyzer = TransactionLogAnalyzer(self.w3)
        analyzed_logs = txn_log_analyzer.analyze_logs(logs)
        transfer_events = analyzed_logs.get('erc20_transfers', [])

        # Convert TokenTransfer dataclass instances to dictionaries
        transfer_events_dicts = [transfer_event.__dict__ for transfer_event in transfer_events]

        return transfer_events_dicts

    @cached_property
    def contract_bytecode(self):
        return self.w3.eth.get_code(self.token_address)

    def get_contract_creation_info(self, token_address: str) -> Dict[str, Any]:
        """
        Retrieves the contract creation information.

        Parameters:
        -----------
        token_address : str
            The contract address.

        Returns:
        --------
        Dict[str, Any]
            A dictionary containing the creator's address, creation block number, and transaction hash.
        """
        token_address = self.w3.to_checksum_address(token_address)
        creation_txn = None

        # Efficiently retrieve the creation transaction using 'eth_getTransactionByBlock'
        for block_number in range(self.w3.eth.get_block_number(), 0, -1):
            block = self.w3.eth.get_block(block_number, full_transactions=True)
            for txn in block['transactions']:
                if txn['to'] is None and txn.get('creates') == token_address:
                    creation_txn = txn
                    break
            if creation_txn:
                break
            
        if creation_txn:
            return {
                'creator': creation_txn['from'],
                'creation_block': creation_txn['blockNumber'],
                'creation_transaction': creation_txn['hash'].hex(),
            }
        else:
            return {'error': 'Contract creation information not found'}