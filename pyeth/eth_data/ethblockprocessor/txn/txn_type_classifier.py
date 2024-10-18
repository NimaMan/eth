from hexbytes import HexBytes
from typing import Dict, Any, Union
from web3 import Web3
from ethblockprocessor.data_models.txn_models import TransactionType


class ContractInteractionClassifier:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.known_functions = {
             # General Uniswap functions
            self.w3.keccak(text="swap(uint256,uint256,address,bytes)").hex()[:8]: "Uniswap Swap",
            '791ac947': "Uniswap V2: Router 2",
            '04e45aaf': "Uniswap V3: Router 2",

            # Uniswap V2 Router Swap Functions
            self.w3.keccak(text="swapExactTokensForTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Exact Tokens For Tokens",
            self.w3.keccak(text="swapTokensForExactTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Tokens For Exact Tokens",
            self.w3.keccak(text="swapExactETHForTokens(uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Exact ETH For Tokens",
            self.w3.keccak(text="swapTokensForExactETH(uint256,uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Tokens For Exact ETH",
            self.w3.keccak(text="swapExactTokensForETH(uint256,uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Exact Tokens For ETH",
            self.w3.keccak(text="swapETHForExactTokens(uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap ETH For Exact Tokens",

            # Fee on transfer variants
            self.w3.keccak(text="swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Exact Tokens For Tokens (Fee on Transfer)",
            self.w3.keccak(text="swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Exact ETH For Tokens (Fee on Transfer)",
            self.w3.keccak(text="swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Uniswap V2: Swap Exact Tokens For ETH (Fee on Transfer)",

            # Liquidity Functions
            self.w3.keccak(text="addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Add Liquidity",
            self.w3.keccak(text="addLiquidityETH(address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Add Liquidity ETH",
            self.w3.keccak(text="removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Remove Liquidity",
            self.w3.keccak(text="removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Remove Liquidity ETH",
            self.w3.keccak(text="removeLiquidityWithPermit(address,address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)").hex()[:8]: "Remove Liquidity With Permit",
            self.w3.keccak(text="removeLiquidityETHWithPermit(address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)").hex()[:8]: "Remove Liquidity ETH With Permit",
            self.w3.keccak(text="removeLiquidityETHSupportingFeeOnTransferTokens(address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Remove Liquidity ETH (Fee on Transfer)",
            self.w3.keccak(text="removeLiquidityETHWithPermitSupportingFeeOnTransferTokens(address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)").hex()[:8]: "Remove Liquidity ETH With Permit (Fee on Transfer)",

            # Deposits and Withdrawals
            self.w3.keccak(text="deposit()").hex()[:8]: "Deposit",
            self.w3.keccak(text="withdraw(uint256)").hex()[:8]: "Withdraw",
            self.w3.keccak(text="depositETH()").hex()[:8]: "Deposit ETH",
            self.w3.keccak(text="withdrawETH(uint256)").hex()[:8]: "Withdraw ETH",

            # Staking and Unstaking
            self.w3.keccak(text="stake(uint256)").hex()[:8]: "Stake",
            self.w3.keccak(text="unstake(uint256)").hex()[:8]: "Unstake",
            self.w3.keccak(text="claim()").hex()[:8]: "Claim Rewards",
            self.w3.keccak(text="exit()").hex()[:8]: "Exit Staking",

            # Lending and Borrowing
            self.w3.keccak(text="borrow(uint256)").hex()[:8]: "Borrow",
            self.w3.keccak(text="repay(uint256)").hex()[:8]: "Repay",
            self.w3.keccak(text="flashLoan(address,address[],uint256[],uint256[],address,bytes,uint16)").hex()[:8]: "Flash Loan",
            self.w3.keccak(text="liquidate(address,uint256,address)").hex()[:8]: "Liquidate",

            # Governance
            self.w3.keccak(text="propose(address[],uint256[],string[],bytes[],string)").hex()[:8]: "Propose",
            self.w3.keccak(text="castVote(uint256,uint8)").hex()[:8]: "Cast Vote",
            self.w3.keccak(text="delegate(address)").hex()[:8]: "Delegate",
            self.w3.keccak(text="queue(uint256)").hex()[:8]: "Queue Proposal",
            self.w3.keccak(text="execute(uint256)").hex()[:8]: "Execute Proposal",

            # NFT Operations
            self.w3.keccak(text="mint(address,uint256)").hex()[:8]: "Mint NFT",
            self.w3.keccak(text="burn(uint256)").hex()[:8]: "Burn NFT",
            self.w3.keccak(text="safeTransferFrom(address,address,uint256,bytes)").hex()[:8]: "Safe Transfer NFT",

            # Token Operations
            self.w3.keccak(text="approve(address,uint256)").hex()[:8]: "Approve",
            self.w3.keccak(text="transfer(address,uint256)").hex()[:8]: "Transfer",
            self.w3.keccak(text="transferFrom(address,address,uint256)").hex()[:8]: "Transfer From",

            # Miscellaneous
            self.w3.keccak(text="multicall(bytes[])").hex()[:8]: "Multicall",
            self.w3.keccak(text="setApprovalForAll(address,bool)").hex()[:8]: "Set Approval For All",
            self.w3.keccak(text="upgradeTo(address)").hex()[:8]: "Upgrade Contract",
            
            "0162e2d0": "BananaGun",
            "088890dc": "Maestro",
            "2f100e4a": "Maestro",
            "09c182c3": "SigmaBuy",
            "3a571299": "SigmaSell",
            
            "51b001": "LayerSwap 1",

            
            'c9567bf9': 'OpenTrading',
            }
        
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
        else:
            return TransactionType.CONTRACT_INTERACTION.value

        if function_signature in self.known_functions:
            return self.known_functions[function_signature]
        
        if to_address in self.known_contracts:
            return self.known_contracts[to_address]

        return TransactionType.CONTRACT_INTERACTION.value


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
            return TransactionType.CONTRACT_CREATION.value
        else:
            input_data = transaction.get('input')
            if input_data.hex() == '0x' or input_data.hex() == '':
                return TransactionType.ETH_TRANSFER.value
            elif self._starts_with(input_data, self.erc20_transfer_signature):
                return TransactionType.ERC20_TRANSFER.value
            elif self._starts_with(input_data, self.erc721_transfer_signature):
                return TransactionType.ERC721_TRANSFER.value
            elif self._starts_with(input_data, self.erc1155_transfer_signature):
                return TransactionType.ERC1155_TRANSFER.value
            elif self._starts_with(input_data, self.approve_signature):
                return TransactionType.APPROVE.value
            else:
                return self.contract_interaction_classifier.classify_interaction(transaction)

    def _starts_with(self, input_data: Union[HexBytes, str], prefix: str) -> bool:
        if isinstance(input_data, HexBytes):
            return input_data.hex().startswith(prefix)
        elif isinstance(input_data, str):
            return input_data.startswith(prefix)
        else:
            raise TypeError(f"Unsupported input type: {type(input_data)}")