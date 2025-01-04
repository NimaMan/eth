from web3 import Web3

w3 = Web3()

# Function Signatures (without 0x prefix)
FUNCTION_SIGNATURES = {

    # ERC20/721/1155 Functions
    w3.keccak(text="name()").hex()[:8]: "Name",
    w3.keccak(text="symbol()").hex()[:8]: "Symbol",
    w3.keccak(text="decimals()").hex()[:8]: "Decimals",
    w3.keccak(text="totalSupply()").hex()[:8]: "Total Supply",
    w3.keccak(text="balanceOf(address)").hex()[:8]: "Balance Of",
    w3.keccak(text="allowance(address,address)").hex()[:8]: "Allowance",
    w3.keccak(text="increaseAllowance(address,uint256)").hex()[:8]: "Increase Allowance",
    w3.keccak(text="decreaseAllowance(address,uint256)").hex()[:8]: "Decrease Allowance",
    
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
    w3.keccak(text="lockLPToken(address _lpToken, uint256 _amount, uint256 _unlock_date, address _referral, bool _fee_in_eth, address _withdrawer)").hex()[:8]: "Lock Liquidity Tokens",

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
    
    # Uniswap V3 NonfungiblePositionManager Functions
    w3.keccak(text="mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))").hex()[:8]: "Mint Position V3",
    w3.keccak(text="increaseLiquidity((uint256,uint256,uint256,uint256,uint256,uint256))").hex()[:8]: "Increase Liquidity V3",
    w3.keccak(text="decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))").hex()[:8]: "Decrease Liquidity V3",
    w3.keccak(text="collect((uint256,address,uint128,uint128))").hex()[:8]: "Collect V3",
    w3.keccak(text="burn(uint256)").hex()[:8]: "Burn Position V3",
    
    # Uniswap V3 Pool Functions
    w3.keccak(text="initialize(uint160)").hex()[:8]: "Initialize Pool V3",
    w3.keccak(text="mint(address,int24,int24,uint128,bytes)").hex()[:8]: "Add Liquidity V3",
    w3.keccak(text="swap(address,bool,int256,uint160,bytes)").hex()[:8]: "Swap V3",
    w3.keccak(text="flash(address,uint256,uint256,bytes)").hex()[:8]: "Flash V3",
    
    # Uniswap V3 Factory Functions
    w3.keccak(text="createPool(address,address,uint24)").hex()[:8]: "Create Pool V3",
    w3.keccak(text="setOwner(address)").hex()[:8]: "Set Factory Owner V3",
    w3.keccak(text="enableFeeAmount(uint24,int24)").hex()[:8]: "Enable Fee Amount V3",

    "0162e2d0": "BananaGun",
    "088890dc": "Maestro",
    "2f100e4a": "Maestro",
    "09c182c3": "SigmaBuy",
    "3a571299": "SigmaSell",
    "51b001": "LayerSwap 1",

    'c9567bf9': 'Trading Enabled',
    'fb201b1d': 'Trading Enabled',
    "8a8c523c": "Trading Enabled",
    "ed995307": "Add Liquidity",
    '667f6526': 'Set Tax',
    '74010ece': 'Set Max Txn Amount',
    'ea1644d5': 'Set Max Wallet Size',
    '715018a6': "Renounce Ownership",
    '8af416f6': "Lock Liquidity Tokens",
    
    }

EVENT_TOPICS = {
    # ERC20/721/1155 Events (these are shared standards)
    'Transfer': w3.keccak(text="Transfer(address,address,uint256)").hex(),  # Used for ERC20/721/NFT positions
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
    'InitializeV3': w3.keccak(text="Initialize(uint160,int24)").hex(),
    'MintV3': w3.keccak(text="Mint(address,address,int24,int24,uint128,uint256,uint256)").hex(),
    'BurnV3': w3.keccak(text="Burn(address,int24,int24,uint128,uint256,uint256)").hex(),
    'SwapV3': w3.keccak(text="Swap(address,address,int256,int256,uint160,uint128,int24)").hex(),
    'SetFeeProtocolV3': w3.keccak(text="SetFeeProtocol(uint8,uint8,uint8,uint8)").hex(),
    'CollectProtocolV3': w3.keccak(text="CollectProtocol(address,address,uint128,uint128)").hex(),
    
    # Uniswap V3 Factory Events
    'PoolCreatedV3': w3.keccak(text="PoolCreated(address,address,uint24,int24,address)").hex(),
    'FeeAmountEnabled': w3.keccak(text="FeeAmountEnabled(uint24,int24)").hex(),
    'OwnerChanged': w3.keccak(text="OwnerChanged(address,address)").hex(),
    
    # Uniswap V3 NonfungiblePositionManager Events
    'IncreaseLiquidityV3': w3.keccak(text="IncreaseLiquidity(uint256,uint128,uint256,uint256)").hex(),
    'DecreaseLiquidityV3': w3.keccak(text="DecreaseLiquidity(uint256,uint128,uint256,uint256)").hex(),
    
}

EVENT_TOPICS_REVERSE = {v: k for k, v in EVENT_TOPICS.items()}

UNISWAP_CONTRACTS = {
    "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D": "Uniswap V2: Router 2",
    "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45": "Uniswap V3: Router 2",
    '0xC36442b4a4522E871399CD717aBDD847Ab11FE88': "POSITION_MANAGER",
    '0x1f98431c8ad98523631ae4a59f267346ea31f984': "FACTORY",
    '0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45': "ROUTER",
    '0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6': "QUOTER",
}