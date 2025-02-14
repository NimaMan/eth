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
    
