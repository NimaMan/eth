from typing import Dict, Any, List
from web3 import Web3
from decimal import Decimal
from eth_block_processor.data_models.receipt_models import *


class TransactionLogAnalyzer:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.erc20_transfer_topic = self.w3.keccak(text="Transfer(address,address,uint256)").hex()
        self.erc721_transfer_topic = self.w3.keccak(text="Transfer(address,address,uint256)").hex()
        self.erc1155_transfer_single_topic = self.w3.keccak(text="TransferSingle(address,address,address,uint256,uint256)").hex()
        self.erc1155_transfer_batch_topic = self.w3.keccak(text="TransferBatch(address,address,address,uint256[],uint256[])").hex()
        self.uniswap_v2_sync_topic = self.w3.keccak(text="Sync(uint112,uint112)").hex()
        self.uniswap_v2_swap_topic = self.w3.keccak(text="Swap(address,uint256,uint256,uint256,uint256,address)").hex()
        self.approve_signature = self.w3.keccak(text="Approval(address,address,uint256)").hex()
        self.mint_signature = self.w3.keccak(text="Mint(address,uint256,uint256)").hex()
        self.burn_signature = self.w3.keccak(text="Burn(address,uint256,uint256,address)").hex()
        self.deposit_signature = self.w3.keccak(text="Deposit(address,uint256)").hex()
        self.withdraw_signature = self.w3.keccak(text="Withdraw(address,uint256)").hex()
        self.weth_withdrawal_topic = self.w3.keccak(text="Withdrawal(address,uint256)").hex()
        self.pair_signature = self.w3.keccak(text="PairCreated(address,address,address,uint256)").hex()
        self.owner_signature = self.w3.keccak(text="OwnershipTransferred(address,address)").hex()
        self.trading_enabled_signature = self.w3.keccak(text="TradingEnabled(uint256)").hex()
        self.trading_disabled_signature = self.w3.keccak(text="TradingDisabled(uint256)").hex()

        # Additional events
        self.uniswap_v2_collect_topic = self.w3.keccak(text="Collect(address,address,uint256,uint256)").hex()
        self.uniswap_v2_flash_topic = self.w3.keccak(text="Flash(address,uint256,uint256,uint256,uint256)").hex()
        self.uniswap_v2_increase_observation_cardinality_next_topic = self.w3.keccak(text="IncreaseObservationCardinalityNext(uint16,uint16)").hex()
        self.uniswap_v2_set_fee_protocol_topic = self.w3.keccak(text="SetFeeProtocol(uint8,uint8)").hex()
        self.uniswap_v2_collect_protocol_topic = self.w3.keccak(text="CollectProtocol(address,address,uint128,uint128)").hex()

        self.exclude_from_fees_signature = self.w3.keccak(text="ExcludeFromFees(address,bool)").hex()
        self.exclude_from_limits_signature = self.w3.keccak(text="ExcludeFromLimits(address,bool)").hex()
        self.set_max_tx_amount_signature = self.w3.keccak(text="SetMaxTxAmount(uint256)").hex()
        self.set_max_wallet_token_signature = self.w3.keccak(text="SetMaxWalletToken(uint256)").hex()
        self.set_max_wallet_signature = self.w3.keccak(text="SetMaxWallet(uint256)").hex()
        
        # Uniswap V3 Pool events
        self.uniswap_v3_initialize_topic = self.w3.keccak(text="Initialize(uint160,int24)").hex()
        self.uniswap_v3_mint_topic = self.w3.keccak(text="Mint(address,address,int24,int24,uint128,uint256,uint256)").hex()
        self.uniswap_v3_burn_topic = self.w3.keccak(text="Burn(address,int24,int24,uint128,uint256,uint256)").hex()
        self.uniswap_v3_swap_topic = self.w3.keccak(text="Swap(address,address,int256,int256,uint160,uint128,int24)").hex()

        # Uniswap V3 Factory events
        self.uniswap_v3_pool_created_topic = self.w3.keccak(text="PoolCreated(address,address,uint24,int24,address)").hex()

        # Uniswap V3 NFT Manager events
        self.uniswap_v3_increase_liquidity_topic = self.w3.keccak(text="IncreaseLiquidity(uint256,uint128,uint256,uint256)").hex()
        self.uniswap_v3_decrease_liquidity_topic = self.w3.keccak(text="DecreaseLiquidity(uint256,uint128,uint256,uint256)").hex()

    def analyze_logs(self, logs: List[Dict[str, Any]]) -> Dict[str, List[Any]]:
        result = {
            'erc20_transfers': [],
            'erc721_transfers': [],
            'erc1155_transfers': [],
            'uniswap_v2_syncs': [],
            'uniswap_v2_swaps': [],
            'mints': [],
            'burns': [],
            'deposits': [],
            'withdraws': [],
            'pair_events': [],
            'approvals': [],
            'owner_events': [],
            'trading_enabled_events': [],
            'trading_disabled_events': [],
            'other_events': [],
            'unique_addresses': set(),
            'erc20_contracts': set(),
        }
        
        for log in logs:
            try:
                event = self.classify_and_parse_log(log)
            except Exception as e:
                print(f"Error parsing log: {log}")
                raise e
            if isinstance(event, ERC20Transfer):
                result['erc20_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, ERC721Transfer):
                result['erc721_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
            elif isinstance(event, ERC1155Transfer):
                result['erc1155_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
            elif isinstance(event, UniswapV2Sync):
                result['uniswap_v2_syncs'].append(event)
                result['unique_addresses'].add(event.pair_address)
            elif isinstance(event, UniswapV2Swap):
                result['uniswap_v2_swaps'].append(event)
                result['unique_addresses'].add(event.sender)
                result['unique_addresses'].add(event.to)
                result['unique_addresses'].add(event.pair_address)
            elif isinstance(event, MintAction):
                result['mints'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, BurnAction):
                result['burns'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, DepositAction):
                result['deposits'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, WithdrawAction):
                result['withdraws'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, PairAction):
                result['pair_events'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.token0)
                result['unique_addresses'].add(event.token1)
            elif isinstance(event, ERC20Approval):
                result['approvals'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['unique_addresses'].add(event.owner)
                result['unique_addresses'].add(event.spender)
            elif isinstance(event, ERC721Approval):
                result['approvals'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['unique_addresses'].add(event.owner)
                result['unique_addresses'].add(event.approved_address)
            elif isinstance(event, OwnerEvent):
                result['owner_events'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.previous_owner)
                result['unique_addresses'].add(event.new_owner)
            elif isinstance(event, TradingEnabledEvent):
                result['trading_enabled_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, TradingDisabledEvent):
                result['trading_disabled_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['erc20_contracts'].add(event.token_address)
            else:
                result['other_events'].append(event)
        
        return result

    def classify_and_parse_log(self, log: Dict[str, Any]) -> Any:
        if not log['topics']:
            return None
        topic = log['topics'][0].hex() if isinstance(log['topics'][0], bytes) else log['topics'][0]
        try:
            if topic == self.erc20_transfer_topic:
                return self.parse_erc20_transfer(log)
            elif topic == self.erc721_transfer_topic:
                return self.parse_erc721_transfer(log)
            elif topic == self.erc1155_transfer_single_topic:
                return self.parse_erc1155_single_transfer(log)
            elif topic == self.erc1155_transfer_batch_topic:
                return self.parse_erc1155_batch_transfer(log)
            elif topic == self.uniswap_v2_sync_topic:
                return self.parse_uniswap_v2_sync(log)
            elif topic == self.uniswap_v2_swap_topic:
                return self.parse_uniswap_v2_swap(log)
            elif topic == self.mint_signature:
                return self.parse_mint(log)
            elif topic == self.approve_signature:
                return self.parse_approve(log)
            elif topic == self.burn_signature:
                return self.parse_burn(log)
            elif topic == self.deposit_signature:
                return self.parse_deposit(log)
            elif topic == self.withdraw_signature or topic == self.weth_withdrawal_topic:
                return self.parse_withdraw(log)
            elif topic == self.pair_signature:
                return self.parse_pair(log)
            elif topic == self.owner_signature:
                return self.parse_owner(log)
            elif topic == self.trading_enabled_signature:
                return self.parse_trading_enabled(log)
            elif topic == self.trading_disabled_signature:
                return self.parse_trading_disabled(log)
            else:
                return self.parse_other_event(log)
        except Exception as e:
            return self.parse_other_event(log)

    def parse_erc20_transfer(self, log: Dict[str, Any]) -> ERC20Transfer:
        """Parse ERC20 Transfer event log"""
        # Handle both hex string and bytes data formats
        data = log['data']
        if isinstance(data, bytes):
            data = data.hex()
        if not data.startswith('0x'):
            data = '0x' + data
        
        return ERC20Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if isinstance(log['topics'][1], bytes) else self.w3.to_checksum_address(log['topics'][1][-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if isinstance(log['topics'][2], bytes) else self.w3.to_checksum_address(log['topics'][2][-40:]),
            amount=int(data, 16) if data != '0x' else 0,
            log_index=log['logIndex']
        )

    def parse_erc721_transfer(self, log: Dict[str, Any]) -> ERC721Transfer:
        return ERC721Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if len(log['topics']) > 1 else None,
            to_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if len(log['topics']) > 2 else None,
            token_id=int(log['topics'][3].hex(), 16) if len(log['topics']) > 3 and log['topics'][3].hex() != '' else 0,
            log_index=log['logIndex'],
        )

    def parse_erc1155_single_transfer(self, log: Dict[str, Any]) -> ERC1155Transfer:
        data = log['data'].hex()
        return ERC1155Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            operator=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            from_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][3].hex()[-40:]),
            token_ids=[int(data[:64], 16) if data[:64] else 0],
            amounts=[int(data[64:128], 16) if len(data) >= 128 and data[64:128] else 0],
            log_index=log['logIndex'],
        )

    def parse_erc1155_batch_transfer(self, log: Dict[str, Any]) -> ERC1155Transfer:
        data = log['data'].hex()
        ids_offset = int(data[:66], 16) * 2
        amounts_offset = int(data[66:130], 16) * 2
        ids_length = int(data[ids_offset:ids_offset+64], 16) if ids_offset+64 < len(data) else 0
        amounts_length = int(data[amounts_offset:amounts_offset+64], 16) if amounts_offset+64 < len(data) else 0
        
        ids = [int(data[i:i+64], 16) for i in range(ids_offset+64, ids_offset+64+(ids_length*64), 64)] if ids_length > 0 else []
        amounts = [int(data[i:i+64], 16) for i in range(amounts_offset+64, amounts_offset+64+(amounts_length*64), 64)] if amounts_length > 0 else []
        
        return ERC1155Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            operator=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            from_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][3].hex()[-40:]),
            token_ids=ids,
            amounts=amounts,
            log_index=log['logIndex'],
        )
    
    def parse_uniswap_v2_sync(self, log: Dict[str, Any]) -> UniswapV2Sync:
        return UniswapV2Sync(
            pair_address=self.w3.to_checksum_address(log['address']),
            reserve0=int(log['data'].hex()[:64], 16) if log['data'].hex()[:64] != '' else 0,
            reserve1=int(log['data'].hex()[64:], 16) if log['data'].hex()[64:] != '' else 0,
            log_index=log['logIndex'],
        )
    
    def parse_uniswap_v2_swap(self, log: Dict[str, Any]) -> UniswapV2Swap:
        return UniswapV2Swap(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            to=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            amount0In=int(log['data'].hex()[:64], 16) if log['data'].hex()[:64] != '' else 0,
            amount1In=int(log['data'].hex()[64:128], 16) if log['data'].hex()[64:128] != '' else 0,
            amount0Out=int(log['data'].hex()[128:192], 16) if log['data'].hex()[128:192] != '' else 0,
            amount1Out=int(log['data'].hex()[192:], 16) if log['data'].hex()[192:] != '' else 0,
            log_index=log['logIndex'],
        )

    def parse_approve(self, log: Dict[str, Any]) -> ERC20Approval:
        return ERC20Approval(
            token_address=self.w3.to_checksum_address(log['address']),
            owner=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if len(log['topics']) > 1 else None,
            spender=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if len(log['topics']) > 2 else None,
            amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
            log_index=log['logIndex'],
        )
    
    def parse_erc721_approval(self, log: Dict[str, Any]) -> ERC721Approval:
        return ERC721Approval(
            token_address=self.w3.to_checksum_address(log['address']),
            owner=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            approved=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            token_id=int(log['topics'][3].hex(), 16) if log['topics'][3].hex() != '' else 0,
            log_index=log['logIndex'],
        )
    
    def parse_mint(self, log: Dict[str, Any]) -> MintAction:
        return MintAction(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            amount0=int(log['data'].hex()[:64], 16) if log['data'].hex()[:64] != '' else 0,
            amount1=int(log['data'].hex()[64:], 16) if log['data'].hex()[64:] != '' else 0,
            log_index=log['logIndex'],
        )
    
    def parse_burn(self, log: Dict[str, Any]) -> BurnAction:
        return BurnAction(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
            log_index=log['logIndex'],
        )
    
    def parse_deposit(self, log: Dict[str, Any]) -> DepositAction:
        if len(log['topics']) > 1:
            # This is likely a WETH deposit
            return DepositAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
                amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
                log_index=log['logIndex'],
            )
        else:
            # This is likely the custom deposit
            data = log['data']
            sender = None # figure out how to get this
            amount = int(data[66:], 16) if data[66:] else None
            return DepositAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=sender,
                amount=amount,
                log_index=log['logIndex'],
            )
    
    def parse_withdraw(self, log: Dict[str, Any]) -> WithdrawAction:
        if len(log['topics']) > 1:
            return WithdrawAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
                amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
                log_index=log['logIndex'],
            )
        else:
            return WithdrawAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=None,
                amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
                log_index=log['logIndex'],
            )
    
    def parse_pair(self, log: Dict[str, Any]) -> PairAction:
        return PairAction(
            pair_address=self.w3.to_checksum_address(log['address']),
            token0=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            token1=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            log_index=log['logIndex'],
        )
    
    def parse_owner(self, log: Dict[str, Any]) -> OwnerEvent:
        return OwnerEvent(
            pair_address=self.w3.to_checksum_address(log['address']),
            previous_owner=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            new_owner=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            log_index=log['logIndex'],
        )
    
    def parse_trading_enabled(self, log: Dict[str, Any]) -> TradingEnabledEvent:
        return TradingEnabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=int(log['topics'][1].hex(), 16) if log['topics'][1].hex() != '' else 0,
            log_index=log['logIndex'],
        )
    
    def parse_trading_disabled(self, log: Dict[str, Any]) -> TradingDisabledEvent:  
        return TradingDisabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=int(log['topics'][1].hex(), 16) if log['topics'][1].hex() != '' else 0,
            log_index=log['logIndex'],
        )
    
    def parse_other_event(self, log: Dict[str, Any]) -> Dict[str, Any]:
        return {
            'address': self.w3.to_checksum_address(log['address']),
            'topics': [topic.hex() if isinstance(topic, bytes) else topic for topic in log.get('topics', [])],
            'data': log['data'].hex() if isinstance(log['data'], bytes) else log['data'],
            'log_index': log.get('logIndex', None),
        }
