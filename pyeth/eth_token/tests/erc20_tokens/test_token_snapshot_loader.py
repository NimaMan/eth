from collections import defaultdict

from eth_token.erc20_token.token_snapshot import (
    TokenSnapshot,
    build_token_snapshot,
    load_token_snapshot,
)
from eth_token.erc20_token.erc20_token import TokenLifecycleState


class DummyTransferTracker:
    def __init__(self):
        self.approved_addresses = {"0xabc"}
        self.address_tx_counter = defaultdict(int)
        self.erc20_transfers = {}
        self.eth_transfers = {}
        self.other_denom_transfers = {}
        self.approvals = []


class DummyToken:
    def __init__(self):
        self.contract_address = "0x123"
        self.name = "Token"
        self.symbol = "TKN"
        self.decimals = 18
        self.total_supply = 1_000_000
        self.creation_block = 1
        self.creation_timestamp = 2
        self.creation_tx = "0xcreation"
        self.creator_address = "0xcreator"
        self.creator_nonce = 1
        self.latest_block_number = 10
        self.latest_block_timestamp = 20
        self.token_life_cycle_status = TokenLifecycleState.TRADING_ENABLED
        self.is_scam = False
        self.has_pool = True
        self.trading_enabled = True
        self.trading_enabled_block = 5
        self.trading_enabled_tx = "0xtrade"
        self.ownership_renounced = False
        self.current_owner = "0xowner"
        self.ownership_renounced_block = None
        self.token_control_addresses = {"0xowner"}
        self.pool_addresses = ("0xpool",)
        self.all_pool_reserves = {"0xpool": {"token_reserve": 100}}
        self.current_prices = {"0xpool": {"price": 1.0}}
        self.total_liquidity_by_denom = {"ETH": 10.0}
        self.latest_pools_price_ratio = {"0xpool": 1.1}
        self.total_bribe_amount = 0
        self.bribe_amounts_by_tx = {}
        self.transfer_tracker = DummyTransferTracker()

    def get_pool_info_dict(self):
        return {"0xpool": {"pool_type": "UNISWAP-V2"}}


class DummyReader:
    def __init__(self, snapshot):
        self.snapshot = snapshot
        self.requests = []

    def get_token_snapshot(self, token_address):
        self.requests.append(token_address)
        return self.snapshot


def test_token_snapshot_from_dict_roundtrip():
    dummy = DummyToken()
    snapshot_dict = build_token_snapshot(dummy)
    token_snapshot = TokenSnapshot.from_dict(snapshot_dict)

    assert token_snapshot.contract_address == snapshot_dict["contract_address"]
    assert token_snapshot.metadata["symbol"] == snapshot_dict["metadata"]["symbol"]
    assert token_snapshot.to_dict() == snapshot_dict


def test_load_token_snapshot_uses_reader():
    dummy = DummyToken()
    snapshot_dict = build_token_snapshot(dummy)
    reader = DummyReader(snapshot_dict)

    snap = load_token_snapshot(dummy.contract_address, reader=reader)

    assert snap is not None
    assert reader.requests == [dummy.contract_address]
    assert snap.contract_address == dummy.contract_address
