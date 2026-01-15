from typing import Dict, Any, List, Union
from web3 import Web3
from eth_data.chain_utils.function_signatures import EVENT_TOPICS
from eth_data.tx_processor.data_models.receipt_models import *


class TransactionLogProcessor:
    def __init__(self, w3: Web3):
        self.w3 = w3

    def process_logs(self, logs: List[Dict[str, Any]]) -> Dict[str, List[Any]]:
        result = {
            'unique_addresses': set(),
            'erc20_contracts': set(),
            'erc20_transfers': [],
            'erc721_transfers': [],
            'erc1155_transfers': [],
            'erc20_approval_events': [],
            'erc721_approval_events': [],
            'ownership_transferred_events': [],
            'ownership_transfer_started_events': [],
            'access_control_role_granted_events': [],
            'access_control_role_revoked_events': [],
            'proxy_admin_changed_events': [],
            'contract_creation_events': [],
            'trading_enabled_events': [],
            'trading_disabled_events': [],
            'deposit_events': [],
            'withdraw_events': [],
            'uniswap_v2_syncs': [],
            'uniswap_v2_swaps': [],
            'uniswap_v2_mints': [],
            'uniswap_v2_burns': [],
            'uniswap_v2_pair_created_events': [],
            'uniswap_v3_pools': [],
            'uniswap_v3_initializations': [],
            'uniswap_v3_mints': [],
            'uniswap_v3_swaps': [],
            'uniswap_v3_burns': [],
            'uniswap_v3_positions': [],
            'uniswap_v3_increases': [],
            'uniswap_v3_decreases': [],
            'uniswap_v4_initializes': [],
            'uniswap_v4_modifies': [],
            'uniswap_v4_swaps': [],
            'uniswap_v4_donates': [],
            'uniswap_v4_protocol_fee_updates': [],
            'uniswap_v4_dynamic_lp_fee_updates': [],
            'uniswap_v4_protocol_fee_controller_updates': [],
            'uniswap_v4_balance_deltas': [],
            'permit2_events': [],
            'other_events': [],
        }
        
        for log in logs:
            try:
                event = self.classify_and_parse_log(log)
            except Exception as e:
                # TODO: log if you want to get the unparsed log
                continue
            if event is None:
                continue  # Skip logs we could not classify into a concrete event
            if isinstance(event, ERC20TransferEvent):
                result['erc20_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, ERC721TransferEvent):
                result['erc721_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
            elif isinstance(event, ERC1155TransferEvent):
                result['erc1155_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
            elif isinstance(event, UniswapV2SyncEvent):
                result['uniswap_v2_syncs'].append(event)
                result['unique_addresses'].add(event.pair_address)
            elif isinstance(event, UniswapV2SwapEvent):
                result['uniswap_v2_swaps'].append(event)
                result['unique_addresses'].add(event.sender)
                result['unique_addresses'].add(event.to)
                result['unique_addresses'].add(event.pair_address)
            elif isinstance(event, UniswapV2MintEvent):
                result['uniswap_v2_mints'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, UniswapV2BurnEvent):
                result['uniswap_v2_burns'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, DepositEvent):
                result['deposit_events'].append(event)
                if event.pair_address:
                    result['unique_addresses'].add(event.pair_address)
                if self.w3.is_address(event.sender):
                    result['unique_addresses'].add(event.sender)
            elif isinstance(event, WithdrawEvent):
                result['withdraw_events'].append(event)
                if event.pair_address:
                    result['unique_addresses'].add(event.pair_address)
                if event.sender:
                    result['unique_addresses'].add(event.sender)
            elif isinstance(event, UniswapV2PairCreatedEvent):
                result['uniswap_v2_pair_created_events'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.token0)
                result['unique_addresses'].add(event.token1)
            elif isinstance(event, ERC20ApprovalEvent):
                result['erc20_approval_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['unique_addresses'].add(event.owner)
                result['unique_addresses'].add(event.spender)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, ERC721ApprovalEvent):
                result['erc721_approval_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['unique_addresses'].add(event.owner)
                result['unique_addresses'].add(event.approved_address)
            elif isinstance(event, OwnershipTransferredEvent):
                result['ownership_transferred_events'].append(event)
                result['unique_addresses'].add(event.contract_address)
                result['unique_addresses'].add(event.previous_owner)
                result['unique_addresses'].add(event.new_owner)
            elif isinstance(event, OwnershipTransferStartedEvent):
                result['ownership_transfer_started_events'].append(event)
                result['unique_addresses'].add(event.contract_address)
                result['unique_addresses'].add(event.previous_owner)
                result['unique_addresses'].add(event.new_owner)
            elif isinstance(event, AccessControlRoleGrantedEvent):
                result['access_control_role_granted_events'].append(event)
                result['unique_addresses'].add(event.contract_address)
                result['unique_addresses'].add(event.account)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, AccessControlRoleRevokedEvent):
                result['access_control_role_revoked_events'].append(event)
                result['unique_addresses'].add(event.contract_address)
                result['unique_addresses'].add(event.account)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, ProxyAdminChangedEvent):
                result['proxy_admin_changed_events'].append(event)
                result['unique_addresses'].add(event.contract_address)
                result['unique_addresses'].add(event.previous_admin)
                result['unique_addresses'].add(event.new_admin)
            elif isinstance(event, TradingEnabledEvent):
                result['trading_enabled_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, TradingDisabledEvent):
                result['trading_disabled_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, UniswapV3PoolCreatedEvent):
                result['uniswap_v3_pools'].append(event)
                result['erc20_contracts'].add(event.token0)
                result['erc20_contracts'].add(event.token1)
                result['unique_addresses'].add(event.pool)
                result['unique_addresses'].add(event.token0)
                result['unique_addresses'].add(event.token1)
            elif isinstance(event, UniswapV3InitializeEvent):
                result['uniswap_v3_initializations'].append(event)
                result['unique_addresses'].add(event.pool_address)
            elif isinstance(event, UniswapV3MintEvent):
                result['uniswap_v3_mints'].append(event)
                result['unique_addresses'].add(event.pool_address)
                result['unique_addresses'].add(event.sender)
                result['unique_addresses'].add(event.owner)
            elif isinstance(event, UniswapV3PositionEvent):
                result['uniswap_v3_positions'].append(event)
                result['unique_addresses'].add(event.pool_address)
                result['unique_addresses'].add(event.owner)
            elif isinstance(event, UniswapV3IncreaseLiquidityEvent):
                result['uniswap_v3_increases'].append(event)
                result['unique_addresses'].add(event.pool_address)
            elif isinstance(event, UniswapV3DecreaseLiquidityEvent):
                result['uniswap_v3_decreases'].append(event)
                result['unique_addresses'].add(event.pool_address)
            elif isinstance(event, UniswapV3SwapEvent):
                result['uniswap_v3_swaps'].append(event)
                result['unique_addresses'].add(event.pool_address)
                result['unique_addresses'].add(event.sender)
                result['unique_addresses'].add(event.recipient)
            elif isinstance(event, Permit2Event):
                result['permit2_events'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
                result['unique_addresses'].add(event.owner)
                result['unique_addresses'].add(event.token)
                result['unique_addresses'].add(event.spender)
            elif isinstance(event, UniswapV4InitializeEvent):
                result['uniswap_v4_initializes'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
                result['unique_addresses'].add(event.currency0)
                result['unique_addresses'].add(event.currency1)
            elif isinstance(event, UniswapV4ModifyLiquidityEvent):
                result['uniswap_v4_modifies'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, UniswapV4SwapEvent):
                result['uniswap_v4_swaps'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, UniswapV4DonateEvent):
                result['uniswap_v4_donates'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, UniswapV4FeeUpdatedEvent):
                result['uniswap_v4_protocol_fee_updates'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
            elif isinstance(event, UniswapV4DynamicLPFeeUpdatedEvent):
                result['uniswap_v4_dynamic_lp_fee_updates'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
            elif isinstance(event, UniswapV4FeeControllerUpdatedEvent):
                result['uniswap_v4_protocol_fee_controller_updates'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
                result['unique_addresses'].add(event.protocol_fee_controller)
            elif isinstance(event, UniswapV4BalanceDeltaEvent):
                result['uniswap_v4_balance_deltas'].append(event)
                result['unique_addresses'].add(event.pool_manager_address)
                result['unique_addresses'].add(event.settler)
            else:
                result['other_events'].append(event)
                result['unique_addresses'].add(event.get('address'))
        return result

    def classify_and_parse_log(self, log: Dict[str, Any]) -> Any:
        if not log['topics']:
            return None
        topic = log['topics'][0].hex() if isinstance(log['topics'][0], bytes) else log['topics'][0]
        if topic.startswith('0x'):
            topic = topic[2:]
        try:
            if topic == EVENT_TOPICS['Transfer']:
                return self.parse_transfer(log)
            elif topic == EVENT_TOPICS['TransferSingle']:
                return self.parse_erc1155_single_transfer(log)
            elif topic == EVENT_TOPICS['TransferBatch']:
                return self.parse_erc1155_batch_transfer(log)
            elif topic == EVENT_TOPICS['Sync']:
                return self.parse_uniswap_v2_sync(log)
            elif topic == EVENT_TOPICS['Swap']:
                return self.parse_uniswap_v2_swap(log)
            elif topic == EVENT_TOPICS['Mint']:
                return self.parse_uniswap_v2_mint_event(log)
            elif topic == EVENT_TOPICS['Approval']:
                return self.parse_approve(log)
            elif topic == EVENT_TOPICS['Burn']:
                return self.parse_uniswap_v2_burn_event(log)
            elif topic == EVENT_TOPICS['Deposit']:
                return self.parse_deposit_event(log)
            elif topic == EVENT_TOPICS['Withdraw']:
                return self.parse_withdraw_event(log)
            elif topic == EVENT_TOPICS['PairCreated']:
                return self.parse_uniswap_v2_pair_created_event(log)
            elif topic == EVENT_TOPICS['OwnershipTransferred']:
                return self.parse_ownership_transferred_event(log)
            elif topic == EVENT_TOPICS['OwnershipTransferStarted']:
                return self.parse_ownership_transfer_started_event(log)
            elif topic == EVENT_TOPICS['RoleGranted']:
                return self.parse_role_granted_event(log)
            elif topic == EVENT_TOPICS['RoleRevoked']:
                return self.parse_role_revoked_event(log)
            elif topic == EVENT_TOPICS['AdminChanged']:
                return self.parse_admin_changed_event(log)
            elif topic == EVENT_TOPICS['TradingEnabled']:
                return self.parse_trading_enabled_event(log)
            elif topic == EVENT_TOPICS['TradingDisabled']:
                return self.parse_trading_disabled_event(log)
            # Uniswap V3 events
            elif topic == EVENT_TOPICS['PoolCreatedV3']:
                return self.parse_uniswap_v3_pool_created(log)
            elif topic == EVENT_TOPICS['InitializeV3']:
                return self.parse_uniswap_v3_initialize(log)
            elif topic == EVENT_TOPICS['MintV3']:
                return self.parse_uniswap_v3_mint(log)
            elif topic == EVENT_TOPICS['BurnV3']:
                return self.parse_uniswap_v3_burn(log)
            elif topic == EVENT_TOPICS['SwapV3']:
                return self.parse_uniswap_v3_swap(log)
            elif topic == EVENT_TOPICS['IncreaseLiquidityV3']:
                return self.parse_uniswap_v3_increase_liquidity(log)
            elif topic == EVENT_TOPICS['DecreaseLiquidityV3']:
                return self.parse_uniswap_v3_decrease_liquidity(log)
            # Uniswap V4 events
            elif topic == EVENT_TOPICS['Permit2']:
                return self.parse_permit2_event(log)
            elif topic == EVENT_TOPICS['InitializeV4']:
                return self.parse_uniswap_v4_initialize_event(log)
            elif topic == EVENT_TOPICS['ModifyLiquidityV4']:
                return self.parse_uniswap_v4_modify_liquidity_event(log)
            elif topic == EVENT_TOPICS['SwapV4']:
                return self.parse_uniswap_v4_swap_event(log)
            elif topic == EVENT_TOPICS['DonateV4']:
                return self.parse_uniswap_v4_donate_event(log)
            elif topic == EVENT_TOPICS['ProtocolFeeUpdatedV4']:
                return self.parse_uniswap_v4_fee_updated_event(log)
            elif topic == EVENT_TOPICS['DynamicLPFeeUpdatedV4']:
                return self.parse_uniswap_v4_dynamic_lp_fee_updated_event(log)
            elif topic == EVENT_TOPICS['ProtocolFeeControllerUpdatedV4']:
                return self.parse_uniswap_v4_fee_controller_updated_event(log)
            elif topic == EVENT_TOPICS['BalanceDeltaV4']:
                return self.parse_uniswap_v4_balance_delta_event(log)
            else:
                return self.parse_other_event(log)
        except Exception as e:
            raise e 

    def _ensure_hex_string(self, data) -> str:
        """Convert bytes or string data to a hex string with '0x' prefix"""
        if isinstance(data, bytes):
            data = data.hex()
        if not data.startswith('0x'):
            data = '0x' + data
        return data

    def _process_address(self, value: Union[str, bytes]) -> str:
        """Convert address to checksum format"""
        if isinstance(value, bytes):
            value = value.hex()
        address = value[-40:] if len(value) > 40 else value
        return self.w3.to_checksum_address('0x' + address if not address.startswith('0x') else address)

    def _process_integer(self, value: Union[str, bytes, int]) -> int:
        """Convert hex string or bytes to integer, handling empty cases"""
        if isinstance(value, int) or isinstance(value, float):
            return value
        if not value or value == '0x':
            return None
        if isinstance(value, bytes):
            value = value.hex()
        if isinstance(value, str):
            value = value[2:] if value.startswith('0x') else value
        return int(value, 16)

    def parse_other_event(self, log: Dict[str, Any]) -> Dict[str, Any]:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        event = {
            'address': self.w3.to_checksum_address(log['address']),
            'topics': topics,
            'data': data,
            'log_index': self._process_integer(log['logIndex']),
        }
        topic0 = topics[0] if topics else None
        if topic0 and topic0 in EVENT_TOPICS.values():
            event_type = [
                name for name, signature in EVENT_TOPICS.items() if signature == topic0
            ]
            if event_type:
                event['event_type'] = event_type[0]

        candidate_addresses = set()

        address = self.w3.to_checksum_address(log['address'])
        candidate_addresses.add(address)

        for topic in log['topics']:
            if isinstance(topic, bytes):
                topic_bytes = topic
            else:
                topic_bytes = bytes.fromhex(topic[2:] if topic.startswith('0x') else topic)
            if len(topic_bytes) == 32 and topic_bytes[:12] == b'\x00' * 12:
                candidate_address = self._process_address(topic_bytes[12:])
                candidate_addresses.add(candidate_address)

        data_bytes = bytes.fromhex(data[2:])
        for idx in range(0, len(data_bytes), 32):
            chunk = data_bytes[idx: idx + 32]
            if len(chunk) == 32 and chunk[:12] == b'\x00' * 12:
                candidate_address = self._process_address(chunk[12:])
                candidate_addresses.add(candidate_address)

        candidate_addresses.discard(address)
        if candidate_addresses:
            event['addresses'] = sorted(candidate_addresses)
        return event

    def parse_transfer(self, log: Dict[str, Any]) -> ERC20TransferEvent:
        """Parse ERC20 Transfer event log Event signature: Transfer(address indexed from, address indexed to, uint256 value)
        Topic[0]: Event signature hash
        Topic[1]: from address (indexed)
        Topic[2]: to address (indexed)
        Data: value (uint256)
        """
        # Handle both hex string and bytes data formats
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        if len(topics) == 3:
            return self.parse_erc20_transfer(log, data, topics)
        else:
            return self.parse_erc721_transfer(log, data, topics)

    def parse_erc20_transfer(self, log: Dict[str, Any], data: str, topics: List[str]) -> ERC20TransferEvent:
        return ERC20TransferEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(topics[1][-40:]),
            to_address=self.w3.to_checksum_address(topics[2][-40:]),
            amount=self._process_integer(data),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_erc721_transfer(self, log: Dict[str, Any], data: str, topics: List[str]) -> ERC721TransferEvent:
        # For ERC721, the token ID is in the 4th topic (index 3)
        token_id = self._process_integer(topics[3]) if len(topics) > 3 else None
        
        return ERC721TransferEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(topics[1][-40:]) if len(topics) > 1 else None,
            to_address=self.w3.to_checksum_address(topics[2][-40:]) if len(topics) > 2 else None,
            token_id=token_id,
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_erc1155_single_transfer(self, log: Dict[str, Any]) -> ERC1155TransferEvent:
        data = self._ensure_hex_string(log['data'])
        return ERC1155TransferEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            operator=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            from_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][3].hex()[-40:]),
            token_ids=[self._process_integer(data[2:66])],
            amounts=[self._process_integer(data[66:130])],
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_erc1155_batch_transfer(self, log: Dict[str, Any]) -> ERC1155TransferEvent:
        data = log['data'].hex()
        ids_offset = int(data[:66], 16) * 2
        amounts_offset = int(data[66:130], 16) * 2
        ids_length = int(data[ids_offset:ids_offset+64], 16) if ids_offset+64 < len(data) else 0
        amounts_length = int(data[amounts_offset:amounts_offset+64], 16) if amounts_offset+64 < len(data) else 0
        
        ids = [int(data[i:i+64], 16) for i in range(ids_offset+64, ids_offset+64+(ids_length*64), 64)] if ids_length > 0 else []
        amounts = [int(data[i:i+64], 16) for i in range(amounts_offset+64, amounts_offset+64+(amounts_length*64), 64)] if amounts_length > 0 else []
        
        return ERC1155TransferEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            operator=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            from_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][3].hex()[-40:]),
            token_ids=ids,
            amounts=amounts,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v2_sync(self, log: Dict[str, Any]) -> UniswapV2SyncEvent:
        data = self._ensure_hex_string(log['data'])
        return UniswapV2SyncEvent(
            pair_address=self.w3.to_checksum_address(log['address']),
            reserve0=self._process_integer(data[2:66]),
            reserve1=self._process_integer(data[66:130]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v2_swap(self, log: Dict[str, Any]) -> UniswapV2SwapEvent:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV2SwapEvent(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(topics[1][-40:]),
            to=self.w3.to_checksum_address(topics[2][-40:]),
            amount0In=self._process_integer(data[2:66]),
            amount1In=self._process_integer(data[66:130]),
            amount0Out=self._process_integer(data[130:194]),
            amount1Out=self._process_integer(data[194:]),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_approve(self, log: Dict[str, Any]) -> ERC20ApprovalEvent:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        if len(topics) == 3:
            return self.parse_erc20_approve(log, data, topics)
        else:
            return self.parse_erc721_approval(log, topics)

    def parse_erc20_approve(self, log: Dict[str, Any], data: str, topics: List[str]) -> ERC20ApprovalEvent:
        return ERC20ApprovalEvent(
                token_address=self.w3.to_checksum_address(log['address']),
                owner=self.w3.to_checksum_address(topics[1][-40:]),
                spender=self.w3.to_checksum_address(topics[2][-40:]),
                amount=self._process_integer(data),
                log_index=self._process_integer(log['logIndex'])
            )
    
    def parse_erc721_approval(self, log: Dict[str, Any], topics: List[str]) -> ERC721ApprovalEvent:
        return  ERC721ApprovalEvent(
                token_address=self.w3.to_checksum_address(log['address']),
                owner=self.w3.to_checksum_address(topics[1][-40:]),
                approved_address=self.w3.to_checksum_address(topics[2][-40:]),
                token_id=self._process_integer(topics[3]),
                log_index=self._process_integer(log['logIndex'])
            )
    
    def parse_uniswap_v2_mint_event(self, log: Dict[str, Any]) -> UniswapV2MintEvent:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV2MintEvent(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(topics[1][-40:]),
            amount0=self._process_integer(data[2:66]),
            amount1=self._process_integer(data[66:130]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v2_burn_event(self, log: Dict[str, Any]) -> UniswapV2BurnEvent:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV2BurnEvent(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(topics[1][-40:]),
            amount0=self._process_integer(data[2:66]),
            amount1=self._process_integer(data[66:130]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_deposit_event(self, log: Dict[str, Any]) -> DepositEvent:
        """Parse deposit event log
        
        Handles two types of deposits:
        1. Complex deposit: Deposit(uint256 id, address indexed tokenAddress, address indexed withdrawalAddress, uint256 amount, uint256 unlockTime)
        2. WETH deposit: Deposit(address indexed dst, uint256 wad)
        
        Topics[0]: Event signature hash
        For complex deposits:
            Topics[1]: tokenAddress (indexed)
            Topics[2]: withdrawalAddress (indexed)
            Data: id, amount, unlockTime
        For WETH deposits:
            Topics[1]: dst address (indexed)
            Data: wad (amount)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Check if this is a complex deposit (with id, token, withdrawal)
        if len(topics) > 2:  # Complex deposit has more topics
            id = self._process_integer(data[2:66])
            amount = self._process_integer(data[66:130])
            unlock_time = self._process_integer(data[130:])
            
            return DepositEvent(
                id=id,
                token_address=self.w3.to_checksum_address(topics[1][-40:]),
                withdrawal_address=self.w3.to_checksum_address(topics[2][-40:]),
                amount=amount,
                unlock_time=unlock_time,
                log_index=self._process_integer(log['logIndex'])
            )
        else:  # Simple deposit (like WETH)
            # For WETH deposit, data contains only the amount (wad)
            sender = self.w3.to_checksum_address(topics[1][-40:]) if len(topics) > 1 else None
            return DepositEvent(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=sender,
                amount= self._process_integer(data),
                log_index=self._process_integer(log['logIndex'])
            )
    
    def parse_withdraw_event(self, log: Dict[str, Any]) -> WithdrawEvent:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        if len(topics) > 1:
            return WithdrawEvent(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=self.w3.to_checksum_address(topics[1][-40:]),
                amount=self._process_integer(data),
                log_index=self._process_integer(log['logIndex'])
            )
        else:
            return WithdrawEvent(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=None,
                amount=self._process_integer(data),
                log_index=self._process_integer(log['logIndex'])
            )
    
    def parse_uniswap_v2_pair_created_event(self, log: Dict[str, Any]) -> UniswapV2PairCreatedEvent:
        """Parse Uniswap pair creation event
        
        PairCreated(address token0, address token1, address pair, uint)
        Topic[0]: Event signature
        Topic[1]: token0 address (indexed)
        Topic[2]: token1 address (indexed)
        Data: [pair address (32 bytes), uint256]
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV2PairCreatedEvent(
            pair_address=self.w3.to_checksum_address(data[26:66]),
            token0=self.w3.to_checksum_address(topics[1][-40:]),
            token1=self.w3.to_checksum_address(topics[2][-40:]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_ownership_transferred_event(self, log: Dict[str, Any]) -> OwnershipTransferredEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return OwnershipTransferredEvent(
            contract_address=self.w3.to_checksum_address(log['address']),
            previous_owner=self.w3.to_checksum_address(topics[1][-40:]),
            new_owner=self.w3.to_checksum_address(topics[2][-40:]),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_ownership_transfer_started_event(self, log: Dict[str, Any]) -> OwnershipTransferStartedEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return OwnershipTransferStartedEvent(
            contract_address=self.w3.to_checksum_address(log['address']),
            previous_owner=self.w3.to_checksum_address(topics[1][-40:]),
            new_owner=self.w3.to_checksum_address(topics[2][-40:]),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_role_granted_event(self, log: Dict[str, Any]) -> AccessControlRoleGrantedEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return AccessControlRoleGrantedEvent(
            contract_address=self.w3.to_checksum_address(log['address']),
            role=topics[1],
            account=self.w3.to_checksum_address(topics[2][-40:]),
            sender=self.w3.to_checksum_address(topics[3][-40:]),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_role_revoked_event(self, log: Dict[str, Any]) -> AccessControlRoleRevokedEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return AccessControlRoleRevokedEvent(
            contract_address=self.w3.to_checksum_address(log['address']),
            role=topics[1],
            account=self.w3.to_checksum_address(topics[2][-40:]),
            sender=self.w3.to_checksum_address(topics[3][-40:]),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_admin_changed_event(self, log: Dict[str, Any]) -> ProxyAdminChangedEvent:
        data = self._ensure_hex_string(log['data'])[2:]
        data = data.zfill(128)
        previous_chunk = data[:64]
        new_chunk = data[64:128]
        previous_admin = self.w3.to_checksum_address('0x' + previous_chunk[-40:])
        new_admin = self.w3.to_checksum_address('0x' + new_chunk[-40:])
        return ProxyAdminChangedEvent(
            contract_address=self.w3.to_checksum_address(log['address']),
            previous_admin=previous_admin,
            new_admin=new_admin,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_trading_enabled_event(self, log: Dict[str, Any]) -> TradingEnabledEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return TradingEnabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=self._process_integer(topics[1]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_trading_disabled_event(self, log: Dict[str, Any]) -> TradingDisabledEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return TradingDisabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=self._process_integer(topics[1]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    # --------------------------------------------------------------------------
    # Uniswap V3 Pool Events 
    # --------------------------------------------------------------------------

    def parse_uniswap_v3_pool_created(self, log: Dict[str, Any]) -> UniswapV3PoolCreatedEvent:
        """Parse Uniswap V3 pool creation event
        Event: PoolCreated(token0, token1, fee, tickSpacing, pool)
        
        Topics[0]: Event signature
        Topics[1]: token0 address (indexed)
        Topics[2]: token1 address (indexed)
        Topics[3]: fee (indexed)
        Data: tickSpacing (int24), pool (address)
        """
        data = self._ensure_hex_string(log['data'])
        # Handle both hex string and bytes formats for topics
        token0 = self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if isinstance(log['topics'][1], bytes) else self.w3.to_checksum_address(log['topics'][1][-40:])
        token1 = self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if isinstance(log['topics'][2], bytes) else self.w3.to_checksum_address(log['topics'][2][-40:])
        fee = int(log['topics'][3].hex(), 16) if isinstance(log['topics'][3], bytes) else int(log['topics'][3], 16)    
        
        tick_spacing = int(data[2:66], 16)  # First 32 bytes
        pool = self.w3.to_checksum_address('0x' + data[90:130])  # Last 40 bytes of second 32-byte chunk
        
        return UniswapV3PoolCreatedEvent(
            token0=token0,
            token1=token1,
            fee=fee,
            tick_spacing=tick_spacing,
            pool=pool,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_initialize(self, log: Dict[str, Any]) -> UniswapV3InitializeEvent:
        """Parse Uniswap V3 pool initialization
        Event: Initialize(uint160 sqrtPriceX96, int24 tick)
        
        Topics[0]: Event signature
        Data: sqrtPriceX96 (uint160), tick (int24)
        """
        data = self._ensure_hex_string(log['data'])
        return UniswapV3InitializeEvent(
            pool_address=self.w3.to_checksum_address(log['address']),
            sqrt_price_x96=self._process_integer(data[2:66]),
            tick=int.from_bytes(bytes.fromhex(data[66:130]), byteorder='big', signed=True),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_mint(self, log: Dict[str, Any]) -> UniswapV3MintEvent:
        """Parse Uniswap V3 mint event
        Event: Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: owner address (indexed)
        Topics[2]: tickLower (indexed, int24)
        Topics[3]: tickUpper (indexed, int24)
        Data: 
            - sender (address, 32 bytes with padding)
            - amount (uint128, 32 bytes)
            - amount0 (uint256, 32 bytes)
            - amount1 (uint256, 32 bytes)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]        
        # Parse ticks as signed integers
        tick_lower = int.from_bytes(bytes.fromhex(topics[2][2:]), byteorder='big', signed=True)
        tick_upper = int.from_bytes(bytes.fromhex(topics[3][2:]), byteorder='big', signed=True)
        
        return UniswapV3MintEvent(
            pool_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(data[26:66]),
            owner=self.w3.to_checksum_address(topics[1].hex()[-40:]) if isinstance(topics[1], bytes) else self.w3.to_checksum_address(topics[1][-40:]),
            tick_lower=tick_lower,
            tick_upper=tick_upper,
            amount=self._process_integer(data[66:130]),    # Second 32 bytes
            amount0=self._process_integer(data[130:194]),  # Third 32 bytes
            amount1=self._process_integer(data[194:258]),  # Fourth 32 bytes
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_swap(self, log: Dict[str, Any]) -> UniswapV3SwapEvent:
        """Parse Uniswap V3 swap event
        Event: Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
        
        Topics[0]: Event signature
        Topics[1]: sender address (indexed)
        Topics[2]: recipient address (indexed)
        Data: amount0 (int256), amount1 (int256), sqrtPriceX96 (uint160), liquidity (uint128), tick (int24)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
         # Convert hex data (skipping "0x") into bytes
        data_bytes = bytes.fromhex(data[2:])
        
        # Parse each field from the data bytes
        amount0 = int.from_bytes(data_bytes[0:32], byteorder='big', signed=True)
        amount1 = int.from_bytes(data_bytes[32:64], byteorder='big', signed=True)
        sqrt_price_x96 = int.from_bytes(data_bytes[64:96], byteorder='big', signed=False)
        liquidity = int.from_bytes(data_bytes[96:128], byteorder='big', signed=False)
        tick = int.from_bytes(data_bytes[128:160], byteorder='big', signed=True)
        
        return UniswapV3SwapEvent(
            pool_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(topics[1][-40:]),
            recipient=self.w3.to_checksum_address(topics[2][-40:]),
            amount0=amount0,
            amount1=amount1,
            sqrt_price_x96=sqrt_price_x96,
            liquidity=liquidity,
            tick=tick,
            log_index=self._process_integer(log['logIndex'])
        )
        
    def parse_uniswap_v3_position(self, log: Dict[str, Any]) -> UniswapV3PositionEvent:
        """Parse Uniswap V3 position event (IncreaseLiquidity)
        Event: IncreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: tokenId (indexed)
        Topics[2]: owner address (optional, if provided)
        Topics[3]: tick_lower (signed, optional)
        Topics[4]: tick_upper (signed, optional)
        Data: liquidity (uint128), amount0 (uint256), amount1 (uint256)
        """
        data = self._ensure_hex_string(log['data'])
        # For signed tick values, convert using two's-complement conversion.
        tick_lower = 0
        tick_upper = 0
        if len(log['topics']) > 3:
            t = self._ensure_hex_string(log['topics'][3])
            # Remove "0x" and convert to bytes:
            tick_lower = int.from_bytes(bytes.fromhex(t[2:]), byteorder='big', signed=True)
        if len(log['topics']) > 4:
            t = self._ensure_hex_string(log['topics'][4])
            tick_upper = int.from_bytes(bytes.fromhex(t[2:]), byteorder='big', signed=True)
        
        owner = None
        if len(log['topics']) > 2:
            # Assume topics[2] is already a hex string; take the last 40 characters.
            t = self._ensure_hex_string(log['topics'][2])
            owner = self.w3.to_checksum_address(t[-40:])
        
        return UniswapV3PositionEvent(
            token_id=int(self._ensure_hex_string(log['topics'][1]).lstrip("0x"), 16),
            liquidity=int(data[2:66], 16),      # First 32 bytes
            amount0=int(data[66:130], 16),       # Next 32 bytes
            amount1=int(data[130:194], 16),      # Last 32 bytes
            pool_address=self.w3.to_checksum_address(log['address']),
            owner=owner,
            tick_lower=tick_lower,
            tick_upper=tick_upper,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_increase_liquidity(self, log: Dict[str, Any]) -> UniswapV3IncreaseLiquidityEvent:
        """Parse Uniswap V3 increase liquidity event
        Event: IncreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: tokenId (indexed)
        Data: liquidity (uint128), amount0 (uint256), amount1 (uint256)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV3IncreaseLiquidityEvent(
            token_id=int(topics[1].lstrip("0x"), 16),
            liquidity=int(data[2:66], 16) if data[2:66] else 0,
            amount0=int(data[66:130], 16) if data[66:130] else 0,
            amount1=int(data[130:194], 16) if data[130:194] else 0,
            pool_address=self.w3.to_checksum_address(log['address']),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_uniswap_v3_decrease_liquidity(self, log: Dict[str, Any]) -> UniswapV3DecreaseLiquidityEvent:
        """Parse Uniswap V3 decrease liquidity event
        Event: DecreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: tokenId (indexed)
        Data: liquidity (uint128), amount0 (uint256), amount1 (uint256)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV3DecreaseLiquidityEvent(
            token_id=int(topics[1].lstrip("0x"), 16),
            liquidity=int(data[2:66], 16) if data[2:66] else 0,
            amount0=int(data[66:130], 16) if data[66:130] else 0,
            amount1=int(data[130:194], 16) if data[130:194] else 0,
            pool_address=self.w3.to_checksum_address(log['address']),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_uniswap_v3_burn(self, log: Dict[str, Any]) -> UniswapV3BurnEvent:
        """Parse Uniswap V3 burn event
        Event: Burn(address indexed sender, address indexed owner, uint256 amount0, uint256 amount1)

        Topics[0]: Event signature
        Topics[1]: sender address (indexed) - used here as token_id
        Topics[2]: owner address (indexed)
        Data: amount0 (uint256), amount1 (uint256)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV3BurnEvent(
            token_id=int(topics[1].lstrip("0x"), 16),
            liquidity=int(data[2:66], 16) if data[2:66] else 0,
            amount0=int(data[66:130], 16) if data[66:130] else 0,
            amount1=int(data[130:194], 16) if data[130:194] else 0,
            pool_address=self.w3.to_checksum_address(log['address']),
            log_index=self._process_integer(log['logIndex'])
        )
    
    # --------------------------------------------------------------------------
    # Uniswap V4 Pool Events 
    # --------------------------------------------------------------------------

    def parse_uniswap_v4_initialize_event(self, log: Dict[str, Any]) -> UniswapV4InitializeEvent:
        """
        Parse Uniswap V4 pool initialization event.
        Event Format:
          Initialize(bytes32 event_id, address indexed currency0, address indexed currency1,
                     uint24 fee, int24 tickSpacing, address hooks, uint160 sqrtPriceX96, int24 tick)

        Expected log layout:
          - log['address'] is the pool manager (emitter)
          - topics[1] is the event id (bytes32)
          - topics[2] is the first currency address (indexed)
          - topics[3] is the second currency address (indexed)
          - log['data'] is a concatenation of five 32-byte words representing:
                fee, tickSpacing, hooks, sqrtPriceX96, tick
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]

        # Extract indexed parameters from topics
        event_id = topics[1] if isinstance(topics[1], str) else topics[1].hex()
        currency0 = self.w3.to_checksum_address(topics[2][-40:])
        currency1 = self.w3.to_checksum_address(topics[3][-40:])

        # Each parameter occupies 64 hex characters (32 bytes)
        fee = self._process_integer('0x' + data[2:66])
        tick_spacing = int.from_bytes(bytes.fromhex(data[66:130]), byteorder='big', signed=True)
        # Address is in the last 40 hex characters of the 32-byte word
        hooks = self.w3.to_checksum_address('0x' + data[130+24:194])
        sqrt_price_x96 = self._process_integer('0x' + data[194:258])
        tick = int.from_bytes(bytes.fromhex(data[258:322]), byteorder='big', signed=True)
        return UniswapV4InitializeEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            event_id=event_id,
            currency0=currency0,
            currency1=currency1,
            fee=fee,
            tick_spacing=tick_spacing,
            hooks=hooks,
            sqrt_price_x96=sqrt_price_x96,
            tick=tick,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v4_modify_liquidity_event(self, log: Dict[str, Any]) -> UniswapV4ModifyLiquidityEvent:
        """
        Parse a Uniswap V4 ModifyLiquidity event.
        
        Event Format:
          ModifyLiquidity(bytes32 indexed id, address indexed sender, int24 tickLower, int24 tickUpper, int256 liquidityDelta, bytes32 salt)
        
        Topics:
          [0]: Event signature
          [1]: id (bytes32)
          [2]: sender (address)
        
        Data (concatenated 32-byte words):
          - First 32 bytes: tickLower (int24, padded to 32-bytes as signed)
          - Next 32 bytes: tickUpper (int24, padded to 32-bytes as signed)
          - Next 32 bytes: liquidityDelta (int256, padded as signed)
          - Next 32 bytes: salt (bytes32)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Process indexed values.
        event_id = topics[1]
        sender = self.w3.to_checksum_address(topics[2][-40:])
        
        # Parse data fields - ensure both tick values are parsed as signed integers
        tick_lower = int.from_bytes(bytes.fromhex(data[2:66]), byteorder='big', signed=True)
        tick_upper = int.from_bytes(bytes.fromhex(data[66:130]), byteorder='big', signed=True)
        liquidity_delta = int.from_bytes(bytes.fromhex(data[130:194]), byteorder='big', signed=True)
        salt_hex = data[194:258]
        
        return UniswapV4ModifyLiquidityEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            event_id=event_id,
            sender=sender,
            tick_lower=tick_lower,
            tick_upper=tick_upper,
            liquidity_delta=liquidity_delta,
            salt=salt_hex,
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_permit2_event(self, log: Dict[str, Any]) -> Permit2Event:
        """
        Parse the Permit (Permit2Event) event emitted by the Permit2Event contract.
        
        Event Format:
          Permit(address indexed owner, address indexed token, address indexed spender,
                 uint160 amount, uint48 expiration, uint48 nonce)
        
        Topics:
          [0]: Event signature (0xc6a377bfc4eb120024a8ac08eef205be16b817020812c73223e81d1bdb9708ec)
          [1]: owner
          [2]: token
          [3]: spender
        
        Data (padded to 32-byte words):
          - First 32 bytes: amount (uint160)
          - Next 32 bytes: expiration (uint48)
          - Next 32 bytes: nonce (uint48)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        owner = self.w3.to_checksum_address(topics[1][-40:])
        token = self.w3.to_checksum_address(topics[2][-40:])
        spender = self.w3.to_checksum_address(topics[3][-40:])
        
        # Each value occupies a 32-byte word. Slice off the "0x" and process.
        amount = self._process_integer('0x' + data[2:66])
        expiration = self._process_integer('0x' + data[66:130])
        nonce = self._process_integer('0x' + data[130:194])
        
        return Permit2Event(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            owner=owner,
            token=token,
            spender=spender,
            amount=amount,
            expiration=expiration,
            nonce=nonce,
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_uniswap_v4_swap_event(self, log: Dict[str, Any]) -> UniswapV4SwapEvent:
        """
        Parse Uniswap V4 Swap event
        Event: Swap(bytes32 indexed id, address indexed sender, int128 amount0, int128 amount1,
                    uint160 sqrtPriceX96, uint128 liquidity, int24 tick, uint24 fee)
        
        Topics:
          [0]: Event signature
          [1]: id (bytes32)
          [2]: sender (address)
        
        Data:
          - amount0 (int128 as 32-byte word)
          - amount1 (int128 as 32-byte word)
          - sqrtPriceX96 (uint160 as 32-byte word)
          - liquidity (uint128 as 32-byte word)
          - tick (int24 as 32-byte word)
          - fee (uint24 as 32-byte word)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Extract indexed parameters
        event_id = topics[1]
        sender = self.w3.to_checksum_address(topics[2][-40:])
        
        # Parse data parameters (each 32-byte word)
        amount0 = int.from_bytes(bytes.fromhex(data[2:66]), byteorder='big', signed=True)
        amount1 = int.from_bytes(bytes.fromhex(data[66:130]), byteorder='big', signed=True)
        sqrt_price_x96 = int.from_bytes(bytes.fromhex(data[130:194]), byteorder='big', signed=False)
        liquidity = int.from_bytes(bytes.fromhex(data[194:258]), byteorder='big', signed=False)
        tick = int.from_bytes(bytes.fromhex(data[258:322]), byteorder='big', signed=True)
        fee = int.from_bytes(bytes.fromhex(data[322:386]), byteorder='big', signed=False)
        
        return UniswapV4SwapEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            event_id=event_id,
            sender=sender,
            amount0=amount0,
            amount1=amount1,
            sqrt_price_x96=sqrt_price_x96,
            liquidity=liquidity,
            tick=tick,
            fee=fee,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v4_donate_event(self, log: Dict[str, Any]) -> UniswapV4DonateEvent:
        """
        Parse Uniswap V4 Donate event
        Event: Donate(bytes32 indexed id, address indexed sender, int256 amount0, int256 amount1)
        
        Topics:
          [0]: Event signature
          [1]: id (bytes32)
          [2]: sender (address)
        
        Data:
          - amount0 (int256 as 32-byte word)
          - amount1 (int256 as 32-byte word)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Extract indexed parameters
        event_id = topics[1]
        sender = self.w3.to_checksum_address(topics[2][-40:])
        
        # Parse data parameters (each 32-byte word)
        amount0 = int.from_bytes(bytes.fromhex(data[2:66]), byteorder='big', signed=True)
        amount1 = int.from_bytes(bytes.fromhex(data[66:130]), byteorder='big', signed=True)
        
        return UniswapV4DonateEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            event_id=event_id,
            sender=sender,
            amount0=amount0,
            amount1=amount1,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v4_fee_updated_event(self, log: Dict[str, Any]) -> UniswapV4FeeUpdatedEvent:
        """
        Parse Uniswap V4 ProtocolFeeUpdated event
        Event: ProtocolFeeUpdated(bytes32 indexed id, uint24 protocolFee)
        
        Topics:
          [0]: Event signature
          [1]: id (bytes32)
        
        Data:
          - protocolFee (uint24 as 32-byte word)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Extract indexed parameters
        event_id = topics[1]
        
        # Parse data parameter
        protocol_fee = int.from_bytes(bytes.fromhex(data[2:66]), byteorder='big', signed=False)
        
        return UniswapV4FeeUpdatedEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            event_id=event_id,
            protocol_fee=protocol_fee,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v4_dynamic_lp_fee_updated_event(self, log: Dict[str, Any]) -> UniswapV4DynamicLPFeeUpdatedEvent:
        """
        Parse Uniswap V4 DynamicLPFeeUpdated event
        Event: DynamicLPFeeUpdated(bytes32 indexed id, uint24 dynamicLPFee)
        
        Topics:
          [0]: Event signature
          [1]: id (bytes32)
        
        Data:
          - dynamicLPFee (uint24 as 32-byte word)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Extract indexed parameters
        event_id = topics[1]
        
        # Parse data parameter
        dynamic_lp_fee = int.from_bytes(bytes.fromhex(data[2:66]), byteorder='big', signed=False)
        
        return UniswapV4DynamicLPFeeUpdatedEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            event_id=event_id,
            dynamic_lp_fee=dynamic_lp_fee,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v4_fee_controller_updated_event(self, log: Dict[str, Any]) -> UniswapV4FeeControllerUpdatedEvent:
        """
        Parse Uniswap V4 ProtocolFeeControllerUpdated event
        Event: ProtocolFeeControllerUpdated(address indexed protocolFeeController)
        
        Topics:
          [0]: Event signature
          [1]: protocolFeeController (address)
        
        Data: None
        """
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Extract indexed parameter
        protocol_fee_controller = self.w3.to_checksum_address(topics[1][-40:])
        
        return UniswapV4FeeControllerUpdatedEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            protocol_fee_controller=protocol_fee_controller,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v4_balance_delta_event(self, log: Dict[str, Any]) -> UniswapV4BalanceDeltaEvent:
        """
        Parse Uniswap V4 BalanceDelta event
        Event: BalanceDelta(bytes32 indexed poolId, address indexed settler, int256 delta0, int256 delta1)
        
        Topics:
          [0]: Event signature
          [1]: poolId (bytes32)
          [2]: settler (address)
        
        Data:
          - delta0 (int256 as 32-byte word)
          - delta1 (int256 as 32-byte word)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Extract indexed parameters
        pool_id = topics[1]
        settler = self.w3.to_checksum_address(topics[2][-40:])
        
        # Parse data parameters (each 32-byte word)
        delta0 = int.from_bytes(bytes.fromhex(data[2:66]), byteorder='big', signed=True)
        delta1 = int.from_bytes(bytes.fromhex(data[66:130]), byteorder='big', signed=True)
        
        return UniswapV4BalanceDeltaEvent(
            pool_manager_address=self.w3.to_checksum_address(log['address']),
            pool_id=pool_id,
            settler=settler,
            delta0=delta0,
            delta1=delta1,
            log_index=self._process_integer(log['logIndex'])
        )
    
