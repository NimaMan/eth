/// Transaction Processor - Rust equivalent of Python TransactionProcessor
/// 
/// OBJECTIVE: Process simulation results into complete ProcessedTransaction objects
/// with ALL decoded events (ERC20Transfer, MintAction, UniswapV2Sync, etc.)
/// 
/// This is the core transaction processing logic that:
/// 1. Takes simulation results (logs, traces, balance changes)
/// 2. Decodes ALL event logs using LogDecoder 
/// 3. Processes internal transactions from traces
/// 4. Converts balance changes to proper format
/// 5. Creates complete ProcessedTransaction object
/// 
/// Key insight: Uses the WORKING approach from chain_state_persisting_sequential_tx_simulator.rs
/// instead of the broken manual filtering approach that was throwing away events.

use super::data_models::{ProcessedTransaction, TransactionFees};
use super::{LogDecoder, TransactionClassifier, DecodedEvent, TransactionTraceProcessor, AddressBalanceChangeCalculator};
use eyre::Result;
use alloy_primitives::{Address, U256, B256, Bytes};
use std::collections::HashMap;

/// Core transaction processing logic equivalent to Python's TransactionProcessor
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
    /// This is the Rust equivalent of Python's process_transaction() method.
    /// It takes raw transaction data and converts it into a complete ProcessedTransaction
    /// with ALL decoded events, just like the Python version does with log_processor.process_logs().
    /// 
    /// Based on the WORKING approach from chain_state_persisting_sequential_tx_simulator.rs:292-308
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
        status: String,
        nonce: u64,
        logs: Vec<alloy_primitives::Log>,
        gas_limit: u64,
        address_balance_changes: Option<HashMap<Address, super::data_models::AddressBalanceChange>>,
    ) -> Result<ProcessedTransaction> {
        
        // STEP 1: Decode ALL event logs using LogDecoder (just like Python's log_processor.process_logs)
        let mut erc20_transfers = Vec::new();
        let mut erc721_transfers = Vec::new(); 
        let mut erc1155_transfers = Vec::new();
        let mut approvals = Vec::new();
        let mut erc721_approvals = Vec::new();
        let mut uniswap_v2_syncs = Vec::new();
        let mut uniswap_v2_swaps = Vec::new();
        let mut uniswap_v3_swaps = Vec::new();
        let mut uniswap_v3_mints = Vec::new();
        let mut uniswap_v3_burns = Vec::new();
        let mut uniswap_v3_positions = Vec::new();
        let mut mints = Vec::new();
        let mut burns = Vec::new();
        let mut deposits = Vec::new();
        let mut withdraws = Vec::new();
        let mut pair_events = Vec::new();
        let mut owner_events = Vec::new();
        let mut trading_enabled_events = Vec::new();
        let mut trading_disabled_events = Vec::new();
        
        for (log_index, log) in logs.iter().enumerate() {
            if let Ok(Some(decoded_event)) = self.decoder.decode_log(log, log_index as u64) {
                // Store ALL event types in their respective arrays
                match decoded_event {
                    DecodedEvent::ERC20Transfer(transfer) => {
                        erc20_transfers.push(transfer);
                    },
                    DecodedEvent::ERC721Transfer(transfer) => {
                        erc721_transfers.push(transfer);
                    },
                    DecodedEvent::ERC1155Transfer(transfer) => {
                        erc1155_transfers.push(transfer);
                    },
                    DecodedEvent::ERC20Approval(approval) => {
                        approvals.push(approval);
                    },
                    DecodedEvent::ERC721Approval(approval) => {
                        erc721_approvals.push(approval);
                    },
                    DecodedEvent::UniswapV2Sync(sync) => {
                        uniswap_v2_syncs.push(sync);
                    },
                    DecodedEvent::UniswapV2Swap(swap) => {
                        uniswap_v2_swaps.push(swap);
                    },
                    DecodedEvent::UniswapV3Swap(swap) => {
                        uniswap_v3_swaps.push(swap);
                    },
                    DecodedEvent::UniswapV3Mint(mint_v3) => {
                        uniswap_v3_mints.push(mint_v3);
                    },
                    DecodedEvent::UniswapV3Burn(burn_v3) => {
                        uniswap_v3_burns.push(burn_v3);
                    },
                    DecodedEvent::UniswapV3Position(position) => {
                        uniswap_v3_positions.push(position);
                    },
                    DecodedEvent::MintAction(mint_action) => {
                        mints.push(mint_action);
                    },
                    DecodedEvent::BurnAction(burn_action) => {
                        burns.push(burn_action);
                    },
                    DecodedEvent::DepositAction(deposit) => {
                        deposits.push(deposit);
                    },
                    DecodedEvent::WithdrawAction(withdraw) => {
                        withdraws.push(withdraw);
                    },
                    DecodedEvent::PairAction(pair) => {
                        pair_events.push(pair);
                    },
                    DecodedEvent::OwnerEvent(owner) => {
                        owner_events.push(owner);
                    },
                    DecodedEvent::TradingEnabledEvent(enabled) => {
                        trading_enabled_events.push(enabled);
                    },
                    DecodedEvent::TradingDisabledEvent(disabled) => {
                        trading_disabled_events.push(disabled);
                    },
                    // TODO: Add all other event types (V3/V4 events, etc.)
                    _ => {
                        // For now, ignore unhandled events
                    }
                }
            }
        }
        
        // STEP 2: Create transaction fees
        let fees = TransactionFees {
            gas_price,
            gas_used,
            txn_fee: gas_price * U256::from(gas_used),
            max_fee_per_gas: None, // Not available from simulation
            max_priority_fee: None, // Not available from simulation
            protocol_type: "simulation".to_string(),
        };
        
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
            input,
        );
        
        // STEP 4: Set all the decoded data
        processed_tx.fees = fees;
        processed_tx.erc20_transfers = erc20_transfers;
        processed_tx.erc721_transfers = erc721_transfers;
        processed_tx.erc1155_transfers = erc1155_transfers;
        processed_tx.approvals = approvals;
        processed_tx.erc721_approvals = erc721_approvals;
        processed_tx.uniswap_v2_syncs = uniswap_v2_syncs;
        processed_tx.uniswap_v2_swaps = uniswap_v2_swaps;
        processed_tx.uniswap_v3_swaps = uniswap_v3_swaps;
        processed_tx.uniswap_v3_mints = uniswap_v3_mints;
        processed_tx.uniswap_v3_burns = uniswap_v3_burns;
        processed_tx.uniswap_v3_positions = uniswap_v3_positions;
        processed_tx.mints = mints;
        processed_tx.burns = burns;
        processed_tx.deposits = deposits;
        processed_tx.withdraws = withdraws;
        processed_tx.pair_events = pair_events;
        processed_tx.owner_events = owner_events;
        processed_tx.trading_enabled_events = trading_enabled_events;
        processed_tx.trading_disabled_events = trading_disabled_events;
        
        // Set address balance changes if provided
        if let Some(balance_changes) = address_balance_changes {
            processed_tx.address_balance_changes = balance_changes;
        }
        
        // STEP 5: Classify transaction type based on decoded events
        // TODO: Implement proper classification logic
        processed_tx.txn_type = self.determine_transaction_type(&processed_tx);
        
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
        
        // Generate synthetic transaction hash for simulation
        let tx_hash = B256::random();
        
        // Extract transaction parameters from UnsignedTransaction
        let from = unsigned_tx.from.unwrap_or(Address::ZERO);
        let to = unsigned_tx.to;
        let value = unsigned_tx.value.unwrap_or(U256::ZERO);
        let input = unsigned_tx.data.as_ref().map(|d| d.to_vec()).unwrap_or_default();
        let gas_price = U256::from(unsigned_tx.gas_price.unwrap_or(20_000_000_000));
        let gas_used = simulation_result.gas_used;
        let status = if simulation_result.success { "1".to_string() } else { "0".to_string() };
        let nonce = unsigned_tx.nonce.unwrap_or(0);
        let gas_limit = unsigned_tx.gas.unwrap_or(300_000);
        
        // Use current timestamp for simulation
        let block_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Process logs from simulation result using tx_log_processor
        let logs = self.decoder.extract_logs_from_call_frame(&simulation_result.call_trace);
        
        // Extract internal transactions from call trace
        let trace_processor = TransactionTraceProcessor::new();
        let internal_transactions = trace_processor.extract_internal_transactions_from_call_trace(
            &simulation_result.call_trace
        );
        
        // First decode the logs to get ERC20 transfers
        let mut erc20_transfers = Vec::new();
        for (log_index, log) in logs.iter().enumerate() {
            if let Ok(Some(decoded_event)) = self.decoder.decode_log(log, log_index as u64) {
                if let DecodedEvent::ERC20Transfer(transfer) = decoded_event {
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
            )?
        );
        
        // Use the existing method to process everything
        let mut processed_tx = self.process_transaction_from_raw_data(
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
            logs,
            gas_limit,
            address_balance_changes,
        ).await?;
        
        // Set the extracted internal transactions
        processed_tx.internal_transactions = internal_transactions;
        
        Ok(processed_tx)
    }
    
    /// Determine transaction type based on decoded events and transaction data
    fn determine_transaction_type(&self, processed_tx: &ProcessedTransaction) -> String {
        // Simple classification logic for now
        if !processed_tx.erc20_transfers.is_empty() {
            "ERC20_TRANSFER".to_string()
        } else if !processed_tx.erc721_transfers.is_empty() {
            "ERC721_TRANSFER".to_string()
        } else if !processed_tx.erc1155_transfers.is_empty() {
            "ERC1155_TRANSFER".to_string()
        } else if processed_tx.value > U256::ZERO {
            "ETH_TRANSFER".to_string()
        } else {
            "CONTRACT_INTERACTION".to_string()
        }
    }
}

impl Default for TxProcessor {
    fn default() -> Self {
        Self::new()
    }
}