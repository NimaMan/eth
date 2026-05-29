/// Core transaction processor.
///
/// Converts raw transaction data and simulation results into complete
/// `ProcessedTransaction` objects with decoded events, internal calls, balance
/// changes, fee fields, and classification.
///
/// This processor:
/// 1. Takes simulation results (logs, traces, balance changes)
/// 2. Decodes event logs using LogDecoder
/// 3. Processes internal transactions from traces
/// 4. Converts balance changes to proper format
/// 5. Creates complete ProcessedTransaction object
use super::data_models::{
    tx_models::ETHTransfer, ContractCreationEvent, InternalTransaction, ProcessedAccessListItem,
    ProcessedTransaction, TradingDisabledEvent, TradingEnabledEvent, TransactionFees,
};
use super::{
    address_index::{
        build_unknown_event_record, extract_candidate_addresses_from_log, populate_unique_addresses,
    },
    AddressBalanceChangeCalculator, DecodedEvent, LogDecoder, TransactionClassifier,
    TransactionTraceProcessor, TransactionType,
};
use alloy_eips::eip7702::SignedAuthorization;
use alloy_primitives::{Address, B256, U256};
use eyre::{eyre, Result};
use reth_chain_query::function_signatures::FUNCTION_SIGNATURES;
use std::collections::{HashMap, HashSet};

pub struct TxProcessor {
    pub decoder: LogDecoder,
    pub classifier: TransactionClassifier,
}

impl TxProcessor {
    /// Create new transaction processor
    pub fn new() -> Self {
        Self {
            decoder: LogDecoder::new(),
            classifier: TransactionClassifier::new(),
        }
    }

    /// Process transaction from raw data into ProcessedTransaction
    ///
    /// It takes raw transaction data and converts it into a complete
    /// `ProcessedTransaction` with decoded events.
    pub async fn process_transaction_from_raw_data(
        &self,
        tx_hash: B256,
        block_number: u64,
        block_timestamp: u64,
        tx_index: u64,
        from: Address,
        to: Option<Address>,
        value: U256,
        input: Vec<u8>,
        gas_price: U256,
        gas_used: u64,
        status: bool,
        nonce: u64,
        raw_tx_type: u8,
        max_fee_per_gas: Option<U256>,
        max_priority_fee_per_gas: Option<U256>,
        logs: Vec<alloy_primitives::Log>,
        gas_limit: u64,
        access_list: Vec<super::data_models::ProcessedAccessListItem>,
        blob_versioned_hashes: Vec<B256>,
        max_fee_per_blob_gas: Option<U256>,
        blob_gas_used: Option<u64>,
        signed_authorizations: Vec<SignedAuthorization>,
        address_balance_changes: Option<HashMap<Address, super::data_models::AddressBalanceChange>>,
    ) -> Result<ProcessedTransaction> {
        // STEP 1: Decode event logs using LogDecoder.
        let mut erc20_transfers = Vec::new();
        let mut erc721_transfers = Vec::new();
        let mut erc1155_transfers = Vec::new();
        let mut erc20_approval_events = Vec::new();
        let mut erc721_approval_events = Vec::new();
        let mut approval_for_all_events = Vec::new();
        let mut uniswap_v2_syncs = Vec::new();
        let mut uniswap_v2_swaps = Vec::new();
        let mut uniswap_v3_pools = Vec::new();
        let mut uniswap_v3_initializations = Vec::new();
        let mut uniswap_v3_swaps = Vec::new();
        let mut uniswap_v3_mints = Vec::new();
        let mut uniswap_v3_burns = Vec::new();
        let mut uniswap_v3_positions = Vec::new();
        let mut uniswap_v3_increases = Vec::new();
        let mut uniswap_v3_decreases = Vec::new();
        let mut uniswap_v2_mints = Vec::new();
        let mut uniswap_v2_burns = Vec::new();
        let mut uniswap_v4_initializes = Vec::new();
        let mut uniswap_v4_modifies = Vec::new();
        let mut uniswap_v4_swaps = Vec::new();
        let mut uniswap_v4_donates = Vec::new();
        let mut uniswap_v4_protocol_fee_updates = Vec::new();
        let mut uniswap_v4_dynamic_lp_fee_updates = Vec::new();
        let mut uniswap_v4_protocol_fee_controller_updates = Vec::new();
        let mut uniswap_v4_balance_deltas = Vec::new();
        let mut permit2_events = Vec::new();
        let mut deposit_events = Vec::new();
        let mut withdraw_events = Vec::new();
        let mut uniswap_v2_pair_created_events = Vec::new();
        let mut ownership_transferred_events = Vec::new();
        let mut ownership_transfer_started_events = Vec::new();
        let mut access_control_role_granted_events = Vec::new();
        let mut access_control_role_revoked_events = Vec::new();
        let mut proxy_admin_changed_events = Vec::new();
        let mut trading_enabled_events = Vec::new();
        let mut trading_disabled_events = Vec::new();
        let mut other_events = Vec::new();

        for (log_index, log) in logs.iter().enumerate() {
            let candidate_addresses = extract_candidate_addresses_from_log(log);
            match self.decoder.decode_log(log, log_index as u64) {
                Ok(Some(decoded_event)) => match decoded_event {
                    DecodedEvent::ERC20TransferEvent(event) => erc20_transfers.push(event),
                    DecodedEvent::ERC721TransferEvent(event) => erc721_transfers.push(event),
                    DecodedEvent::ERC1155TransferEvent(event) => erc1155_transfers.push(event),
                    DecodedEvent::ERC20ApprovalEvent(event) => erc20_approval_events.push(event),
                    DecodedEvent::ERC721ApprovalEvent(event) => erc721_approval_events.push(event),
                    DecodedEvent::ApprovalForAllEvent(event) => approval_for_all_events.push(event),
                    DecodedEvent::UniswapV2SyncEvent(event) => uniswap_v2_syncs.push(event),
                    DecodedEvent::UniswapV2SwapEvent(event) => uniswap_v2_swaps.push(event),
                    DecodedEvent::UniswapV3PoolCreatedEvent(event) => uniswap_v3_pools.push(event),
                    DecodedEvent::UniswapV3InitializeEvent(event) => {
                        uniswap_v3_initializations.push(event)
                    }
                    DecodedEvent::UniswapV3SwapEvent(event) => uniswap_v3_swaps.push(event),
                    DecodedEvent::UniswapV3MintEvent(event) => uniswap_v3_mints.push(event),
                    DecodedEvent::UniswapV3BurnEvent(event) => uniswap_v3_burns.push(event),
                    DecodedEvent::UniswapV3PositionEvent(event) => uniswap_v3_positions.push(event),
                    DecodedEvent::UniswapV3IncreaseLiquidityEvent(event) => {
                        uniswap_v3_increases.push(event)
                    }
                    DecodedEvent::UniswapV3DecreaseLiquidityEvent(event) => {
                        uniswap_v3_decreases.push(event)
                    }
                    DecodedEvent::UniswapV2MintEvent(event) => uniswap_v2_mints.push(event),
                    DecodedEvent::UniswapV2BurnEvent(event) => uniswap_v2_burns.push(event),
                    DecodedEvent::DepositEvent(event) => deposit_events.push(event),
                    DecodedEvent::WithdrawEvent(event) => withdraw_events.push(event),
                    DecodedEvent::UniswapV2PairCreatedEvent(event) => {
                        uniswap_v2_pair_created_events.push(event)
                    }
                    DecodedEvent::OwnershipTransferredEvent(event) => {
                        ownership_transferred_events.push(event)
                    }
                    DecodedEvent::OwnershipTransferStartedEvent(event) => {
                        ownership_transfer_started_events.push(event)
                    }
                    DecodedEvent::AccessControlRoleGrantedEvent(event) => {
                        access_control_role_granted_events.push(event)
                    }
                    DecodedEvent::AccessControlRoleRevokedEvent(event) => {
                        access_control_role_revoked_events.push(event)
                    }
                    DecodedEvent::ProxyAdminChangedEvent(event) => {
                        proxy_admin_changed_events.push(event)
                    }
                    DecodedEvent::TradingEnabledEvent(event) => trading_enabled_events.push(event),
                    DecodedEvent::TradingDisabledEvent(event) => {
                        trading_disabled_events.push(event)
                    }
                    DecodedEvent::UniswapV4InitializeEvent(event) => {
                        uniswap_v4_initializes.push(event)
                    }
                    DecodedEvent::UniswapV4ModifyLiquidityEvent(event) => {
                        uniswap_v4_modifies.push(event)
                    }
                    DecodedEvent::UniswapV4SwapEvent(event) => uniswap_v4_swaps.push(event),
                    DecodedEvent::UniswapV4DonateEvent(event) => uniswap_v4_donates.push(event),
                    DecodedEvent::UniswapV4FeeUpdatedEvent(event) => {
                        uniswap_v4_protocol_fee_updates.push(event)
                    }
                    DecodedEvent::UniswapV4DynamicLPFeeUpdatedEvent(event) => {
                        uniswap_v4_dynamic_lp_fee_updates.push(event)
                    }
                    DecodedEvent::UniswapV4FeeControllerUpdatedEvent(event) => {
                        uniswap_v4_protocol_fee_controller_updates.push(event)
                    }
                    DecodedEvent::UniswapV4BalanceDeltaEvent(event) => {
                        uniswap_v4_balance_deltas.push(event)
                    }
                    DecodedEvent::Permit2Event(event) => permit2_events.push(event),
                    DecodedEvent::UniswapV3CollectEvent(_) => {
                        other_events.push(build_unknown_event_record(
                            log,
                            log_index as u64,
                            Some("UniswapV3CollectEvent"),
                            &candidate_addresses,
                        ));
                    }
                },
                Ok(None) => {
                    other_events.push(build_unknown_event_record(
                        log,
                        log_index as u64,
                        None,
                        &candidate_addresses,
                    ));
                }
                Err(_) => {
                    other_events.push(build_unknown_event_record(
                        log,
                        log_index as u64,
                        Some("UNDECODED"),
                        &candidate_addresses,
                    ));
                }
            }
        }

        // STEP 2: Create transaction fees
        let protocol_type = match raw_tx_type {
            0 => "legacy",
            1 => "eip2930",
            2 => "eip1559",
            3 => "eip4844",
            4 => "eip7702",
            _ => "unknown",
        }
        .to_string();
        let fees = TransactionFees {
            gas_price,
            gas_used,
            gas_limit,
            tx_fee: gas_price * U256::from(gas_used),
            protocol_type,
            max_fee_per_gas: max_fee_per_gas.clone(),
            max_priority_fee: max_priority_fee_per_gas.clone(),
            max_fee_per_blob_gas: max_fee_per_blob_gas.clone(),
            blob_gas_used,
        };

        // Determine simple ETH transfer events before building the struct.
        let empty_internals: &[InternalTransaction] = &[];
        let eth_transfers = self.extract_eth_transfers(from, to, value, &input, empty_internals);

        // STEP 3: Create ProcessedTransaction with all extracted data
        let mut processed_tx = ProcessedTransaction::new(
            tx_hash,
            block_number,
            block_timestamp,
            tx_index,
            from,
            to,
            value,
            status,
            nonce,
            raw_tx_type,
            input,
        );

        // STEP 4: Set all the decoded data
        processed_tx.fees = fees;
        processed_tx.erc20_transfers = erc20_transfers;
        processed_tx.erc721_transfers = erc721_transfers;
        processed_tx.erc1155_transfers = erc1155_transfers;
        processed_tx.erc20_approval_events = erc20_approval_events;
        processed_tx.erc721_approval_events = erc721_approval_events;
        processed_tx.approval_for_all_events = approval_for_all_events;
        processed_tx.uniswap_v2_syncs = uniswap_v2_syncs;
        processed_tx.uniswap_v2_swaps = uniswap_v2_swaps;
        processed_tx.uniswap_v3_pools = uniswap_v3_pools;
        processed_tx.uniswap_v3_initializations = uniswap_v3_initializations;
        processed_tx.uniswap_v3_swaps = uniswap_v3_swaps;
        processed_tx.uniswap_v3_mints = uniswap_v3_mints;
        processed_tx.uniswap_v3_burns = uniswap_v3_burns;
        processed_tx.uniswap_v3_positions = uniswap_v3_positions;
        processed_tx.uniswap_v3_increases = uniswap_v3_increases;
        processed_tx.uniswap_v3_decreases = uniswap_v3_decreases;
        processed_tx.uniswap_v2_mints = uniswap_v2_mints;
        processed_tx.uniswap_v2_burns = uniswap_v2_burns;
        processed_tx.uniswap_v4_initializes = uniswap_v4_initializes;
        processed_tx.uniswap_v4_modifies = uniswap_v4_modifies;
        processed_tx.uniswap_v4_swaps = uniswap_v4_swaps;
        processed_tx.uniswap_v4_donates = uniswap_v4_donates;
        processed_tx.uniswap_v4_protocol_fee_updates = uniswap_v4_protocol_fee_updates;
        processed_tx.uniswap_v4_dynamic_lp_fee_updates = uniswap_v4_dynamic_lp_fee_updates;
        processed_tx.uniswap_v4_protocol_fee_controller_updates =
            uniswap_v4_protocol_fee_controller_updates;
        processed_tx.uniswap_v4_balance_deltas = uniswap_v4_balance_deltas;
        processed_tx.permit2_events = permit2_events;
        processed_tx.deposit_events = deposit_events;
        processed_tx.withdraw_events = withdraw_events;
        processed_tx.uniswap_v2_pair_created_events = uniswap_v2_pair_created_events;
        processed_tx.ownership_transferred_events = ownership_transferred_events;
        processed_tx.ownership_transfer_started_events = ownership_transfer_started_events;
        processed_tx.access_control_role_granted_events = access_control_role_granted_events;
        processed_tx.access_control_role_revoked_events = access_control_role_revoked_events;
        processed_tx.proxy_admin_changed_events = proxy_admin_changed_events;
        processed_tx.trading_enabled_events = trading_enabled_events;
        processed_tx.trading_disabled_events = trading_disabled_events;
        processed_tx.other_events = other_events;
        processed_tx.access_list = access_list;
        processed_tx.blob_versioned_hashes = blob_versioned_hashes;
        processed_tx.signed_authorizations = signed_authorizations;
        processed_tx.eth_transfers = eth_transfers;

        let mut erc20_contracts = Self::collect_erc20_contracts(&processed_tx);
        for approval in &processed_tx.approval_for_all_events {
            processed_tx.erc721_contracts.insert(approval.token_address);
        }
        // Exclude canonical WETH from the ERC20 contract set because the
        // balance calculator treats it as the ETH denomination.
        let weth = alloy_primitives::address!("0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2");
        erc20_contracts.remove(&weth);

        // Set address balance changes if provided
        if let Some(balance_changes) = address_balance_changes {
            processed_tx.address_balance_changes = balance_changes;
        }

        processed_tx.erc20_contracts = erc20_contracts;

        let tx_type_label = self.classify_tx_type(&processed_tx);
        self.add_tx_type_events(&tx_type_label, &mut processed_tx);

        populate_unique_addresses(&mut processed_tx);

        processed_tx.bribe_amount =
            Self::calculate_bribe_amount(&processed_tx.fees);

        processed_tx.actions = self.identify_actions(&tx_type_label, &processed_tx);
        processed_tx.tx_type = tx_type_label;

        Ok(processed_tx)
    }

    /// Process transaction from simulation result into ProcessedTransaction
    ///
    /// This is the MAIN method that takes simulation results and produces a complete
    /// ProcessedTransaction with ALL decoded events, internal transactions, and balance changes.
    /// This is what processed_tx_provider uses to convert simulation to ProcessedTransaction.
    pub async fn process_transaction_from_simulation_result(
        &self,
        unsigned_tx: &tx_simulator::UnsignedTransaction,
        simulation_result: &tx_simulator::FullSimulationResult,
        block_number: u64,
        tx_index: u64,
    ) -> Result<ProcessedTransaction> {
        // Extract transaction parameters from UnsignedTransaction
        let from = unsigned_tx.from.unwrap_or(Address::ZERO);
        let to = unsigned_tx.to;
        let value = unsigned_tx.value.unwrap_or(U256::ZERO);
        let input = unsigned_tx
            .data
            .as_ref()
            .map(|d| d.to_vec())
            .unwrap_or_default();

        let mut synthetic_hash_input = Vec::new();
        synthetic_hash_input.extend_from_slice(&block_number.to_be_bytes());
        synthetic_hash_input.extend_from_slice(&tx_index.to_be_bytes());
        synthetic_hash_input.extend_from_slice(from.as_slice());
        if let Some(to) = to {
            synthetic_hash_input.extend_from_slice(to.as_slice());
        }
        synthetic_hash_input.extend_from_slice(&input);
        let tx_hash = alloy_primitives::keccak256(synthetic_hash_input);

        let max_fee_per_gas = unsigned_tx.max_fee_per_gas.map(|fee| U256::from(fee));
        let max_priority_fee_per_gas = unsigned_tx
            .max_priority_fee_per_gas
            .map(|fee| U256::from(fee));
        let gas_price = U256::from(simulation_result.effective_gas_price.ok_or_else(|| {
            eyre!(
                "simulated tx missing effective gas price block={} tx_index={}; tx_simulator must populate it from the block gas environment",
                block_number,
                tx_index
            )
        })?);
        let gas_used = simulation_result.gas_used;
        let status = simulation_result.success;
        let nonce = unsigned_tx.nonce.unwrap_or(0);
        let gas_limit = unsigned_tx.gas.unwrap_or(300_000);
        let raw_tx_type = simulation_result.tx_type.unwrap_or_else(|| {
            if unsigned_tx.max_fee_per_gas.is_some()
                || unsigned_tx.max_priority_fee_per_gas.is_some()
            {
                2
            } else {
                0
            }
        });

        // Use current timestamp for simulation
        let block_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Process logs from simulation result using tx_log_processor
        let logs = if !simulation_result.logs.is_empty() {
            simulation_result.logs.clone()
        } else {
            self.decoder
                .extract_logs_from_call_frame(&simulation_result.call_trace)
        };

        // Extract internal transactions from call trace
        let trace_processor = TransactionTraceProcessor::new();
        let internal_transactions = trace_processor
            .extract_internal_transactions_from_call_trace(&simulation_result.call_trace);

        // First decode the logs to get ERC20 transfers
        let mut erc20_transfers = Vec::new();
        for (log_index, log) in logs.iter().enumerate() {
            if let Ok(Some(decoded_event)) = self.decoder.decode_log(log, log_index as u64) {
                if let DecodedEvent::ERC20TransferEvent(transfer) = decoded_event {
                    erc20_transfers.push(transfer);
                }
            }
        }

        // Calculate address balance changes from DECODED transfers and internal transactions
        let mut balance_calculator = AddressBalanceChangeCalculator::new();
        let address_balance_changes = Some(
            balance_calculator.calculate_balance_changes_from_processed_data(
                &erc20_transfers,
                &internal_transactions,
                block_number,
                tx_index,
            )?,
        );

        let access_list: Vec<ProcessedAccessListItem> = unsigned_tx
            .access_list
            .iter()
            .map(|item| ProcessedAccessListItem {
                address: item.address,
                storage_keys: item.storage_keys.clone(),
            })
            .collect();
        let blob_versioned_hashes = unsigned_tx.blob_versioned_hashes.clone();
        let max_fee_per_blob_gas_u256 = unsigned_tx.max_fee_per_blob_gas.map(U256::from);
        let signed_authorizations = unsigned_tx.signed_authorizations.clone();

        // Use the existing method to process everything
        let mut processed_tx = self
            .process_transaction_from_raw_data(
                tx_hash,
                block_number,
                block_timestamp,
                tx_index,
                from,
                to,
                value,
                input.clone(),
                gas_price,
                gas_used,
                status,
                nonce,
                raw_tx_type,
                max_fee_per_gas.clone(),
                max_priority_fee_per_gas.clone(),
                logs,
                gas_limit,
                access_list,
                blob_versioned_hashes,
                max_fee_per_blob_gas_u256,
                None,
                signed_authorizations,
                address_balance_changes,
            )
            .await?;

        // Set the extracted internal transactions
        processed_tx.internal_transactions = internal_transactions;
        processed_tx.struct_logs = simulation_result.struct_logs.clone();
        processed_tx.bribe_amount =
            Self::calculate_bribe_amount(&processed_tx.fees);
        processed_tx.eth_transfers = self.extract_eth_transfers(
            from,
            to,
            value,
            &input,
            &processed_tx.internal_transactions,
        );
        let tx_type_label = self.classify_tx_type(&processed_tx);
        self.add_tx_type_events(&tx_type_label, &mut processed_tx);
        populate_unique_addresses(&mut processed_tx);
        processed_tx.actions = self.identify_actions(&tx_type_label, &processed_tx);
        processed_tx.tx_type = tx_type_label;

        Ok(processed_tx)
    }

    fn classify_tx_type(&self, processed_tx: &ProcessedTransaction) -> String {
        if let Some(signature_type) = self.detect_signature_tx_type(processed_tx) {
            return signature_type.to_string();
        }

        if !processed_tx.trading_enabled_events.is_empty() {
            return "Trading Enabled".to_string();
        }
        if !processed_tx.trading_disabled_events.is_empty() {
            return "Trading Disabled".to_string();
        }
        if !processed_tx.contract_creation_events.is_empty() || processed_tx.to_address.is_none() {
            return "Contract Creation".to_string();
        }

        match self.classifier.classify(processed_tx) {
            TransactionType::Transfer => {
                if !processed_tx.erc20_transfers.is_empty() {
                    "ERC20 Transfer".to_string()
                } else if !processed_tx.erc721_transfers.is_empty() {
                    "ERC721 Transfer".to_string()
                } else if !processed_tx.erc1155_transfers.is_empty() {
                    "ERC1155 Transfer".to_string()
                } else if !processed_tx.eth_transfers.is_empty()
                    || (processed_tx.value > U256::ZERO && processed_tx.input.is_empty())
                {
                    "Ether Transfer".to_string()
                } else {
                    "Transfer".to_string()
                }
            }
            TransactionType::Swap => "Swap".to_string(),
            TransactionType::AddLiquidity => "Add Liquidity".to_string(),
            TransactionType::RemoveLiquidity => "Remove Liquidity".to_string(),
            TransactionType::Approval => "Approval".to_string(),
            TransactionType::ContractCreation => "Contract Creation".to_string(),
            TransactionType::ContractInteraction => "Contract Interaction".to_string(),
            TransactionType::Failed => "Failed".to_string(),
            TransactionType::Unknown => "Contract Interaction".to_string(),
        }
    }

    fn detect_signature_tx_type(
        &self,
        processed_tx: &ProcessedTransaction,
    ) -> Option<&'static str> {
        if processed_tx.input.len() < 4 {
            return None;
        }
        let selector = hex::encode(&processed_tx.input[..4]);
        let label = FUNCTION_SIGNATURES.get(&selector)?;
        match *label {
            "Trading Enabled" | "Set Fees/Enable Trading" => Some("Trading Enabled"),
            "Disable Trading" => Some("Trading Disabled"),
            _ => None,
        }
    }

    fn identify_actions(&self, tx_type: &str, processed_tx: &ProcessedTransaction) -> Vec<String> {
        let mut actions: Vec<String> = Vec::new();

        Self::push_action_if(
            &mut actions,
            tx_type == "Contract Creation",
            "Contract Creation",
        );
        Self::push_action_if(
            &mut actions,
            tx_type == "Trading Enabled" || !processed_tx.trading_enabled_events.is_empty(),
            "Trading Enable",
        );
        Self::push_action_if(
            &mut actions,
            tx_type == "Trading Disabled" || !processed_tx.trading_disabled_events.is_empty(),
            "Trading Disable",
        );
        if self.is_add_liquidity_action(processed_tx) {
            Self::push_action_if(&mut actions, true, "Add Liquidity");
        }
        if self.is_swap_action(processed_tx) {
            Self::push_action_if(&mut actions, true, "Swap");
        }
        Self::push_action_if(
            &mut actions,
            !processed_tx.ownership_transferred_events.is_empty()
                || !processed_tx.ownership_transfer_started_events.is_empty(),
            "Ownership Change",
        );

        actions
    }

    fn push_action_if(actions: &mut Vec<String>, condition: bool, action: &str) {
        if condition && !actions.iter().any(|existing| existing == action) {
            actions.push(action.to_string());
        }
    }

    fn add_tx_type_events(&self, tx_type: &str, processed_tx: &mut ProcessedTransaction) {
        match tx_type {
            "Trading Enabled" => {
                if let Some(token_address) = processed_tx.to_address {
                    let already_present = processed_tx
                        .trading_enabled_events
                        .iter()
                        .any(|event| event.token_address == token_address);
                    if !already_present {
                        processed_tx
                            .trading_enabled_events
                            .push(TradingEnabledEvent {
                                token_address,
                                block_number: processed_tx.block_number,
                                log_index: 0,
                            });
                    }
                    processed_tx.erc20_contracts.insert(token_address);
                }
            }
            "Trading Disabled" => {
                if let Some(token_address) = processed_tx.to_address {
                    let already_present = processed_tx
                        .trading_disabled_events
                        .iter()
                        .any(|event| event.token_address == token_address);
                    if !already_present {
                        processed_tx
                            .trading_disabled_events
                            .push(TradingDisabledEvent {
                                token_address,
                                block_number: processed_tx.block_number,
                                log_index: 0,
                            });
                    }
                    processed_tx.erc20_contracts.insert(token_address);
                }
            }
            "Contract Creation" => {
                if let Some(contract_address) = processed_tx.contract_address {
                    let already_present = processed_tx
                        .contract_creation_events
                        .iter()
                        .any(|event| event.contract_address == contract_address);
                    if !already_present {
                        processed_tx
                            .contract_creation_events
                            .push(ContractCreationEvent { contract_address });
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_erc20_contracts(processed_tx: &ProcessedTransaction) -> HashSet<Address> {
        let mut erc20_contracts = HashSet::new();
        for transfer in &processed_tx.erc20_transfers {
            erc20_contracts.insert(transfer.token_address);
        }
        for approval in &processed_tx.erc20_approval_events {
            erc20_contracts.insert(approval.token_address);
        }
        for event in &processed_tx.trading_enabled_events {
            erc20_contracts.insert(event.token_address);
        }
        for event in &processed_tx.trading_disabled_events {
            erc20_contracts.insert(event.token_address);
        }
        for pool_created in &processed_tx.uniswap_v2_pair_created_events {
            erc20_contracts.insert(pool_created.token0);
            erc20_contracts.insert(pool_created.token1);
        }
        for pool_created in &processed_tx.uniswap_v3_pools {
            erc20_contracts.insert(pool_created.token0);
            erc20_contracts.insert(pool_created.token1);
        }
        for permit in &processed_tx.permit2_events {
            erc20_contracts.insert(permit.token);
        }
        for deposit in &processed_tx.deposit_events {
            if let Some(token) = deposit.token_address {
                erc20_contracts.insert(token);
            }
        }
        erc20_contracts
    }

    fn is_add_liquidity_action(&self, tx: &ProcessedTransaction) -> bool {
        self.is_add_liquidity_action_v2(tx) || self.is_add_liquidity_action_v3(tx)
    }

    fn is_add_liquidity_action_v2(&self, tx: &ProcessedTransaction) -> bool {
        !tx.uniswap_v2_syncs.is_empty() && !tx.uniswap_v2_mints.is_empty()
    }

    fn is_add_liquidity_action_v3(&self, tx: &ProcessedTransaction) -> bool {
        let has_position = !tx.uniswap_v3_positions.is_empty();
        if !has_position {
            return false;
        }

        let has_transfers = tx.erc20_transfers.len() >= 2;
        if !has_transfers {
            return false;
        }

        let has_pool_created = !tx.uniswap_v3_pools.is_empty();
        let has_initialization = !tx.uniswap_v3_initializations.is_empty();

        if has_pool_created {
            has_initialization && has_position && has_transfers
        } else {
            has_position && has_transfers
        }
    }

    fn is_swap_action(&self, tx: &ProcessedTransaction) -> bool {
        !tx.uniswap_v2_swaps.is_empty()
            || !tx.uniswap_v3_swaps.is_empty()
            || !tx.uniswap_v4_swaps.is_empty()
    }

    fn extract_eth_transfers(
        &self,
        from: Address,
        to: Option<Address>,
        value: U256,
        input: &[u8],
        internal_transactions: &[InternalTransaction],
    ) -> Vec<ETHTransfer> {
        let mut transfers = Vec::new();
        let is_simple_transfer = value > U256::ZERO
            && input.is_empty()
            && internal_transactions.is_empty()
            && to.is_some();

        if is_simple_transfer {
            if let Some(to_address) = to {
                transfers.push(ETHTransfer {
                    from_address: from,
                    to_address,
                    amount: value,
                });
            }
        }

        transfers
    }

    pub(crate) fn calculate_bribe_amount(fees: &TransactionFees) -> U256 {
        fees.max_priority_fee
            .unwrap_or(U256::ZERO)
            .saturating_mul(U256::from(fees.gas_used))
    }
}

impl Default for TxProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};

    use crate::tx_processor::data_models::{UniswapV2PairCreatedEvent, UniswapV3PoolCreatedEvent};

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            1,
            1_700,
            0,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn erc20_contracts_include_v2_pair_created_tokens() {
        let mut tx = tx();
        let token0 = address!("1111111111111111111111111111111111111111");
        let token1 = address!("2222222222222222222222222222222222222222");
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("3333333333333333333333333333333333333333"),
                token0,
                token1,
                factory_address: address!("5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f"),
                log_index: 0,
            });

        let contracts = TxProcessor::collect_erc20_contracts(&tx);

        assert!(contracts.contains(&token0));
        assert!(contracts.contains(&token1));
    }

    #[test]
    fn erc20_contracts_include_v3_pool_created_tokens() {
        let mut tx = tx();
        let token0 = address!("1111111111111111111111111111111111111111");
        let token1 = address!("2222222222222222222222222222222222222222");
        tx.uniswap_v3_pools.push(UniswapV3PoolCreatedEvent {
            factory_address: address!("1f98431c8ad98523631ae4a59f267346ea31f984"),
            token0,
            token1,
            fee: 3_000,
            tick_spacing: 60,
            pool: address!("3333333333333333333333333333333333333333"),
            log_index: 0,
        });

        let contracts = TxProcessor::collect_erc20_contracts(&tx);

        assert!(contracts.contains(&token0));
        assert!(contracts.contains(&token1));
    }
}
