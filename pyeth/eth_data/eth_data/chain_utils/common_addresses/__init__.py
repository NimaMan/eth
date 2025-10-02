from web3 import Web3
from .all_cex_addresses import *
from .all_etf_addresses import *
from .stablecoin_addresses import *
from .validators import *
from .misc import *
from .dex_pool_types import DEX_POOL_TYPES, DEX_POOL_TYPE_SET


DENOM_ADDRESSES = {
    # Stablecoins
    '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48': 'USDC',
    '0xdAC17F958D2ee523a2206206994597C13D831ec7': 'USDT',
    '0x6B175474E89094C44Da98b954EedeAC495271d0F': 'DAI',
    '0x4Fabb145d64652a948d72533023f6E7A623C7C53': 'BUSD',
    '0x8E870D67F660D95d5be530380D0eC0bd388289E1': 'PAX',
    '0x956F47F50A910163D8BF957Cf5846D573E7f87CA': 'FEI',
    '0x853d955aCEf822Db058eb8505911ED77F175b99e': 'FRAX',
    '0x5f98805A4E8be255a32880FDeC7F6728C6568bA0': 'LUSD',
    
    # Wrapped Tokens
    '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2': 'WETH',
    '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599': 'WBTC',
    '0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c': 'WBNB',
    
    # Other Major Tokens
    '0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0': 'MATIC',
    '0x514910771AF9Ca656af840dff83E8264EcF986CA': 'LINK',
    '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984': 'UNI',
    '0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9': 'AAVE',
    
    # Wrapped Versions of Stablecoins
    '0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643': 'cDAI',
    '0x39AA39c021dfbaE8faC545936693aC917d5E7563': 'cUSDC',
    '0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5': 'cETH',
    
    # Curve LP Tokens
    '0x6c3F90f043a72FA612cbac8115EE7e52BDe6E490': '3Crv',
    '0x06325440D014e39736583c165C2963BA99fAf14E': 'stETH-ETH',
    
    # Yearn Tokens
    '0xdA816459F1AB5631232FE5e97a05BBBb94970c95': 'yvDAI',
    '0xa354F35829Ae975e850e23e9615b11Da1B3dC4DE': 'yvUSDC',
    
    # Liquid Staking Derivatives
    '0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84': 'stETH',
    '0xBe9895146f7AF43049ca1c1AE358B0541Ea49704': 'cbETH',
    '0xae78736Cd615f374D3085123A210448E74Fc6393': 'rETH',
    
    # Major DeFi Governance Tokens
    '0x9f8F72aA9304c8B593d555F12eF6589cC3A579A2': 'MKR',
    '0xD533a949740bb3306d119CC777fa900bA034cd52': 'CRV',
    '0xC011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F': 'SNX',
    '0xc00e94Cb662C3520282E6f5717214004A7f26888': 'COMP',
    '0x0bc529c00C6401aEF6D220BE8C6Ea1667F6Ad93e': 'YFI',
    '0x6B3595068778DD592e39A122f4f5a5cF09C90fE2': 'SUSHI',
    '0xba100000625a3754423978a60c9317c58a424e3D': 'BAL',
    '0x111111111117dC0aa78b770fA6A738034120C302': '1INCH',
    
    # L2 Tokens
    '0xB50721BCf8d664c30412Cfbc6cf7a15145234ad1': 'ARB',
    '0x4200000000000000000000000000000000000042': 'OP',
    
    # High-volume tokens
    '0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE': 'SHIB',
    '0x6982508145454Ce325dDbE47a25d4ec3d2311933': 'PEPE',
}


DENOM_NAMES_TO_ADDRESS = {v: k for k, v in DENOM_ADDRESSES.items()}


# Complete decimals mapping for all tokens
ERC20_TOKEN_DECIMALS = {
    # Stablecoins
    'USDC': 6,
    'USDT': 6,
    "USD1": 18,
    'DAI': 18,
    'BUSD': 18,
    'PAX': 18,
    'FEI': 18,
    'FRAX': 18,
    'LUSD': 18,
    'BRZ': 18,
    'CADC': 18,
    'DOLA': 18,
    'EUROC': 6,
    'EURCV': 18,
    'EURS': 2,
    'EURT': 6,
    'FDUSD': 18,
    'GHO': 18,
    'GUSD': 2,
    'GYEN': 6,
    'IDRT': 2,
    'JPYC': 18,
    'MIM': 18,
    'MKUSD': 18,
    'OUSD': 18,
    'PAXG': 18,
    'PYUSD': 6,
    'RAI': 18,
    'sUSD': 18,
    'TUSD': 18,
    'USD0': 18,
    'USDP': 18,
    'USDS': 6,
    'USDD': 18,
    'USDE': 18,
    'XAUt': 6,
    'XIDR': 6,
    'XSGD': 6,
    'XUSD': 6,
    'ZUSD': 6,
    
    # Wrapped Tokens
    'WETH': 18,
    'WBTC': 8,
    'WBNB': 18,
    
    # Other Major Tokens
    'MATIC': 18,
    'LINK': 18,
    'UNI': 18,
    'AAVE': 18,
    
    # Wrapped Versions of Stablecoins
    'cDAI': 8,    # Compound DAI
    'cUSDC': 8,   # Compound USDC
    'cETH': 8,    # Compound ETH
    
    # Curve LP Tokens
    '3Crv': 18,   # 3pool LP token
    'stETH-ETH': 18,  # stETH-ETH LP token
    
    # Yearn Tokens
    'yvDAI': 18,  # Yearn DAI vault
    'yvUSDC': 6,  # Yearn USDC vault
    
    # Liquid Staking Derivatives
    'stETH': 18,  # Lido staked ETH
    'cbETH': 18,  # Coinbase staked ETH
    'rETH': 18,   # Rocket Pool ETH
    
    # Major DeFi Governance Tokens
    'MKR': 18,
    'CRV': 18,
    'SNX': 18,
    'COMP': 18,
    'YFI': 18,
    'SUSHI': 18,
    'BAL': 18,
    '1INCH': 18,
    
    # L2 Tokens
    'ARB': 18,
    'OP': 18,
    
    # High-volume tokens
    'SHIB': 18,
    'PEPE': 18,
}


DENOM_ADDRESSES = {**DENOM_ADDRESSES, **STABLECOINS_NAME_BY_ADDRESS}


addresses_by_name = {
    'zero_address': '0x0000000000000000000000000000000000000000',
    'dead_address': '0x000000000000000000000000000000000000dEaD',
    'WETH': '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
    'WBTC': '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599',
    'TrustSwap: Team Finance Lock': '0xE2fE530C047f2d85298b07D9333C05737f1435fB',
    'UNCX Network Security: LP Lockers': '0x663A5C229c09b049E36dCc11a9B0d4a8Eb9db214',
    'UNCX Network Security: Token Vesting': '0xDba68f07d1b7Ca219f78ae8582C213d975c25cAf',
    'UNCX Network Lockers: V3 Proof of Reserves 2': '0x7f5C649856F900d15C83741f45AE46f5C6858234',
    'UNCX Network Lockers: V3 Proof of Reserves 3': '0xFD235968e65B0990584585763f837A5b5330e6DE',
    'PinkLock02': '0x71B5759d73262FBb223956913ecF4ecC51057641',
    'UniswapV2Router02': '0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D',
    'UniswapV2Factory': '0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f',
    'UniswapV3Factory': '0x1F98431c8aD98523631AE4a59f267346ea31F984',
    "UniswapUniversalRouter": "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD",
    'NonfungiblePositionManager': '0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
    'WETH_USDT': '0x11b815efB8f581194ae79006d24E0d814B7697F6', 
    'Banana Gun: Router 2': '0x3328F7f4A1D1C57c35df56bBf0c9dCAFCA309C49',
    'Banana Gun: Deployer': '0xBCd3a47e4d0000cf170E25d1bD3d53F7C08be0A6',
    'Banana Gun: Deployer 2': '0x35fC556d6f8675B26fDF1542e6E894100155B34E',
    'Banana Gun': '0xC465CC50B7D5A29b9308968f870a4B242A8e1873',
    'Maestro: Router 2': '0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e',
    'Maestro: Deployer': '0x6599aE06914f1f5Ec0053d3F475348D40E608442',
    'Unibot': '0x5c9321e92Ba4eb43f2901c4952358e132163a85A',
    'Sigma / Alphaman': '0xe76014c179F19dA26Bb30A0f085FF0A466B92829',
    "Metamask: Swap Router": "0x881D40237659C251811CEC9c364ef91dC08D300C",
    "1inch v5: Aggregation Router": "0x1111111254EEB25477B68fb85Ed929f73A960582",
}

addresses_by_name = {**addresses_by_name, **STABLECOINS_ADDRESS_BY_NAME}

names_by_address = {v: k for k, v in addresses_by_name.items()}

byte_addresses_by_name = {k: Web3.to_bytes(hexstr=v) for k, v in addresses_by_name.items()}

names_by_byte_addresses = {v: k for k, v in byte_addresses_by_name.items()}

denominator_addresses_by_name = {key: addresses_by_name[key] for key in ['WETH', 'USDC', 'USDT', 'DAI'] if key in addresses_by_name}

denominator_names_by_address = {v: k for k, v in denominator_addresses_by_name.items()}

denominator_byte_addresses_by_name = {key: byte_addresses_by_name[key] for key in denominator_addresses_by_name}

denominator_names_by_byte_address = {v: k for k, v in denominator_byte_addresses_by_name.items()}


# Add CEX addresses to the main dictionaries
addresses_by_name.update(CEX_ADDRESSES_BY_NAME)
names_by_address.update(CEX_NAMES_BY_ADDRESS)

# Create a set of CEX addresses for quick lookup
CEX_ADDRESSES_SET = set(CEX_NAMES_BY_ADDRESS.keys())

# Add ETF addresses to the main dictionaries
addresses_by_name.update(ETF_ADDRESSES_BY_NAME)
names_by_address.update(ETF_NAMES_BY_ADDRESS)
