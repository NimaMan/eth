from collections import defaultdict, OrderedDict
from eth_block_processor.utils.common_addresses import fee_recipients_set


class ProcessedTxStateDiffCalculator:
    """
    ProcessedTxStateDiffCalculator: Tracks and analyzes state changes in Ethereum transactions.

    This class calculates net state changes for addresses involved in transactions by:
    1. Tracking token and denomination (ETH/WETH) movements for each address
    2. Handling special cases like WETH conversions and bribes
    3. Filtering out insignificant state changes based on thresholds

    Key Features:
    - Maintains chronological order of transfers using OrderedDict
    - Tracks both incoming and outgoing movements for tokens and denominations
    - Handles special addresses (WETH, null, dead addresses)
    - Identifies significant state changes based on configurable thresholds
    - Excludes WETH conversions from denomination movements
    - Tracks bribe payments to known fee recipients
    """

    def __init__(
        self,
        denom_state_change_threshold: float = 0.0005,
        token_state_change_threshold: float = 0.1,
        logger=None,
    ):
        self.logger = logger
        self.WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        self.denom_state_change_threshold = denom_state_change_threshold
        self.token_state_change_threshold = token_state_change_threshold
        self.movements = {
            "token": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
            "denom": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
        }

    # ------------------------------------------------------------------ #
    # Internal helpers
    # ------------------------------------------------------------------ #
    def _track_movement(
        self,
        movement_type: str,
        from_addr: str,
        to_addr: str,
        amount: float,
        transfer_id: tuple,
    ):
        """Track a movement between addresses and handle special cases."""
        # WETH conversions are denomination-neutral
        if to_addr == self.WETH_ADDRESS or from_addr == self.WETH_ADDRESS:
            return
        # Skip deployer tx (log_index == '0')
        if transfer_id[2] == "0":
            return

        # Outgoing from `from_addr`
        self.movements[movement_type][from_addr]["out"][transfer_id] = amount

        # Incoming to `to_addr` (unless denom bribe to fee recipient)
        if not (movement_type == "denom" and to_addr in fee_recipients_set):
            self.movements[movement_type][to_addr]["in"][transfer_id] = amount

    # ------------------------------------------------------------------ #
    # Public API
    # ------------------------------------------------------------------ #
    def get_net_changes(self, from_address: str) -> dict:
        """
        Aggregate net changes for every address touched in this tx.

        Returns
        -------
        dict[address] -> {
            token_net : float,
            denom_net : float,
            movements : {token:{in/out}, denom:{in/out}}
        }
        """
        net = {}
        addrs = set(self.movements["token"]) | set(self.movements["denom"])
        for addr in addrs:
            token_in = sum(self.movements["token"][addr]["in"].values())
            token_out = sum(self.movements["token"][addr]["out"].values())
            denom_in = sum(self.movements["denom"][addr]["in"].values())
            denom_out = sum(self.movements["denom"][addr]["out"].values())

            token_net = token_in - token_out
            denom_net = denom_in - denom_out

            if (
                abs(token_net) > self.token_state_change_threshold
                or abs(denom_net) > self.denom_state_change_threshold
                or addr == from_address
            ):
                net[addr] = {
                    "token_net": token_net,
                    "denom_net": denom_net,
                    "movements": {
                        "token": self.movements["token"][addr],
                        "denom": self.movements["denom"][addr],
                    },
                }
        return net

    # ------------------------------------------------------------------ #
    # Convenience entry points
    # ------------------------------------------------------------------ #
    def calculate_state_changes(
        self,
        txn_hash: str,
        from_address: str,
        block_number: int,
        txn_index: int,
        eth_transfers: list,
        erc20_transfers: list,
    ) -> dict:
        """
        Build movements directly from raw transfer lists
        (legacy entry point).
        """
        self.movements = {
            "token": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
            "denom": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
        }

        # ERC-20 transfers
        for tr in erc20_transfers:
            tid = (block_number, txn_index, tr["log_index"])
            is_weth = tr["token_address"] == self.WETH_ADDRESS
            mtype = "denom" if is_weth else "token"
            amount = (
                tr["amount"] / 1e18 if is_weth else tr["amount"]
            )
            self._track_movement(
                mtype, tr["from_address"], tr["to_address"], amount, tid
            )

        # ETH (internal) transfers
        for tr in eth_transfers:
            log_index_or_depth = tr.get("log_index") or tr.get('depth')
            tid = (block_number, txn_index, log_index_or_depth)
            self._track_movement(
                "denom", tr["from_address"], tr["to_address"], tr["amount"], tid
            )

        try:
            return self.get_net_changes(from_address)
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    f"State diff error for {txn_hash}: {exc}", exc_info=True
                )
            return {}

    def calculate_state_changes_from_processed_tx(self, processed_tx) -> dict:
        """
        Preferred path: use a `ProcessedTransaction` object that already contains
        parsed ERC-20 and internal transfers.
        """
        self.movements = {
            "token": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
            "denom": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
        }

        bn, txi = processed_tx.block_number, processed_tx.txn_index

        # 1️⃣ ERC-20 transfers
        for tr in processed_tx.erc20_transfers:
            tid = (bn, txi, tr.log_index)
            is_weth = tr.token_address == self.WETH_ADDRESS
            mtype = "denom" if is_weth else "token"
            amount = float(tr.amount) / 1e18 if is_weth else float(tr.amount)
            self._track_movement(
                mtype, tr.from_address, tr.to_address, amount, tid
            )

        # 2️⃣ Internal ETH transfers
        for it in processed_tx.internal_transactions:
            tid = (bn, txi, it.depth)
            self._track_movement(
                "denom", it.from_address, it.to_address, float(it.value), tid
            )

        try:
            return self.get_net_changes(processed_tx.from_address)
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    f"State diff calc failed for {processed_tx.hash}: {exc}",
                    exc_info=True,
                )
            return {}