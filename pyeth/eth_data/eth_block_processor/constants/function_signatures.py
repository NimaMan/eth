from web3 import Web3

w3 = Web3()

# Function Signatures (without 0x prefix)
FUNCTION_SIGNATURES = {
             # General Uniswap functions
            w3.keccak(text="swap(uint256,uint256,address,bytes)").hex()[:8]: "Uniswap Swap",
            '791ac947': "Uniswap V2: Router 2",
            '04e45aaf': "Uniswap V3: Router 2",

            # Uniswap V2 Router Swap Functions
            w3.keccak(text="swapExactTokensForTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Swap",
            w3.keccak(text="swapTokensForExactTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Swap",
            w3.keccak(text="swapExactETHForTokens(uint256,address[],address,uint256)").hex()[:8]: "Swap",
            w3.keccak(text="swapTokensForExactETH(uint256,uint256,address[],address,uint256)").hex()[:8]: "Swap",
            w3.keccak(text="swapExactTokensForETH(uint256,uint256,address[],address,uint256)").hex()[:8]: "Swap",
            w3.keccak(text="swapETHForExactTokens(uint256,address[],address,uint256)").hex()[:8]: "Swap",

            # Fee on transfer variants
            w3.keccak(text="swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Swap",
            w3.keccak(text="swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)").hex()[:8]: "Swap",
            w3.keccak(text="swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)").hex()[:8]: "Swap",

            # Liquidity Functions
            w3.keccak(text="addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Add Liquidity",
            w3.keccak(text="addLiquidityETH(address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Add Liquidity",
            w3.keccak(text="removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Remove Liquidity",
            w3.keccak(text="removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Remove Liquidity",
            w3.keccak(text="removeLiquidityWithPermit(address,address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)").hex()[:8]: "Remove Liquidity",
            w3.keccak(text="removeLiquidityETHWithPermit(address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)").hex()[:8]: "Remove Liquidity",
            w3.keccak(text="removeLiquidityETHSupportingFeeOnTransferTokens(address,uint256,uint256,uint256,address,uint256)").hex()[:8]: "Remove Liquidity",
            w3.keccak(text="removeLiquidityETHWithPermitSupportingFeeOnTransferTokens(address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)").hex()[:8]: "Remove Liquidity",

            # Deposits and Withdrawals
            w3.keccak(text="deposit()").hex()[:8]: "Deposit",
            w3.keccak(text="withdraw(uint256)").hex()[:8]: "Withdraw",
            w3.keccak(text="depositETH()").hex()[:8]: "Deposit",
            w3.keccak(text="withdrawETH(uint256)").hex()[:8]: "Withdraw",

            # Staking and Unstaking
            w3.keccak(text="stake(uint256)").hex()[:8]: "Stake",
            w3.keccak(text="unstake(uint256)").hex()[:8]: "Unstake",
            w3.keccak(text="claim()").hex()[:8]: "Claim Rewards",
            w3.keccak(text="exit()").hex()[:8]: "Exit Staking",

            # Lending and Borrowing
            w3.keccak(text="borrow(uint256)").hex()[:8]: "Borrow",
            w3.keccak(text="repay(uint256)").hex()[:8]: "Repay",
            w3.keccak(text="flashLoan(address,address[],uint256[],uint256[],address,bytes,uint16)").hex()[:8]: "Flash Loan",
            w3.keccak(text="liquidate(address,uint256,address)").hex()[:8]: "Liquidate",

            # Governance
            w3.keccak(text="propose(address[],uint256[],string[],bytes[],string)").hex()[:8]: "Propose",
            w3.keccak(text="castVote(uint256,uint8)").hex()[:8]: "Cast Vote",
            w3.keccak(text="delegate(address)").hex()[:8]: "Delegate",
            w3.keccak(text="queue(uint256)").hex()[:8]: "Queue Proposal",
            w3.keccak(text="execute(uint256)").hex()[:8]: "Execute Proposal",

            # NFT Operations
            w3.keccak(text="mint(address,uint256)").hex()[:8]: "Mint NFT",
            w3.keccak(text="burn(uint256)").hex()[:8]: "Burn NFT",
            w3.keccak(text="safeTransferFrom(address,address,uint256,bytes)").hex()[:8]: "Safe Transfer NFT",

            # Token Operations
            w3.keccak(text="approve(address,uint256)").hex()[:8]: "Approve",
            w3.keccak(text="transfer(address,uint256)").hex()[:8]: "Transfer",
            w3.keccak(text="transferFrom(address,address,uint256)").hex()[:8]: "Transfer From",

            # Miscellaneous
            w3.keccak(text="multicall(bytes[])").hex()[:8]: "Multicall",
            w3.keccak(text="setApprovalForAll(address,bool)").hex()[:8]: "Set Approval For All",
            w3.keccak(text="upgradeTo(address)").hex()[:8]: "Upgrade Contract",
            
            "0162e2d0": "BananaGun",
            "088890dc": "Maestro",
            "2f100e4a": "Maestro",
            "09c182c3": "SigmaBuy",
            "3a571299": "SigmaSell",
            "51b001": "LayerSwap 1",

            'c9567bf9': 'Trading Enabled',
            "ed995307": "Add Liquidity",
            '667f6526': 'Set Tax'
            }

EVENT_TOPICS = {
    # ERC20/721/1155 Events
    'Transfer': w3.keccak(text="Transfer(address,address,uint256)").hex(),
    'TransferSingle': w3.keccak(text="TransferSingle(address,address,address,uint256,uint256)").hex(),
    'TransferBatch': w3.keccak(text="TransferBatch(address,address,address,uint256[],uint256[])").hex(),
    'Approval': w3.keccak(text="Approval(address,address,uint256)").hex(),
    
    # Basic Token Operations
    'Mint': w3.keccak(text="Mint(address,uint256,uint256)").hex(),
    'Burn': w3.keccak(text="Burn(address,uint256,uint256,address)").hex(),
    'Deposit': w3.keccak(text="Deposit(address,uint256)").hex(),
    'Withdraw': w3.keccak(text="Withdraw(address,uint256)").hex(),
    'WETHWithdrawal': w3.keccak(text="Withdrawal(address,uint256)").hex(),
    
    # Uniswap V2 Events
    'Sync': w3.keccak(text="Sync(uint112,uint112)").hex(),
    'Swap': w3.keccak(text="Swap(address,uint256,uint256,uint256,uint256,address)").hex(),
    'PairCreated': w3.keccak(text="PairCreated(address,address,address,uint256)").hex(),
    'Collect': w3.keccak(text="Collect(address,address,uint256,uint256)").hex(),
    'Flash': w3.keccak(text="Flash(address,uint256,uint256,uint256,uint256)").hex(),
    'IncreaseObservationCardinalityNext': w3.keccak(text="IncreaseObservationCardinalityNext(uint16,uint16)").hex(),
    'SetFeeProtocol': w3.keccak(text="SetFeeProtocol(uint8,uint8)").hex(),
    'CollectProtocol': w3.keccak(text="CollectProtocol(address,address,uint128,uint128)").hex(),
    
    # Token Management Events
    'OwnershipTransferred': w3.keccak(text="OwnershipTransferred(address,address)").hex(),
    'TradingEnabled': w3.keccak(text="TradingEnabled(uint256)").hex(),
    'TradingDisabled': w3.keccak(text="TradingDisabled(uint256)").hex(),
    'ExcludeFromFees': w3.keccak(text="ExcludeFromFees(address,bool)").hex(),
    'ExcludeFromLimits': w3.keccak(text="ExcludeFromLimits(address,bool)").hex(),
    'SetMaxTxAmount': w3.keccak(text="SetMaxTxAmount(uint256)").hex(),
    'SetMaxWalletToken': w3.keccak(text="SetMaxWalletToken(uint256)").hex(),
    'SetMaxWallet': w3.keccak(text="SetMaxWallet(uint256)").hex(),
    
    # Uniswap V3 Pool Events
    'Initialize': w3.keccak(text="Initialize(uint160,int24)").hex(),
    'UniswapV3Mint': w3.keccak(text="Mint(address,address,int24,int24,uint128,uint256,uint256)").hex(),
    'UniswapV3Burn': w3.keccak(text="Burn(address,int24,int24,uint128,uint256,uint256)").hex(),
    'UniswapV3Swap': w3.keccak(text="Swap(address,address,int256,int256,uint160,uint128,int24)").hex(),
    'PoolCreated': w3.keccak(text="PoolCreated(address,address,uint24,int24,address)").hex(),
    
    # Uniswap V3 NFT Manager Events
    'IncreaseLiquidity': w3.keccak(text="IncreaseLiquidity(uint256,uint128,uint256,uint256)").hex(),
    'DecreaseLiquidity': w3.keccak(text="DecreaseLiquidity(uint256,uint128,uint256,uint256)").hex(),
}