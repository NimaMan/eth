import pytest
from hexbytes import HexBytes
from web3 import Web3

from eth_data.tx_processor.tx_log_processor import TransactionLogProcessor
from eth_data.tx_processor.data_models.receipt_models import (
    DepositEvent,
    ERC20ApprovalEvent,
    Permit2Event,
    UniswapV2MintEvent,
    UniswapV2PairCreatedEvent,
    UniswapV2SwapEvent,
    UniswapV2SyncEvent,
    UniswapV3InitializeEvent,
    UniswapV4BalanceDeltaEvent,
    UniswapV4DonateEvent,
    UniswapV4DynamicLPFeeUpdatedEvent,
    UniswapV4FeeControllerUpdatedEvent,
    UniswapV4InitializeEvent,
    UniswapV4ModifyLiquidityEvent,
    UniswapV4FeeUpdatedEvent,
    UniswapV4SwapEvent,
)


def _word_uint(value: int) -> str:
    return f"{value & ((1 << 256) - 1):064x}"


def _word_int(value: int) -> str:
    if value < 0:
        value = (1 << 256) + value
    return f"{value & ((1 << 256) - 1):064x}"


def _word_address(address: str) -> str:
    return f"{'0'*24}{address.lower().replace('0x', '')}"


@pytest.fixture
def log_processor(w3):
    return TransactionLogProcessor(w3)


def test_parse_uniswap_v2_sync(log_processor):
    log = {
        "address": "0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0",
        "topics": [
            HexBytes("0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1")
        ],
        "data": HexBytes(
            "0x00000000000000000000000000000000000000000000000d7fbbe35020a4023b"
            "0000000000000000000000000000000000000000000000015c54ae87720eb000"
        ),
        "logIndex": 136,
    }

    result = log_processor.parse_uniswap_v2_sync(log)

    assert result == UniswapV2SyncEvent(
        pair_address="0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0",
        reserve0=249011873154970419771,
        reserve1=25099878520000000000,
        log_index=136,
    )


def test_parse_uniswap_v2_swap(log_processor):
    log = {
        "address": "0x52fd104d206ee67f5d26bb66f64781af887b2eb0",
        "topics": [
            HexBytes("0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"),
            HexBytes("0x0000000000000000000000007d217ffc52898191d97ef38ef448c8f46ce880e2"),
            HexBytes("0x0000000000000000000000007d217ffc52898191d97ef38ef448c8f46ce880e2"),
        ],
        "data": HexBytes(
            "0x0000000000000000000000000000000000000000000000000898dbc275352a00"
            "0000000000000000000000000000000000000000000000000000000000000000"
            "0000000000000000000000000000000000000000000000000000000000000000"
            "00000000000000000000000000000000000000000000000000ddba6063614200"
        ),
        "logIndex": 137,
    }

    result = log_processor.parse_uniswap_v2_swap(log)

    assert result == UniswapV2SwapEvent(
        pair_address="0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0",
        sender="0x7d217fFC52898191D97eF38EF448c8F46Ce880e2",
        to="0x7d217fFC52898191D97eF38EF448c8F46Ce880e2",
        amount0In=619486577000000000,
        amount1In=0,
        amount0Out=0,
        amount1Out=62410893000000000,
        log_index=137,
    )


def test_parse_uniswap_v2_pair_created_event(log_processor):
    log = {
        "address": "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f",
        "topics": [
            "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9",
            "0x0000000000000000000000002551bc3f26129019624f1fb09ebb880e94dbc22a",
            "0x000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        ],
        "data": "0x00000000000000000000000052fd104d206ee67f5d26bb66f64781af887b2eb0000000000000000000000000000000000000000000000000000000000006259e",
        "blockHash": "0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4",
        "blockNumber": "0x14d4b37",
        "blockTimestamp": "0x67aeda53",
        "transactionHash": "0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66",
        "transactionIndex": "0x8d",
        "logIndex": "0x111",
        "removed": False,
    }

    result = log_processor.parse_uniswap_v2_pair_created_event(log)

    assert result == UniswapV2PairCreatedEvent(
        pair_address="0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0",
        token0="0x2551Bc3f26129019624F1fB09ebB880E94dBc22A",
        token1="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        log_index=273,
    )


def test_parse_erc20_approval(log_processor):
    log = {
        "address": "0x2551bc3f26129019624f1fb09ebb880e94dbc22a",
        "topics": [
            "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925",
            "0x000000000000000000000000a51b1c4766f0fc9409525784b3bf62385ddfa08d",
            "0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d",
        ],
        "data": "0xfffffffffffffffffffffffffffffffffffffffffffffff2728d948e8857ffff",
        "blockHash": "0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4",
        "blockNumber": "0x14d4b37",
        "blockTimestamp": "0x67aeda53",
        "transactionHash": "0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66",
        "transactionIndex": "0x8d",
        "logIndex": "0x113",
        "removed": False,
    }

    result = log_processor.parse_approve(log)

    assert result == ERC20ApprovalEvent(
        token_address="0x2551Bc3f26129019624F1fB09ebB880E94dBc22A",
        owner="0xa51B1C4766F0fC9409525784b3BF62385dDfA08d",
        spender="0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
        amount=115792089237316195423570985008687907853269984665640564039207584007913129639935,
        log_index=275,
    )


def test_parse_deposit_event(log_processor):
    log = {
        "address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        "topics": [
            "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c",
            "0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d",
        ],
        "data": "0x0000000000000000000000000000000000000000000000015af1d78b58c40000",
        "blockHash": "0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4",
        "blockNumber": "0x14d4b37",
        "blockTimestamp": "0x67aeda53",
        "transactionHash": "0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66",
        "transactionIndex": "0x8d",
        "logIndex": "0x114",
        "removed": False,
    }

    result = log_processor.parse_deposit_event(log)

    assert result == DepositEvent(
        pair_address="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        sender="0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
        amount=25000000000000000000,
        log_index=276,
    )


def test_parse_uniswap_v2_mint_event(log_processor):
    log = {
        "address": "0x52fd104d206ee67f5d26bb66f64781af887b2eb0",
        "topics": [
            "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f",
            "0x0000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d",
        ],
        "data": "0x00000000000000000000000000000000000000000000000d8d726b7177a800000000000000000000000000000000000000000000000000015af1d78b58c40000",
        "blockHash": "0x7934151304dfd2d493005eedbda0fc969c80c527f74b4c8d95af06e09b3035a4",
        "blockNumber": "0x14d4b37",
        "blockTimestamp": "0x67aeda53",
        "transactionHash": "0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66",
        "transactionIndex": "0x8d",
        "logIndex": "0x119",
        "removed": False,
    }

    result = log_processor.parse_uniswap_v2_mint_event(log)

    assert result == UniswapV2MintEvent(
        pair_address="0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0",
        sender="0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
        amount0=250000000000000000000,
        amount1=25000000000000000000,
        log_index=281,
    )


def test_parse_uniswap_v3_initialize_event(log_processor):
    log = {
        "address": "0xbe931909a485643acdb7eef4184c1284cd8efe8f",
        "topics": ["0x98636036cb66a9c19a37435efc1e90142190214e8abeb821bdba3f2990dd4c95"],
        "data": "0x0000000000000000000000000000000000000000000010dc8023230f0bb0a246fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffbc900",
        "blockHash": "0xa33211cc390ecf352ec46ec92fb4d6e8f3da0838ea149956d4434155b7753d1c",
        "blockNumber": "0x14ee285",
        "blockTimestamp": "0x67c211f3",
        "transactionHash": "0xcb878cb2353c10f70837e2bfd3b41b16104b9e08d280ddbf461f2e70a2918719",
        "transactionIndex": "0x69",
        "logIndex": "0xc8",
        "removed": False,
    }

    result = log_processor.parse_uniswap_v3_initialize(log)

    assert result == UniswapV3InitializeEvent(
        pool_address="0xbe931909a485643ACdb7EeF4184c1284cd8eFe8f",
        sqrt_price_x96=79625380684338992030278,
        tick=-276224,
        log_index=200,
    )


def test_parse_permit2_event(log_processor):
    owner = "0x0123456789aBCdEf0123456789aBCdEf01234567"
    token = "0xfedcba9876543210fedcba9876543210fedcba98"
    spender = "0x000000000000000000000000000000000000dead"
    amount = 10**18
    expiration = 2**40
    nonce = 12345

    log = {
        "address": "0x000000000022d473030f116ddee9f6b43ac78ba3",
        "topics": [
            "0xc6a377bfc4eb120024a8ac08eef205be16b817020812c73223e81d1bdb9708ec",
            f"0x{_word_address(owner)}",
            f"0x{_word_address(token)}",
            f"0x{_word_address(spender)}",
        ],
        "data": f"0x{_word_uint(amount)}{_word_uint(expiration)}{_word_uint(nonce)}",
        "logIndex": 42,
    }

    result = log_processor.parse_permit2_event(log)

    assert result == Permit2Event(
        pool_manager_address=Web3.to_checksum_address("0x000000000022d473030f116ddee9f6b43ac78ba3"),
        owner=Web3.to_checksum_address(owner),
        token=Web3.to_checksum_address(token),
        spender=Web3.to_checksum_address(spender),
        amount=amount,
        expiration=expiration,
        nonce=nonce,
        log_index=42,
    )


def test_parse_uniswap_v4_initialize_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    currency0 = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
    currency1 = "0xDee6cDd28Da9f51e3A8421395973894a884F3B2D"
    hooks = "0x0000000000000000000000000000000000001337"
    event_id = "0x01" + "0"*62
    fee = 3000
    tick_spacing = 200
    sqrt_price = 2**96
    tick = 120

    data = (
        "0x"
        f"{_word_uint(fee)}"
        f"{_word_int(tick_spacing)}"
        f"{_word_address(hooks)}"
        f"{_word_uint(sqrt_price)}"
        f"{_word_int(tick)}"
    )
    log = {
        "address": pool_manager,
        "topics": [
            "0x98636036cb66a9c19a37435efc1e90142190214e8abeb821bdba3f2990dd4c95",
            event_id,
            f"0x{_word_address(currency0)}",
            f"0x{_word_address(currency1)}",
        ],
        "data": data,
        "logIndex": 7,
    }

    result = log_processor.parse_uniswap_v4_initialize_event(log)

    assert result == UniswapV4InitializeEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        event_id=event_id,
        currency0=Web3.to_checksum_address(currency0),
        currency1=Web3.to_checksum_address(currency1),
        fee=fee,
        tick_spacing=tick_spacing,
        hooks=Web3.to_checksum_address(hooks),
        sqrt_price_x96=sqrt_price,
        tick=tick,
        log_index=7,
    )


def test_parse_uniswap_v4_modify_liquidity_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    event_id = "0x02" + "0"*62
    sender = "0x000000000000000000000000000000000000beef"
    tick_lower = -100
    tick_upper = 100
    liquidity_delta = 123456789
    salt = "0x" + "11"*32

    data = (
        "0x"
        f"{_word_int(tick_lower)}"
        f"{_word_int(tick_upper)}"
        f"{_word_int(liquidity_delta)}"
        f"{salt[2:]}"
    )
    log = {
        "address": pool_manager,
        "topics": [
            "0x8dfd2f8b835e923decf3b38cb7c6063954050d805cc19c239293a1d38ff7b8bd",
            event_id,
            f"0x{_word_address(sender)}",
        ],
        "data": data,
        "logIndex": 8,
    }

    result = log_processor.parse_uniswap_v4_modify_liquidity_event(log)

    assert result == UniswapV4ModifyLiquidityEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        event_id=event_id,
        sender=Web3.to_checksum_address(sender),
        tick_lower=tick_lower,
        tick_upper=tick_upper,
        liquidity_delta=liquidity_delta,
        salt=salt[2:],
        log_index=8,
    )


def test_parse_uniswap_v4_swap_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    event_id = "0x03" + "0"*62
    sender = "0x000000000000000000000000000000000000bEEF"
    amount0 = -5000
    amount1 = 4000
    sqrt_price = 2**95
    liquidity = 987654321
    tick = -50
    fee = 100

    data = (
        "0x"
        f"{_word_int(amount0)}"
        f"{_word_int(amount1)}"
        f"{_word_uint(sqrt_price)}"
        f"{_word_uint(liquidity)}"
        f"{_word_int(tick)}"
        f"{_word_uint(fee)}"
    )
    log = {
        "address": pool_manager,
        "topics": [
            "0xf994aeb5d5195c8a2a1beb3a0a4d0b87aea7df5c8eddd9d0d3bf1aaadbcd5474",
            event_id,
            f"0x{_word_address(sender)}",
        ],
        "data": data,
        "logIndex": 9,
    }

    result = log_processor.parse_uniswap_v4_swap_event(log)

    assert result == UniswapV4SwapEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        event_id=event_id,
        sender=Web3.to_checksum_address(sender),
        amount0=amount0,
        amount1=amount1,
        sqrt_price_x96=sqrt_price,
        liquidity=liquidity,
        tick=tick,
        fee=fee,
        log_index=9,
    )


def test_parse_uniswap_v4_donate_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    event_id = "0x04" + "0"*62
    sender = "0x1111111111111111111111111111111111111111"
    amount0 = 123
    amount1 = 456

    data = (
        "0x"
        f"{_word_int(amount0)}"
        f"{_word_int(amount1)}"
    )
    log = {
        "address": pool_manager,
        "topics": [
            "0x3d73afa950f8d028b5bd04bf9f955a192d08bc7569b0ce3f4bfefd4dc262a7e9",
            event_id,
            f"0x{_word_address(sender)}",
        ],
        "data": data,
        "logIndex": 10,
    }

    result = log_processor.parse_uniswap_v4_donate_event(log)

    assert result == UniswapV4DonateEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        event_id=event_id,
        sender=Web3.to_checksum_address(sender),
        amount0=amount0,
        amount1=amount1,
        log_index=10,
    )


def test_parse_uniswap_v4_fee_updated_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    event_id = "0x05" + "0"*62
    protocol_fee = 25

    log = {
        "address": pool_manager,
        "topics": [
            "0xe1fbecdccaba96b47f8a166edc16fef3ac280d1b3da8932e8def5a0f32711586",
            event_id,
        ],
        "data": f"0x{_word_uint(protocol_fee)}",
        "logIndex": 11,
    }

    result = log_processor.parse_uniswap_v4_fee_updated_event(log)

    assert result == UniswapV4FeeUpdatedEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        event_id=event_id,
        protocol_fee=protocol_fee,
        log_index=11,
    )


def test_parse_uniswap_v4_dynamic_lp_fee_updated_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    event_id = "0x06" + "0"*62
    dynamic_fee = 50

    log = {
        "address": pool_manager,
        "topics": [
            "0x41e7dbcf37ca6f06083f652f9c8ff7ffef906a032725ccb2840c362ceb5c7652",
            event_id,
        ],
        "data": f"0x{_word_uint(dynamic_fee)}",
        "logIndex": 12,
    }

    result = log_processor.parse_uniswap_v4_dynamic_lp_fee_updated_event(log)

    assert result == UniswapV4DynamicLPFeeUpdatedEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        event_id=event_id,
        dynamic_lp_fee=dynamic_fee,
        log_index=12,
    )


def test_parse_uniswap_v4_fee_controller_updated_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    controller = "0x2222222222222222222222222222222222222222"
    log = {
        "address": pool_manager,
        "topics": [
            "0x4f7cfa886c53de87dd91819db3924df83816f2ae49317029ab6e0230011f9248",
            f"0x{_word_address(controller)}",
        ],
        "data": "0x",
        "logIndex": 13,
    }

    result = log_processor.parse_uniswap_v4_fee_controller_updated_event(log)

    assert result == UniswapV4FeeControllerUpdatedEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        protocol_fee_controller=Web3.to_checksum_address(controller),
        log_index=13,
    )


def test_parse_uniswap_v4_balance_delta_event(log_processor):
    pool_manager = "0x0000000000000000000000000000000000009999"
    pool_id = "0x07" + "0"*62
    settler = "0x3333333333333333333333333333333333333333"
    delta0 = -111
    delta1 = 222

    data = "0x" f"{_word_int(delta0)}{_word_int(delta1)}"
    log = {
        "address": pool_manager,
        "topics": [
            "0x2a5dfe76f8864c8d7495121fba9b5d1cfbf300f0d5c91cce0ed6f3d1063f4048",
            pool_id,
            f"0x{_word_address(settler)}",
        ],
        "data": data,
        "logIndex": 14,
    }

    result = log_processor.parse_uniswap_v4_balance_delta_event(log)

    assert result == UniswapV4BalanceDeltaEvent(
        pool_manager_address=Web3.to_checksum_address(pool_manager),
        pool_id=pool_id,
        settler=Web3.to_checksum_address(settler),
        delta0=delta0,
        delta1=delta1,
        log_index=14,
    )
