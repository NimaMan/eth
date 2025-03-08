import pytest
from hexbytes import HexBytes
from eth_block_processor.txn.txn_log_processor import TransactionLogProcessor
from eth_block_processor.data_models.receipt_models import *
from web3 import Web3


@pytest.fixture
def log_processor(w3):
    return TransactionLogProcessor(w3)


def test_parse_uniswap_v2_sync(log_processor):
    """Test parsing of Uniswap V2 Sync event with string-encoded values"""
    log = {
        'address': '0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0',
        'topics': [HexBytes('0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1')],
        'data': HexBytes(
            '0x00000000000000000000000000000000000000000000000d7fbbe35020a4023b'
            '0000000000000000000000000000000000000000000000015c54ae87720eb000'
        ),
        'logIndex': 136
    }
    
    result = log_processor.parse_uniswap_v2_sync(log)
    
    # Expected values as strings
    expected_reserve0 = '249011873154970419771'  # 0x0d7fbbe35020a4023b
    expected_reserve1 = '25099878520000000000'   # 0x15c54ae87720eb000
    
    assert result == UniswapV2Sync(
        pair_address='0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0',
        reserve0=expected_reserve0,
        reserve1=expected_reserve1,
        log_index=136
    )
    
    # Validate string types and checksum
    assert isinstance(result.reserve0, str)
    assert isinstance(result.reserve1, str)
    assert result.pair_address == '0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0'
    

def test_parse_uniswap_v2_swap(log_processor):
    """Test parsing of Uniswap V2 Swap event with complex data"""
    log = {
        'address': '0x52fd104d206ee67f5d26bb66f64781af887b2eb0',
        'topics': [
            HexBytes('0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822'),
            HexBytes('0x0000000000000000000000007d217ffc52898191d97ef38ef448c8f46ce880e2'),
            HexBytes('0x0000000000000000000000007d217ffc52898191d97ef38ef448c8f46ce880e2')
        ],
        'data': HexBytes(
            '0x0000000000000000000000000000000000000000000000000898dbc275352a00'
            '0000000000000000000000000000000000000000000000000000000000000000'
            '0000000000000000000000000000000000000000000000000000000000000000'
            '000000000000000000000000000000000000000000000000ddba6063614200'
        ),
        'logIndex': 137
    }
    
    result = log_processor.parse_uniswap_v2_swap(log)
    
    expected = UniswapV2Swap(
        pair_address='0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0',
        sender='0x7d217fFC52898191D97eF38EF448c8F46Ce880e2',
        to='0x7d217fFC52898191D97eF38EF448c8F46Ce880e2',
        amount0In='619486577000000000',  # 0x0898dbc275352a00
        amount1In='0',
        amount0Out='0',
        amount1Out='62410893000000000',  # 0xddba6063614200
        log_index=137
    )
    
    assert result == expected
    assert result.pair_address == expected.pair_address
    assert result.sender == expected.sender
    assert result.to == expected.to
    assert isinstance(result.amount0In, str)
    assert isinstance(result.amount1Out, str)
    

def test_parse_pair_creation(log_processor):
    """Test parsing of Uniswap V2 PairCreated event"""
    log = {'address': '0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f',
        'topics': ['0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9',
        '0x0000000000000000000000002551bc3f26129019624f1fb09ebb880e94dbc22a',
        '0x000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2'],
        'data': '0x00000000000000000000000052fd104d206ee67f5d26bb66f64781af887b2eb0000000000000000000000000000000000000000000000000000000000006259e',
        'blockHash': '0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4',
        'blockNumber': '0x14d4b37',
        'blockTimestamp': '0x67aeda53',
        'transactionHash': '0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66',
        'transactionIndex': '0x8d',
        'logIndex': '0x111',
        'removed': False}
    
    result = log_processor.parse_pair(log)
    
    expected = PairAction(
        pair_address='0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0',
        token0='0x2551Bc3f26129019624F1fB09ebB880E94dBc22A',
        token1='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
        log_index=273
    )
    
    assert result == expected
    assert result.pair_address == expected.pair_address
    assert result.token0 == expected.token0
    assert result.token1 == expected.token1
    assert isinstance(result.pair_address, str)
    assert Web3.is_checksum_address(result.pair_address)
    

def test_parse_approval(log_processor):
    """Test parsing of ERC20 Approval event with max uint256 value"""
    log = {'address': '0x2551bc3f26129019624f1fb09ebb880e94dbc22a',
        'topics': ['0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925',
        '0x000000000000000000000000a51b1c4766f0fc9409525784b3bf62385ddfa08d',
        '0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d'],
        'data': '0xfffffffffffffffffffffffffffffffffffffffffffffff2728d948e8857ffff',
        'blockHash': '0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4',
        'blockNumber': '0x14d4b37',
        'blockTimestamp': '0x67aeda53',
        'transactionHash': '0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66',
        'transactionIndex': '0x8d',
        'logIndex': '0x113',
        'removed': False}
            
    result = log_processor.parse_approve(log)
    
    expected = ERC20Approval(
        token_address='0x2551Bc3f26129019624F1fB09ebB880E94dBc22A',
        owner='0xa51B1C4766F0fC9409525784b3BF62385dDfA08d',
        spender='0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D',
        amount='115792089237316195423570985008687907853269984665640564039207584007913129639935',
        log_index=275
    )
    
    assert result == expected
    assert result.owner == expected.owner
    assert result.spender == expected.spender
    assert result.amount == expected.amount
    assert isinstance(result.amount, str)
    assert Web3.is_checksum_address(result.owner)
    assert Web3.is_checksum_address(result.spender)
    

def test_parse_weth_deposit(log_processor):
    """Test parsing of WETH Deposit event"""
    log = {
            'address': '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2',
            'topics': ['0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c',
            '0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d'],
            'data': '0x0000000000000000000000000000000000000000000000015af1d78b58c40000',
            'blockHash': '0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4',
            'blockNumber': '0x14d4b37',
            'blockTimestamp': '0x67aeda53',
            'transactionHash': '0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66',
            'transactionIndex': '0x8d',
            'logIndex': '0x114',
            'removed': False}
                
    result = log_processor.parse_deposit(log)
    
    expected = DepositAction(
        pair_address=Web3.to_checksum_address('0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2'),
        sender=Web3.to_checksum_address('0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D'),
        amount='25000000000000000000',  # 25 ETH
        log_index=276
    )
    
    assert result == expected
    assert result.pair_address == expected.pair_address
    assert result.sender == expected.sender
    assert result.amount == expected.amount
    assert Web3.is_checksum_address(result.pair_address)
    assert Web3.is_checksum_address(result.sender)
    assert isinstance(result.amount, str)
    

def test_parse_uniswap_v2_mint(log_processor):
    """Test parsing of Uniswap V2 Mint event"""
    log = {
        'address': '0x52fd104d206ee67f5d26bb66f64781af887b2eb0',
        'topics': ['0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f',
        '0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d'],
        'data': '0x00000000000000000000000000000000000000000000000d8d726b7177a800000000000000000000000000000000000000000000000000015af1d78b58c40000',
        'blockHash': '0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4',
        'blockNumber': '0x14d4b37',
        'blockTimestamp': '0x67aeda53',
        'transactionHash': '0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66',
        'transactionIndex': '0x8d',
        'logIndex': '0x119',
        'removed': False}
    
    result = log_processor.parse_mint(log)
    
    expected = MintAction(
        pair_address=Web3.to_checksum_address('0x52fd104d206ee67f5d26bb66f64781af887b2eb0'),
        sender=Web3.to_checksum_address('0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D'),
        amount0='250000000000000000000',  # 250 tokens
        amount1='25000000000000000000',   # 25 tokens
        log_index=281
    )
    
    assert result == expected
    assert result.sender == expected.sender
    assert result.amount0 == expected.amount0
    assert result.amount1 == expected.amount1
    assert Web3.is_checksum_address(result.pair_address)
    assert Web3.is_checksum_address(result.sender)
    assert isinstance(result.amount0, str)
    assert isinstance(result.amount1, str)


def test_parse_uniswap_v3_initialize(log_processor):
    """Test parsing of Uniswap V3 Initialize event"""
    log = {'address': '0xbe931909a485643acdb7eef4184c1284cd8efe8f',
        'topics': ['0x98636036cb66a9c19a37435efc1e90142190214e8abeb821bdba3f2990dd4c95'],
        'data': '0x0000000000000000000000000000000000000000000010dc8023230f0bb0a246fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffbc900',
        'blockHash': '0xa33211cc390ecf352ec46ec92fb4d6e8f3da0838ea149956d4434155b7753d1c',
        'blockNumber': '0x14ee285',
        'blockTimestamp': '0x67c211f3',
        'transactionHash': '0xcb878cb2353c10f70837e2bfd3b41b16104b9e08d280ddbf461f2e70a2918719',
        'transactionIndex': '0x69',
        'logIndex': '0xc8',
        'removed': False}
    
    result = log_processor.parse_uniswap_v3_initialize(log)
    expected = UniswapV3Initialize(
        pool_address='0xbe931909a485643ACdb7EeF4184c1284cd8eFe8f',
        sqrt_price_x96=79625380684338992030278,
        tick=-276224,
        log_index=200
    )
    
    assert result == expected


def test_parse_v3_swap(log_processor):
    log = {'address': '0x7c706586679af2ba6d1a9fc2da9c6af59883fdd3',
        'topics': ['0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67',
        '0x00000000000000000000000051c72848c68a965f66fa7a88855f9f7784502a7f',
        '0x00000000000000000000000051c72848c68a965f66fa7a88855f9f7784502a7f'],
        'data': '0x0000000000000000000000000000000000000000000000000fe2beeece54c200ffffffffffffffffffffffffffffffffffffffffffffffffffffff90e13987da0000000000000000000000000000000000000000002a5a1a94ca527a9d1de4f4000000000000000000000000000000000000000000000000079e60225560e879fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffdc231',
        'blockHash': '0x1878ae05c456af94ca704f9852033256d25d594b18bf2bb1f91858660f788c67',
        'blockNumber': '0x14fa6c9',
        'blockTimestamp': '0x67cb50cf',
        'transactionHash': '0x0aca9e9c57b3f977c5b9b7f9b63abae3141163798e5964f261a1e4a896adc1f0',
        'transactionIndex': '0x12',
        'logIndex': '0x42',
        'removed': False}
    
    result = log_processor.parse_uniswap_v3_swap(log)
    
    expected = UniswapV3Swap(
        pool_address='0x7C706586679Af2BA6D1A9fC2DA9C6aF59883fdD3',
        sender='0x51C72848c68a965f66FA7a88855F9f7784502a7F',
        recipient='0x51C72848c68a965f66FA7a88855F9f7784502a7F',
        amount0='1144687188178682368',
        amount1='-477257693222',
        sqrt_price_x96='51200387744091159339590900',
        liquidity='548981905163348089',
        tick='-146895',
        log_index=66
    )
    
    assert result == expected


def test_parse_uniswap_v4_initialize(log_processor):
    log = {'address': '0x000000000004444c5dc75cb358380d2e3de08a90',
        'topics': ['0xdd466e674ea557f56295e2d0218a125ea4b4f0f6f3307b95f85e6110838d6438',
        '0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3',
        '0x0000000000000000000000000000000000000000000000000000000000000000',
        '0x00000000000000000000000051ea52a6a885dd34f30494768cf1d9dc9f004185'],
        'data': '0x0000000000000000000000000000000000000000000000000000000000000bb8000000000000000000000000000000000000000000000000000000000000003c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a49cd33edf133475ff62f463080000000000000000000000000000000000000000000000000000000000018ebd',
        'blockHash': '0x897a9d0561dd8bf37db48cb159260fa902f427e629cf1274abb00509dfb64307',
        'blockNumber': '0x14e6970',
        'blockTimestamp': '0x67bc5e1f',
        'transactionHash': '0xfc79f04b0af526a246e32e431c3db813a0cfa6bc9e60ec5eec555d3e238a461e',
        'transactionIndex': '0x8',
        'logIndex': '0x2c',
        'removed': False}
    
    result = log_processor.parse_uniswap_v4_initialize(log)
    
    expected = UniswapV4Initialize(
        pool_manager_address='0x000000000004444c5dc75cB358380D2e3dE08A90',
        event_id='0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3',
        currency0='0x0000000000000000000000000000000000000000',
        currency1='0x51ea52a6A885DD34f30494768cf1D9DC9f004185',
        fee=3000,
        tick_spacing=60,
        hooks='0x0000000000000000000000000000000000000000',
        sqrt_price_x96=13041953694121149609898616840968,  # Derived from hex value: "000000000000000000000000a49cd33edf133475ff62f46308000000"
        tick=102077,
        log_index=44
    )
    
    assert result == expected
    

def test_parse_permit2(log_processor):
    log = {'address': '0x000000000022d473030f116ddee9f6b43ac78ba3',
        'topics': ['0xc6a377bfc4eb120024a8ac08eef205be16b817020812c73223e81d1bdb9708ec',
        '0x00000000000000000000000084f607c948951b7bfe3edaefa47e192ca4d6a6aa',
        '0x00000000000000000000000051ea52a6a885dd34f30494768cf1d9dc9f004185',
        '0x000000000000000000000000bd216513d74c8cf14cf4747e6aaa6420ff64ee9e'],
        'data': '0x000000000000000000000000ffffffffffffffffffffffffffffffffffffffff0000000000000000000000000000000000000000000000000000000067e3eb0e0000000000000000000000000000000000000000000000000000000000000000',
        'blockHash': '0x897a9d0561dd8bf37db48cb159260fa902f427e629cf1274abb00509dfb64307',
        'blockNumber': '0x14e6970',
        'blockTimestamp': '0x67bc5e1f',
        'transactionHash': '0xfc79f04b0af526a246e32e431c3db813a0cfa6bc9e60ec5eec555d3e238a461e',
        'transactionIndex': '0x8',
        'logIndex': '0x2d',
        'removed': False}
    
    result = log_processor.parse_permit2(log)
    
    expected = Permit2(
        pool_manager_address='0x000000000022D473030F116dDEE9F6B43aC78BA3',
        owner='0x84f607C948951B7Bfe3EdaEFA47E192Ca4D6a6aA',
        token='0x51ea52a6A885DD34f30494768cf1D9DC9f004185',
        spender='0xbD216513d74C8cf14cf4747E6AaA6420FF64ee9e',
        amount='1461501637330902918203684832716283019655932542975',
        expiration=1742990094,
        nonce=0,
        log_index=45
    )
    
    assert result == expected
    

def test_parse_uniswap_v4_modify_liquidity(log_processor):

    log = {'address': '0x000000000004444c5dc75cb358380d2e3de08a90',
        'topics': ['0xf208f4912782fd25c7f114ca3723a2d5dd6f3bcc3ac8db5af63baa85f711d5ec',
        '0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3',
        '0x000000000000000000000000bd216513d74c8cf14cf4747e6aaa6420ff64ee9e'],
        'data': '0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffff2764c00000000000000000000000000000000000000000000000000000000000d89b4000000000000000000000000000000000000000000000084f66f4e166aa01e6400000000000000000000000000000000000000000000000000000000000010af',
        'blockHash': '0x897a9d0561dd8bf37db48cb159260fa902f427e629cf1274abb00509dfb64307',
        'blockNumber': '0x14e6970',
        'blockTimestamp': '0x67bc5e1f',
        'transactionHash': '0xfc79f04b0af526a246e32e431c3db813a0cfa6bc9e60ec5eec555d3e238a461e',
        'transactionIndex': '0x8',
        'logIndex': '0x2f',
        'removed': False}
            
    result = log_processor.parse_uniswap_v4_modify_liquidity(log)
    
    expected = UniswapV4ModifyLiquidity(
        pool_manager_address='0x000000000004444c5dc75cB358380D2e3dE08A90',
        event_id='0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3',
        sender='0xbD216513d74C8cf14cf4747E6AaA6420FF64ee9e',
        tick_lower=-887220,
        tick_upper=887220,
        liquidity_delta=2452727715443591093860,
        salt='00000000000000000000000000000000000000000000000000000000000010af',
        log_index=47
    )
    
    assert result == expected
    

def test_parse_uniswap_v4_swap(log_processor):
    """Test parsing of Uniswap V4 Swap event with negative amounts"""
    log = {'address': '0x000000000004444c5dc75cb358380d2e3de08a90',
        'topics': ['0x40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f',
        '0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3',
        '0x00000000000000000000000066a9893cc07d91d95644aedd05d03f95e1dba8af'],
        'data': '0xfffffffffffffffffffffffffffffffffffffffffffffffffecbd5bca6eaa21000000000000000000000000000000000000000000000007e3ddb0e9b8cd8c28d00000000000000000000000000000000000000a39f9ad35cc7c51bdf0a61efc4000000000000000000000000000000000000000000000084f66f4e166aa01e640000000000000000000000000000000000000000000000000000000000018e440000000000000000000000000000000000000000000000000000000000000bb8',
        'blockHash': '0x8430a7e5fb98fd16babeb750baad6f8683547bd526ab1e078cd2d1846aff9a5b',
        'blockNumber': '0x14e6d7e',
        'blockTimestamp': '0x67bc8ef7',
        'transactionHash': '0x7b7c846ef845c42a2f07919cf6ae955be63746f77d4be28307eaae27c47eaf1b',
        'transactionIndex': '0x13',
        'logIndex': '0xa5',
        'removed': False}
            
    result = log_processor.parse_uniswap_v4_swap(log)
    
    expected = UniswapV4Swap(
        pool_manager_address='0x000000000004444c5dc75cB358380D2e3dE08A90',
        event_id='0x56e7a2e2e41b7868e48c18a2b1de8f78b42c250d366c72485c8fe05edf65e4f3',
        sender='0x66a9893cC07D91D95644AEDD05D03f95e1dBA8Af',
        amount0=-86740761572630000,  
        amount1=2328746925604862476941,
        sqrt_price_x96=12963585779093724829272953188292,
        liquidity=2452727715443591093860,
        tick=101956,
        fee=3000,
        log_index=165
    )
    
    assert result == expected