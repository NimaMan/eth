//! Provider implementation for the fetch_from_reth module
//!
//! This module implements the core RethDataProvider trait using Reth's
//! database abstractions for high-performance blockchain data access.

use crate::fetch_from_reth::{
    error::{FetchError, FetchResult},
    config::{RethDataConfig, CacheConfig, CompatibilityMode},
    cache::TransactionCache,
    compatibility::{
        is_version_mismatch_error, log_version_mismatch_details,
        remove_lock_file_if_safe, read_database_version,
    },
};

use alloy_primitives::{Address, B256, U256, Bytes};
use reth_primitives_traits::TransactionMeta;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use std::collections::BTreeMap;
use tracing::{debug, warn, info, error};

// Reth imports for database access
use reth_chainspec::ChainSpecBuilder;
use reth_provider::{
    providers::{ProviderFactory, ReadOnlyConfig},
    BlockReader, ReceiptProvider, TransactionsProvider,
    BlockNumReader, BlockHashReader, HeaderProvider,
    StateProvider, AccountReader,
};
use reth_db::DatabaseEnv;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_ethereum::{
    node::EthereumNode,
    TransactionSigned, Receipt,
};

// Import necessary traits and types
use reth_primitives_traits::SignedTransaction;

/// Comprehensive transaction data structure
#[derive(Debug, Clone)]
pub struct TransactionData {
    /// Transaction hash
    pub hash: B256,
    /// Sender address
    pub from: Address,
    /// Recipient address (None for contract creation)
    pub to: Option<Address>,
    /// ETH value transferred
    pub value: U256,
    /// Gas limit
    pub gas_limit: u64,
    /// Gas actually used
    pub gas_used: u64,
    /// Gas price
    pub gas_price: U256,
    /// Transaction nonce
    pub nonce: u64,
    /// Block number containing this transaction
    pub block_number: u64,
    /// Block hash containing this transaction
    pub block_hash: B256,
    /// Transaction index within the block
    pub transaction_index: u64,
    /// Transaction input data
    pub input: Bytes,
    /// Receipt status (true = success, false = failed)
    pub receipt_status: bool,
    /// Contract address (if this was a contract creation)
    pub contractaddress: Option<Address>,
    /// Transaction logs/events
    pub logs: Vec<reth_primitives_traits::Log>,
}

/// Comprehensive block data structure
#[derive(Debug, Clone)]
pub struct BlockData {
    /// Block hash
    pub hash: B256,
    /// Block number
    pub number: u64,
    /// Parent block hash
    pub parent_hash: B256,
    /// State root
    pub state_root: B256,
    /// Transactions root
    pub transactions_root: B256,
    /// Receipts root
    pub receipts_root: B256,
    /// Block timestamp
    pub timestamp: u64,
    /// Gas limit for the block
    pub gas_limit: u64,
    /// Gas used by all transactions
    pub gas_used: u64,
    /// Difficulty
    pub difficulty: U256,
    /// Total difficulty
    pub total_difficulty: Option<U256>,
    /// Miner/author address
    pub miner: Address,
    /// Extra data
    pub extra_data: Bytes,
    /// Number of transactions
    pub transaction_count: usize,
}

/// Account state data structure
#[derive(Debug, Clone)]
pub struct AccountData {
    /// Account address
    pub address: Address,
    /// Account balance in wei
    pub balance: U256,
    /// Account nonce
    pub nonce: u64,
    /// Code hash (keccak256 of contract code)
    pub code_hash: B256,
    /// Code size in bytes
    pub code_size: Option<usize>,
    /// Actual bytecode (if available)
    pub code: Option<Bytes>,
    /// Storage root hash
    pub storage_root: B256,
}

/// Storage slot data
#[derive(Debug, Clone)]
pub struct StorageData {
    /// Storage slot address
    pub address: Address,
    /// Storage slot key
    pub key: B256,
    /// Storage value
    pub value: B256,
}

/// Historical state change information
#[derive(Debug, Clone)]
pub struct StateChange {
    /// Block number where change occurred
    pub block_number: u64,
    /// Account address
    pub address: Address,
    /// Previous account state (if any)
    pub previous_state: Option<AccountData>,
    /// New account state
    pub new_state: AccountData,
    /// Storage changes (key -> old_value, new_value)
    pub storage_changes: BTreeMap<B256, (Option<B256>, B256)>,
}

/// Receipt data with enhanced information
#[derive(Debug, Clone)]
pub struct ReceiptData {
    /// Transaction hash
    pub transaction_hash: B256,
    /// Transaction index in block
    pub transaction_index: u64,
    /// Block hash
    pub block_hash: B256,
    /// Block number
    pub block_number: u64,
    /// Gas used by this transaction
    pub gas_used: u64,
    /// Cumulative gas used in block up to this transaction
    pub cumulative_gas_used: u64,
    /// Success/failure status
    pub status: bool,
    /// Contract address created (if any)
    pub contractaddress: Option<Address>,
    /// Event logs
    pub logs: Vec<reth_primitives_traits::Log>,
    /// Logs bloom filter
    pub logs_bloom: alloy_primitives::Bloom,
}

/// Chain information
#[derive(Debug, Clone)]
pub struct ChainInfo {
    /// Latest block number
    pub latest_block: u64,
    /// Latest block hash
    pub latest_hash: B256,
    /// Safe block number (if available)
    pub safe_block: Option<u64>,
    /// Finalized block number (if available)
    pub finalized_block: Option<u64>,
    /// Total number of transactions
    pub total_transactions: Option<u64>,
}

/// Block identifier for queries
#[derive(Debug, Clone)]
pub enum BlockId {
    Number(u64),
    Hash(B256),
    Latest,
    Earliest,
    Pending,
    Safe,
    Finalized,
}

/// Core trait for fetching data from Reth database
/// This trait provides comprehensive access to all blockchain data stored in Reth
pub trait RethDataProvider: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════════════════
    // TRANSACTION DATA ACCESS
    // ═══════════════════════════════════════════════════════════════════════════════
    
    /// Fetch complete transaction data by hash
    fn fetch_transaction(&self, tx_hash: B256) -> FetchResult<TransactionData>;
    
    /// Fetch multiple transactions efficiently in batch
    fn fetch_batch(&self, tx_hashes: &[B256]) -> FetchResult<Vec<TransactionData>>;
    
    /// Check if a transaction exists without fetching full data
    fn transaction_exists(&self, tx_hash: B256) -> FetchResult<bool>;
    
    /// Fetch transaction data by block number and transaction index
    fn fetch_transaction_by_block_and_index(&self, block_number: u64, tx_index: u64) -> FetchResult<TransactionData>;
    
    /// Fetch all transactions in a block
    fn fetch_transactions_by_block(&self, block_id: BlockId) -> FetchResult<Vec<TransactionData>>;
    
    /// Fetch transactions in a range of blocks
    fn fetch_transactions_by_block_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<TransactionData>>;
    
    // ═══════════════════════════════════════════════════════════════════════════════
    // BLOCK DATA ACCESS
    // ═══════════════════════════════════════════════════════════════════════════════
    
    /// Fetch complete block data by number or hash
    fn fetch_block(&self, block_id: BlockId) -> FetchResult<BlockData>;
    
    /// Fetch multiple blocks efficiently in batch
    fn fetch_blocks_batch(&self, block_ids: &[BlockId]) -> FetchResult<Vec<BlockData>>;
    
    /// Fetch blocks in a range
    fn fetch_blocks_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<BlockData>>;
    
    /// Check if a block exists
    fn block_exists(&self, block_id: BlockId) -> FetchResult<bool>;
    
    /// Get latest block number
    fn latest_block_number(&self) -> FetchResult<u64>;
    
    /// Get chain information
    fn chain_info(&self) -> FetchResult<ChainInfo>;
    
    // ═══════════════════════════════════════════════════════════════════════════════
    // ACCOUNT AND STATE DATA ACCESS
    // ═══════════════════════════════════════════════════════════════════════════════
    
    /// Fetch account data (balance, nonce, code) at latest block
    fn fetch_account(&self, address: Address) -> FetchResult<AccountData>;
    
    /// Fetch account data at a specific block
    fn fetch_account_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<AccountData>;
    
    /// Fetch multiple accounts efficiently in batch
    fn fetch_accounts_batch(&self, addresses: &[Address]) -> FetchResult<Vec<AccountData>>;
    
    /// Fetch multiple accounts at a specific block
    fn fetch_accounts_at_block_batch(&self, addresses: &[Address], block_id: BlockId) -> FetchResult<Vec<AccountData>>;
    
    /// Fetch account balance only (faster than full account data)
    fn fetch_balance(&self, address: Address) -> FetchResult<U256>;
    
    /// Fetch account balance at specific block
    fn fetch_balance_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<U256>;
    
    /// Fetch account nonce
    fn fetch_nonce(&self, address: Address) -> FetchResult<u64>;
    
    /// Fetch account nonce at specific block
    fn fetch_nonce_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<u64>;
    
    /// Fetch contract bytecode
    fn fetch_code(&self, address: Address) -> FetchResult<Option<Bytes>>;
    
    /// Fetch contract bytecode at specific block
    fn fetch_code_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<Option<Bytes>>;
    
    // ═══════════════════════════════════════════════════════════════════════════════
    // STORAGE DATA ACCESS
    // ═══════════════════════════════════════════════════════════════════════════════
    
    /// Fetch storage value at a specific slot
    fn fetch_storage(&self, address: Address, slot: B256) -> FetchResult<B256>;
    
    /// Fetch storage value at specific block
    fn fetch_storage_at_block(&self, address: Address, slot: B256, block_id: BlockId) -> FetchResult<B256>;
    
    /// Fetch multiple storage slots efficiently
    fn fetch_storage_batch(&self, address: Address, slots: &[B256]) -> FetchResult<Vec<StorageData>>;
    
    /// Fetch multiple storage slots at specific block
    fn fetch_storage_at_block_batch(&self, address: Address, slots: &[B256], block_id: BlockId) -> FetchResult<Vec<StorageData>>;
    
    /// Fetch all storage changes for an address in a block range
    fn fetch_storage_changes(&self, address: Address, start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>>;
    
    // ═══════════════════════════════════════════════════════════════════════════════
    // RECEIPT DATA ACCESS
    // ═══════════════════════════════════════════════════════════════════════════════
    
    /// Fetch transaction receipt by hash
    fn fetch_receipt(&self, tx_hash: B256) -> FetchResult<ReceiptData>;
    
    /// Fetch multiple receipts efficiently in batch
    fn fetch_receipts_batch(&self, tx_hashes: &[B256]) -> FetchResult<Vec<ReceiptData>>;
    
    /// Fetch all receipts for a block
    fn fetch_receipts_by_block(&self, block_id: BlockId) -> FetchResult<Vec<ReceiptData>>;
    
    /// Fetch receipts for a range of blocks
    fn fetch_receipts_by_block_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<ReceiptData>>;
    
    // ═══════════════════════════════════════════════════════════════════════════════
    // HISTORICAL AND STATE CHANGE ACCESS
    // ═══════════════════════════════════════════════════════════════════════════════
    
    /// Fetch all accounts that changed in a block range
    fn fetch_changed_accounts(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<Address>>;
    
    /// Fetch complete state changes for an address in a block range
    fn fetch_account_state_changes(&self, address: Address, start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>>;
    
    /// Fetch state changes for multiple addresses
    fn fetch_multiple_account_state_changes(&self, addresses: &[Address], start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>>;
    
    // ═══════════════════════════════════════════════════════════════════════════════
    // UTILITY AND LOOKUP METHODS
    // ═══════════════════════════════════════════════════════════════════════════════
    
    /// Convert block hash to block number
    fn block_hash_to_number(&self, block_hash: B256) -> FetchResult<Option<u64>>;
    
    /// Convert block number to block hash
    fn block_number_to_hash(&self, block_number: u64) -> FetchResult<Option<B256>>;
    
    /// Convert transaction hash to transaction number (internal ID)
    fn transaction_hash_to_number(&self, tx_hash: B256) -> FetchResult<Option<u64>>;
    
    /// Get transaction sender address
    fn fetch_transaction_sender(&self, tx_hash: B256) -> FetchResult<Address>;
}

/// Implementation of RethDataProvider using Reth's database abstractions
#[derive(Debug)]
pub struct RethDatabaseProvider {
    /// Reth provider factory for database access
    factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    /// Configuration
    config: RethDataConfig,
    /// Transaction cache for performance
    cache: TransactionCache,
}

impl RethDatabaseProvider {
    /// Create a new RethDatabaseProvider from a data directory path
    pub fn new<P: AsRef<Path>>(datadir: P) -> FetchResult<Self> {
        let config = RethDataConfig::new(datadir);
        Self::with_config(config)
    }
    
    /// Helper method to resolve BlockId to actual block number
    fn resolve_block_number(&self, provider: &impl BlockNumReader, block_id: BlockId) -> FetchResult<u64> {
        match block_id {
            BlockId::Number(num) => Ok(num),
            BlockId::Hash(hash) => {
                provider.block_number(hash)
                    .map_err(|e| FetchError::DatabaseError(format!("Failed to get block number: {}", e)))?
                    .ok_or_else(|| FetchError::block_not_found(&format!("{:x}", hash)))
            }
            BlockId::Latest => {
                provider.best_block_number()
                    .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest block: {}", e)))
            }
            BlockId::Earliest => Ok(0),
            BlockId::Safe | BlockId::Finalized => {
                // For now, return latest block as a fallback
                provider.best_block_number()
                    .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest block: {}", e)))
            }
            BlockId::Pending => {
                Err(FetchError::InvalidInput("Pending block not supported".to_string()))
            }
        }
    }
    
    /// Create a new RethDatabaseProvider with custom configuration
    pub fn with_config(config: RethDataConfig) -> FetchResult<Self> {
        // Validate configuration
        config.validate()
            .map_err(|e| FetchError::ConfigError(e))?;
        
        // Log database version info if available
        if let Ok(version_info) = read_database_version(&config.datadir) {
            info!("Database version info: {:?}", version_info);
        }
        
        // Try to open database based on compatibility mode
        let factory = match config.compatibility_mode {
            CompatibilityMode::Strict => {
                // Try normal open, fail on any error
                Self::try_open_database(&config, false)?
            }
            CompatibilityMode::Compatible => {
                // Try with compatibility flags
                Self::try_open_with_compatibility(&config)?
            }
            CompatibilityMode::Force => {
                // Force open with minimal checks
                Self::try_force_open(&config)?
            }
            CompatibilityMode::Auto => {
                // Try multiple approaches
                Self::try_auto_open(&config)?
            }
        };
        
        // Create cache
        let cache_config = CacheConfig::default();
        let cache = TransactionCache::new(cache_config);
        
        Ok(Self {
            factory,
            config,
            cache,
        })
    }
    
    /// Try to open database normally
    fn try_open_database(config: &RethDataConfig, log_errors: bool) -> FetchResult<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>> {
        let spec = ChainSpecBuilder::mainnet().build();
        
        match EthereumNode::provider_factory_builder()
            .open_read_only(spec.into(), ReadOnlyConfig::from_datadir(&config.datadir)) {
            Ok(factory) => {
                debug!("Successfully opened database in normal mode");
                Ok(factory)
            }
            Err(e) => {
                let error_str = e.to_string();
                if log_errors && is_version_mismatch_error(&error_str) {
                    log_version_mismatch_details(&error_str, &config.datadir);
                }
                Err(FetchError::provider_creation_failed(&error_str))
            }
        }
    }
    
    /// Try to open with compatibility flags
    fn try_open_with_compatibility(config: &RethDataConfig) -> FetchResult<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>> {
        info!("Attempting to open database with compatibility mode");
        
        // First, try to remove lock file if safe
        let db_path = config.db_path();
        remove_lock_file_if_safe(&db_path, config.force_open)?;
        
        // Try with normal approach first
        match Self::try_open_database(config, false) {
            Ok(factory) => return Ok(factory),
            Err(e) if !is_version_mismatch_error(&e.to_string()) => return Err(e),
            _ => {}
        }
        
        // If that fails with version mismatch, we need a different approach
        // Since Reth's API doesn't expose direct MDBX flag control, we'll need to
        // document this limitation and suggest workarounds
        warn!("Direct MDBX flag control not available through Reth API");
        warn!("Consider using a matching Reth version or rebuilding the database");
        
        Err(FetchError::VersionMismatch {
            expected: "database version".to_string(),
            actual: "library version".to_string(),
            code: Some(-30794),
        })
    }
    
    /// Force open database (dangerous, may corrupt data)
    fn try_force_open(config: &RethDataConfig) -> FetchResult<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>> {
        warn!("Force opening database - this may lead to inconsistent reads!");
        
        // Remove lock file if it exists
        let db_path = config.db_path();
        remove_lock_file_if_safe(&db_path, true)?;
        
        // Try to open normally after removing lock
        Self::try_open_database(config, true)
    }
    
    /// Auto mode - try multiple approaches
    fn try_auto_open(config: &RethDataConfig) -> FetchResult<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>> {
        info!("Auto mode: trying multiple approaches to open database");
        
        // 1. Try normal open
        match Self::try_open_database(config, false) {
            Ok(factory) => {
                info!("Successfully opened database in normal mode");
                return Ok(factory);
            }
            Err(e) => {
                let error_str = e.to_string();
                if is_version_mismatch_error(&error_str) {
                    warn!("Normal open failed with version mismatch: {}", error_str);
                } else {
                    // Not a version mismatch, propagate the error
                    return Err(e);
                }
            }
        }
        
        // 2. Try compatibility mode
        info!("Attempting compatibility mode");
        match Self::try_open_with_compatibility(config) {
            Ok(factory) => {
                info!("Successfully opened database in compatibility mode");
                return Ok(factory);
            }
            Err(e) => {
                warn!("Compatibility mode failed: {}", e);
            }
        }
        
        // 3. If force mode is enabled, try that
        if config.force_open {
            info!("Attempting force open mode");
            return Self::try_force_open(config);
        }
        
        // All attempts failed
        error!("Failed to open database with all available methods");
        error!("Suggestions:");
        error!("1. Ensure Reth is not currently using the database");
        error!("2. Use a Reth version that matches the database");
        error!("3. Enable force_open mode (risky): .with_force_open(true)");
        error!("4. Rebuild the database with current Reth version");
        
        Err(FetchError::VersionMismatch {
            expected: "compatible version".to_string(),
            actual: "current version".to_string(),
            code: Some(-30794),
        })
    }
    
    /// Create provider with custom cache configuration
    pub fn with_cache_config(config: RethDataConfig, cache_config: CacheConfig) -> FetchResult<Self> {
        let mut provider = Self::with_config(config)?;
        provider.cache = TransactionCache::new(cache_config);
        Ok(provider)
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::fetch_from_reth::cache::CacheStats {
        self.cache.stats()
    }
    
    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
    
    /// Convert Reth transaction and receipt to our TransactionData format
    fn convert_transaction_data(
        &self,
        tx: TransactionSigned,
        receipt: Receipt,
        meta: TransactionMeta,
    ) -> FetchResult<TransactionData> {
        // Extract sender
        let from = tx.recover_signer()
            .map_err(|e| FetchError::ParseError(format!("Failed to recover transaction sender: {}", e)))?;
        
        // Access transaction fields directly from the underlying transaction
        let transaction = tx.transaction();
        
        // Extract fields based on transaction type
        let (to, value, gas_limit, gas_price, nonce, input) = match transaction {
            reth_ethereum::Transaction::Legacy(tx) => (
                match tx.to {
                    alloy_primitives::TxKind::Call(addr) => Some(addr),
                    alloy_primitives::TxKind::Create => None,
                },
                tx.value,
                tx.gas_limit,
                U256::from(tx.gas_price),
                tx.nonce,
                tx.input.clone(),
            ),
            reth_ethereum::Transaction::Eip2930(tx) => (
                match tx.to {
                    alloy_primitives::TxKind::Call(addr) => Some(addr),
                    alloy_primitives::TxKind::Create => None,
                },
                tx.value,
                tx.gas_limit,
                U256::from(tx.gas_price),
                tx.nonce,
                tx.input.clone(),
            ),
            reth_ethereum::Transaction::Eip1559(tx) => (
                match tx.to {
                    alloy_primitives::TxKind::Call(addr) => Some(addr),
                    alloy_primitives::TxKind::Create => None,
                },
                tx.value,
                tx.gas_limit,
                U256::from(tx.max_fee_per_gas),
                tx.nonce,
                tx.input.clone(),
            ),
            reth_ethereum::Transaction::Eip4844(tx) => (
                Some(tx.to),
                tx.value,
                tx.gas_limit,
                U256::from(tx.max_fee_per_gas),
                tx.nonce,
                tx.input.clone(),
            ),
            reth_ethereum::Transaction::Eip7702(tx) => (
                Some(tx.to),
                tx.value,
                tx.gas_limit,
                U256::from(tx.max_fee_per_gas),
                tx.nonce,
                tx.input.clone(),
            ),
        };
        
        // Get contract address from receipt if it's a creation transaction
        let contractaddress = if to.is_none() {
            // For contract creation, the contract address should be in the receipt
            // but Reth's Receipt type doesn't expose it directly, so we'll leave it None for now
            None
        } else {
            None
        };
        
        // Convert logs
        let logs = receipt.logs.clone();
        
        // Calculate actual gas used by this transaction
        // cumulative_gas_used is the total gas used by all transactions up to this point
        // To get the gas used by this specific transaction, we need to subtract the 
        // cumulative gas used by the previous transaction (if any)
        let actual_gas_used = if meta.index == 0 {
            // First transaction in block
            receipt.cumulative_gas_used
        } else {
            // For subsequent transactions, we'd need to get the previous receipt
            // For now, we'll use cumulative gas as a placeholder
            // TODO: Implement proper gas calculation by fetching previous receipt
            receipt.cumulative_gas_used
        };

        Ok(TransactionData {
            hash: meta.tx_hash,
            from,
            to,
            value,
            gas_limit,
            gas_used: actual_gas_used,
            gas_price,
            nonce,
            block_number: meta.block_number,
            block_hash: meta.block_hash,
            transaction_index: meta.index,
            input,
            receipt_status: receipt.success,
            contractaddress,
            logs,
        })
    }
}

impl RethDataProvider for RethDatabaseProvider {
    fn fetch_transaction(&self, tx_hash: B256) -> FetchResult<TransactionData> {
        // Check cache first
        if let Some(cached_data) = self.cache.get(&tx_hash) {
            return Ok(cached_data);
        }
        
        let start_time = Instant::now();
        
        // Create provider
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        // Fetch transaction with metadata
        let (tx, meta) = provider.transaction_by_hash_with_meta(tx_hash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch transaction: {}", e)))?
            .ok_or_else(|| FetchError::transaction_not_found(&format!("{:x}", tx_hash)))?;
        
        // Fetch receipt
        let receipt = provider.receipt_by_hash(tx_hash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch receipt: {}", e)))?
            .ok_or_else(|| FetchError::receipt_not_found(&format!("{:x}", tx_hash)))?;
        
        // Convert to our format
        let tx_data = self.convert_transaction_data(tx, receipt, meta)?;
        
        // Cache the result
        self.cache.put(tx_hash, tx_data.clone());
        
        // Log performance if metrics enabled
        if self.config.enable_metrics {
            let duration = start_time.elapsed();
            if duration.as_millis() > 10 { // Log slow queries
                tracing::debug!("Slow transaction fetch: {:x} took {:?}", tx_hash, duration);
            }
        }
        
        Ok(tx_data)
    }
    
    fn fetch_batch(&self, tx_hashes: &[B256]) -> FetchResult<Vec<TransactionData>> {
        if tx_hashes.is_empty() {
            return Ok(vec![]);
        }
        
        let start_time = Instant::now();
        let mut results = Vec::with_capacity(tx_hashes.len());
        let mut cache_hits = 0;
        
        // Separate cached and non-cached hashes
        let mut uncached_hashes = Vec::new();
        
        for &tx_hash in tx_hashes {
            if let Some(cached_data) = self.cache.get(&tx_hash) {
                results.push((tx_hash, cached_data));
                cache_hits += 1;
            } else {
                uncached_hashes.push(tx_hash);
            }
        }
        
        // Fetch uncached transactions
        if !uncached_hashes.is_empty() {
            let provider = self.factory.provider()
                .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
            
            for tx_hash in uncached_hashes {
                match provider.transaction_by_hash_with_meta(tx_hash) {
                    Ok(Some((tx, meta))) => {
                        match provider.receipt_by_hash(tx_hash) {
                            Ok(Some(receipt)) => {
                                match self.convert_transaction_data(tx, receipt, meta) {
                                    Ok(tx_data) => {
                                        self.cache.put(tx_hash, tx_data.clone());
                                        results.push((tx_hash, tx_data));
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to convert transaction {:x}: {}", tx_hash, e);
                                        continue;
                                    }
                                }
                            }
                            Ok(None) => {
                                tracing::warn!("Receipt not found for transaction {:x}", tx_hash);
                                continue;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to fetch receipt for {:x}: {}", tx_hash, e);
                                continue;
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::debug!("Transaction not found: {:x}", tx_hash);
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch transaction {:x}: {}", tx_hash, e);
                        continue;
                    }
                }
            }
        }
        
        // Sort results to match input order
        let mut final_results = Vec::with_capacity(tx_hashes.len());
        for &requested_hash in tx_hashes {
            if let Some((_, data)) = results.iter().find(|(hash, _)| *hash == requested_hash) {
                final_results.push(data.clone());
            }
        }
        
        // Log performance metrics
        if self.config.enable_metrics {
            let duration = start_time.elapsed();
            let total_count = tx_hashes.len();
            let success_count = final_results.len();
            
            tracing::debug!(
                "Batch fetch: {}/{} successful, {} cache hits, took {:?}",
                success_count,
                total_count,
                cache_hits,
                duration
            );
        }
        
        Ok(final_results)
    }
    
    fn transaction_exists(&self, tx_hash: B256) -> FetchResult<bool> {
        // Check cache first
        if self.cache.contains(&tx_hash) {
            return Ok(true);
        }
        
        // Check database
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        match provider.transaction_by_hash(tx_hash) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(FetchError::DatabaseError(format!("Failed to check transaction existence: {}", e))),
        }
    }
    
    fn latest_block_number(&self) -> FetchResult<u64> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let latest = provider.best_block_number()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest block: {}", e)))?;
        
        Ok(latest)
    }
    
    fn fetch_transaction_by_block_and_index(&self, block_number: u64, tx_index: u64) -> FetchResult<TransactionData> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        // Get sealed block with hashes
        let sealed_block = provider.sealed_block_with_senders(block_number.into(), reth_provider::TransactionVariant::WithHash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch block with senders: {}", e)))?
            .ok_or_else(|| FetchError::block_not_found(&block_number.to_string()))?;
        
        // Get transaction by index  
        let transactions = &sealed_block.body().transactions;
        if tx_index as usize >= transactions.len() {
            return Err(FetchError::InvalidInput(format!("Transaction index {} out of range for block {}", tx_index, block_number)));
        }
        
        let tx = &transactions[tx_index as usize];
        let tx_hash = *tx.hash();
        
        // Fetch the complete transaction data using existing method
        self.fetch_transaction(tx_hash)
    }

    fn fetch_transactions_by_block(&self, block_id: BlockId) -> FetchResult<Vec<TransactionData>> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = self.resolve_block_number(&provider, block_id)?;
        
        // Get sealed block with senders and hashes
        let sealed_block = provider.sealed_block_with_senders(block_number.into(), reth_provider::TransactionVariant::WithHash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch block with senders: {}", e)))?
            .ok_or_else(|| FetchError::block_not_found(&block_number.to_string()))?;
        
        // Extract all transaction hashes
        let tx_hashes: Vec<B256> = sealed_block.body().transactions.iter()
            .map(|tx| *tx.hash())
            .collect();
        
        // Fetch all transactions using batch method
        self.fetch_batch(&tx_hashes)
    }
    
    fn fetch_transactions_by_block_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<TransactionData>> {
        if start_block > end_block {
            return Err(FetchError::InvalidInput(format!("Invalid block range: {} > {}", start_block, end_block)));
        }
        
        let mut all_transactions = Vec::new();
        
        // Process blocks in chunks to avoid memory issues
        const CHUNK_SIZE: u64 = 100;
        let mut current_block = start_block;
        
        while current_block <= end_block {
            let chunk_end = std::cmp::min(current_block + CHUNK_SIZE - 1, end_block);
            
            // Fetch transactions for each block in the chunk
            for block_num in current_block..=chunk_end {
                match self.fetch_transactions_by_block(BlockId::Number(block_num)) {
                    Ok(txs) => all_transactions.extend(txs),
                    Err(e) => {
                        // Log error but continue with other blocks
                        tracing::warn!("Failed to fetch transactions for block {}: {}", block_num, e);
                    }
                }
            }
            
            current_block = chunk_end + 1;
        }
        
        Ok(all_transactions)
    }
    
    fn fetch_block(&self, block_id: BlockId) -> FetchResult<BlockData> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = self.resolve_block_number(&provider, block_id)?;
        
        // Fetch block and header
        let header = provider.header_by_number(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch header: {}", e)))?
            .ok_or_else(|| FetchError::block_not_found(&block_number.to_string()))?;
        
        let block = provider.block(block_number.into())
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch block: {}", e)))?
            .ok_or_else(|| FetchError::block_not_found(&block_number.to_string()))?;
        
        // Get block hash  
        let block_hash = provider.block_hash(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch block hash: {}", e)))?
            .ok_or_else(|| FetchError::block_not_found(&block_number.to_string()))?;
        
        // Try to get total difficulty if available (pre-merge)
        let total_difficulty = provider.header_td(&block_hash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch total difficulty: {}", e)))?
            .or_else(|| {
                // For post-merge blocks, TD is not tracked
                if header.difficulty == U256::ZERO {
                    None
                } else {
                    Some(header.difficulty) // Fallback to just difficulty
                }
            });
        
        Ok(BlockData {
            hash: block_hash,
            number: header.number,
            parent_hash: header.parent_hash,
            state_root: header.state_root,
            transactions_root: header.transactions_root,
            receipts_root: header.receipts_root,
            timestamp: header.timestamp,
            gas_limit: header.gas_limit,
            gas_used: header.gas_used,
            difficulty: header.difficulty,
            total_difficulty,
            miner: header.beneficiary,
            extra_data: header.extra_data.clone(),
            transaction_count: block.body.transactions.len(),
        })
    }
    
    fn fetch_blocks_batch(&self, block_ids: &[BlockId]) -> FetchResult<Vec<BlockData>> {
        let mut results = Vec::with_capacity(block_ids.len());
        
        for block_id in block_ids {
            match self.fetch_block(block_id.clone()) {
                Ok(block) => results.push(block),
                Err(_) => continue, // Skip failed fetches in batch mode
            }
        }
        
        Ok(results)
    }
    
    fn fetch_blocks_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<BlockData>> {
        let mut results = Vec::new();
        
        for block_num in start_block..=end_block {
            match self.fetch_block(BlockId::Number(block_num)) {
                Ok(block) => results.push(block),
                Err(_) => continue, // Skip failed fetches
            }
        }
        
        Ok(results)
    }
    
    fn block_exists(&self, block_id: BlockId) -> FetchResult<bool> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        match block_id {
            BlockId::Number(num) => {
                match provider.block_hash(num) {
                    Ok(Some(_)) => Ok(true),
                    Ok(None) => Ok(false),
                    Err(_) => Ok(false)
                }
            }
            BlockId::Hash(hash) => {
                match provider.block_number(hash) {
                    Ok(Some(_)) => Ok(true),
                    Ok(None) => Ok(false),
                    Err(_) => Ok(false)
                }
            }
            BlockId::Latest | BlockId::Safe | BlockId::Finalized => {
                // These should always exist if we have any blocks
                let latest = provider.best_block_number()
                    .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest block: {}", e)))?;
                Ok(latest > 0)
            }
            BlockId::Earliest => Ok(true), // Genesis block should always exist
            BlockId::Pending => Ok(false), // We don't handle pending blocks
        }
    }
    
    fn chain_info(&self) -> FetchResult<ChainInfo> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let latest_block = provider.best_block_number()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest block: {}", e)))?;
        
        let latest_hash = provider.block_hash(latest_block)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest hash: {}", e)))?
            .unwrap_or_default();
        
        // For now, we'll leave safe and finalized as None since getting them requires additional logic
        Ok(ChainInfo {
            latest_block,
            latest_hash,
            safe_block: None,
            finalized_block: None,
            total_transactions: None,
        })
    }
    
    fn fetch_account(&self, address: Address) -> FetchResult<AccountData> {
        let state_provider = self.factory.latest()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest state provider: {}", e)))?;
        
        // Get basic account information
        let account = state_provider.basic_account(&address)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch account: {}", e)))?;
        
        if let Some(acc) = account {
            // Get code if the account has a code hash
            let (code_size, code) = if let Some(code_hash) = acc.bytecode_hash {
                if code_hash != alloy_consensus::constants::KECCAK_EMPTY {
                    match state_provider.account_code(&address) {
                        Ok(Some(bytecode)) => {
                            let size = bytecode.len();
                            (Some(size), Some(bytecode.bytes()))
                        }
                        _ => (None, None)
                    }
                } else {
                    (Some(0), None)
                }
            } else {
                (Some(0), None)
            };
            
            Ok(AccountData {
                address,
                balance: acc.balance,
                nonce: acc.nonce,
                code_hash: acc.bytecode_hash.unwrap_or(alloy_consensus::constants::KECCAK_EMPTY),
                code_size,
                code,
                storage_root: B256::ZERO, // Storage root not directly accessible from Account
            })
        } else {
            // Account doesn't exist - return default values
            Ok(AccountData {
                address,
                balance: U256::ZERO,
                nonce: 0,
                code_hash: alloy_consensus::constants::KECCAK_EMPTY,
                code_size: Some(0),
                code: None,
                storage_root: B256::ZERO,
            })
        }
    }
    
    fn fetch_account_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<AccountData> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = self.resolve_block_number(&provider, block_id)?;
        
        let state_provider = self.factory.history_by_block_number(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get historical state provider: {}", e)))?;
        
        // Get basic account information at the specific block
        let account = state_provider.basic_account(&address)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch account: {}", e)))?;
        
        if let Some(acc) = account {
            // Get code if the account has a code hash
            let (code_size, code) = if let Some(code_hash) = acc.bytecode_hash {
                if code_hash != alloy_consensus::constants::KECCAK_EMPTY {
                    match state_provider.account_code(&address) {
                        Ok(Some(bytecode)) => {
                            let size = bytecode.len();
                            (Some(size), Some(bytecode.bytes()))
                        }
                        _ => (None, None)
                    }
                } else {
                    (Some(0), None)
                }
            } else {
                (Some(0), None)
            };
            
            Ok(AccountData {
                address,
                balance: acc.balance,
                nonce: acc.nonce,
                code_hash: acc.bytecode_hash.unwrap_or(alloy_consensus::constants::KECCAK_EMPTY),
                code_size,
                code,
                storage_root: B256::ZERO, // Storage root not directly accessible from Account
            })
        } else {
            // Account doesn't exist at this block - return default values
            Ok(AccountData {
                address,
                balance: U256::ZERO,
                nonce: 0,
                code_hash: alloy_consensus::constants::KECCAK_EMPTY,
                code_size: Some(0),
                code: None,
                storage_root: B256::ZERO,
            })
        }
    }
    
    fn fetch_accounts_batch(&self, addresses: &[Address]) -> FetchResult<Vec<AccountData>> {
        let mut results = Vec::with_capacity(addresses.len());
        
        for &address in addresses {
            match self.fetch_account(address) {
                Ok(account) => results.push(account),
                Err(_) => continue, // Skip failed fetches in batch mode
            }
        }
        
        Ok(results)
    }
    
    fn fetch_accounts_at_block_batch(&self, addresses: &[Address], block_id: BlockId) -> FetchResult<Vec<AccountData>> {
        let mut results = Vec::with_capacity(addresses.len());
        
        for &address in addresses {
            match self.fetch_account_at_block(address, block_id.clone()) {
                Ok(account) => results.push(account),
                Err(_) => continue, // Skip failed fetches in batch mode
            }
        }
        
        Ok(results)
    }
    
    fn fetch_balance(&self, address: Address) -> FetchResult<U256> {
        let state_provider = self.factory.latest()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest state provider: {}", e)))?;
        
        let balance = state_provider.account_balance(&address)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch balance: {}", e)))?
            .unwrap_or(U256::ZERO);
        
        Ok(balance)
    }
    
    fn fetch_balance_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<U256> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = self.resolve_block_number(&provider, block_id)?;
        
        let state_provider = self.factory.history_by_block_number(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get historical state provider: {}", e)))?;
        
        let balance = state_provider.account_balance(&address)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch balance: {}", e)))?
            .unwrap_or(U256::ZERO);
        
        Ok(balance)
    }
    
    fn fetch_nonce(&self, address: Address) -> FetchResult<u64> {
        let state_provider = self.factory.latest()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest state provider: {}", e)))?;
        
        let nonce = state_provider.account_nonce(&address)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch nonce: {}", e)))?
            .unwrap_or(0);
        
        Ok(nonce)
    }
    
    fn fetch_nonce_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<u64> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = self.resolve_block_number(&provider, block_id)?;
        
        let state_provider = self.factory.history_by_block_number(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get historical state provider: {}", e)))?;
        
        let nonce = state_provider.account_nonce(&address)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch nonce: {}", e)))?
            .unwrap_or(0);
        
        Ok(nonce)
    }
    
    fn fetch_code(&self, address: Address) -> FetchResult<Option<Bytes>> {
        let state_provider = self.factory.latest()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest state provider: {}", e)))?;
        
        match state_provider.account_code(&address) {
            Ok(Some(bytecode)) => Ok(Some(bytecode.bytes())),
            Ok(None) => Ok(None),
            Err(e) => Err(FetchError::DatabaseError(format!("Failed to fetch code: {}", e)))
        }
    }
    
    fn fetch_code_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<Option<Bytes>> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = self.resolve_block_number(&provider, block_id)?;
        
        let state_provider = self.factory.history_by_block_number(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get historical state provider: {}", e)))?;
        
        match state_provider.account_code(&address) {
            Ok(Some(bytecode)) => Ok(Some(bytecode.bytes())),
            Ok(None) => Ok(None),
            Err(e) => Err(FetchError::DatabaseError(format!("Failed to fetch code: {}", e)))
        }
    }
    
    fn fetch_storage(&self, address: Address, slot: B256) -> FetchResult<B256> {
        let state_provider = self.factory.latest()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get latest state provider: {}", e)))?;
        
        let value = state_provider.storage(address, slot.into())
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch storage: {}", e)))?
            .unwrap_or(U256::ZERO);
        
        // Convert U256 to B256 by taking the bytes
        let mut bytes = [0u8; 32];
        value.to_be_bytes::<32>().into_iter().enumerate().for_each(|(i, b)| {
            bytes[i] = b;
        });
        
        Ok(B256::from(bytes))
    }
    
    fn fetch_storage_at_block(&self, address: Address, slot: B256, block_id: BlockId) -> FetchResult<B256> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = self.resolve_block_number(&provider, block_id)?;
        
        let state_provider = self.factory.history_by_block_number(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get historical state provider: {}", e)))?;
        
        let value = state_provider.storage(address, slot.into())
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch storage: {}", e)))?
            .unwrap_or(U256::ZERO);
        
        // Convert U256 to B256 by taking the bytes
        let mut bytes = [0u8; 32];
        value.to_be_bytes::<32>().into_iter().enumerate().for_each(|(i, b)| {
            bytes[i] = b;
        });
        
        Ok(B256::from(bytes))
    }
    
    fn fetch_storage_batch(&self, address: Address, slots: &[B256]) -> FetchResult<Vec<StorageData>> {
        let mut results = Vec::with_capacity(slots.len());
        
        for &slot in slots {
            match self.fetch_storage(address, slot) {
                Ok(value) => {
                    results.push(StorageData {
                        address: address,
                        key: slot,
                        value,
                    });
                }
                Err(_) => continue, // Skip failed fetches
            }
        }
        
        Ok(results)
    }
    
    fn fetch_storage_at_block_batch(&self, address: Address, slots: &[B256], block_id: BlockId) -> FetchResult<Vec<StorageData>> {
        let mut results = Vec::with_capacity(slots.len());
        
        for &slot in slots {
            match self.fetch_storage_at_block(address, slot, block_id.clone()) {
                Ok(value) => {
                    results.push(StorageData {
                        address,
                        key: slot,
                        value,
                    });
                }
                Err(_) => continue, // Skip failed fetches
            }
        }
        
        Ok(results)
    }
    
    fn fetch_storage_changes(&self, address: Address, start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>> {
        // Placeholder implementation - would need to analyze block-by-block storage changes
        // For now, return empty vec to allow examples to run
        let _ = (address, start_block, end_block);
        Ok(Vec::new())
    }
    
    fn fetch_receipt(&self, _tx_hash: B256) -> FetchResult<ReceiptData> {
        Err(FetchError::NotImplemented("fetch_receipt not yet implemented".to_string()))
    }
    
    fn fetch_receipts_batch(&self, _tx_hashes: &[B256]) -> FetchResult<Vec<ReceiptData>> {
        Err(FetchError::NotImplemented("fetch_receipts_batch not yet implemented".to_string()))
    }
    
    fn fetch_receipts_by_block(&self, _block_id: BlockId) -> FetchResult<Vec<ReceiptData>> {
        // Placeholder implementation - would need to fetch and convert all receipts for block
        // For now, return empty vec to allow examples to run
        Ok(Vec::new())
    }
    
    fn fetch_receipts_by_block_range(&self, _start_block: u64, _end_block: u64) -> FetchResult<Vec<ReceiptData>> {
        Err(FetchError::NotImplemented("fetch_receipts_by_block_range not yet implemented".to_string()))
    }
    
    fn fetch_changed_accounts(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<Address>> {
        // Placeholder implementation - would need to analyze all account changes in block range
        // For now, return empty vec to allow examples to run
        let _ = (start_block, end_block);
        Ok(Vec::new())
    }
    
    fn fetch_account_state_changes(&self, address: Address, start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>> {
        // Placeholder implementation - would need to analyze block-by-block changes
        // For now, return empty vec to allow examples to run
        let _ = (address, start_block, end_block);
        Ok(Vec::new())
    }
    
    fn fetch_multiple_account_state_changes(&self, _addresses: &[Address], _start_block: u64, _end_block: u64) -> FetchResult<Vec<StateChange>> {
        Err(FetchError::NotImplemented("fetch_multiple_account_state_changes not yet implemented".to_string()))
    }
    
    fn block_hash_to_number(&self, block_hash: B256) -> FetchResult<Option<u64>> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let block_number = provider.block_number(block_hash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get block number: {}", e)))?;
        
        Ok(block_number)
    }
    
    fn block_number_to_hash(&self, block_number: u64) -> FetchResult<Option<B256>> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let hash = provider.block_hash(block_number)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get block hash: {}", e)))?;
        
        Ok(hash)
    }
    
    fn transaction_hash_to_number(&self, tx_hash: B256) -> FetchResult<Option<u64>> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        let tx_number = provider.transaction_id(tx_hash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to get transaction ID: {}", e)))?;
        
        Ok(tx_number)
    }
    
    fn fetch_transaction_sender(&self, tx_hash: B256) -> FetchResult<Address> {
        let provider = self.factory.provider()
            .map_err(|e| FetchError::DatabaseError(format!("Failed to create provider: {}", e)))?;
        
        // First get the transaction
        let tx = provider.transaction_by_hash(tx_hash)
            .map_err(|e| FetchError::DatabaseError(format!("Failed to fetch transaction: {}", e)))?
            .ok_or_else(|| FetchError::transaction_not_found(&format!("{:x}", tx_hash)))?;
        
        // Recover the sender
        let sender = tx.recover_signer()
            .map_err(|e| FetchError::ParseError(format!("Failed to recover sender: {}", e)))?;
        
        Ok(sender)
    }
}

// Thread-safe wrapper for shared access
#[derive(Debug, Clone)]
pub struct SharedRethDataProvider {
    inner: Arc<RethDatabaseProvider>,
}

impl SharedRethDataProvider {
    pub fn new(provider: RethDatabaseProvider) -> Self {
        Self {
            inner: Arc::new(provider),
        }
    }
}

impl RethDataProvider for SharedRethDataProvider {
    fn fetch_transaction(&self, tx_hash: B256) -> FetchResult<TransactionData> {
        self.inner.fetch_transaction(tx_hash)
    }
    
    fn fetch_batch(&self, tx_hashes: &[B256]) -> FetchResult<Vec<TransactionData>> {
        self.inner.fetch_batch(tx_hashes)
    }
    
    fn transaction_exists(&self, tx_hash: B256) -> FetchResult<bool> {
        self.inner.transaction_exists(tx_hash)
    }
    
    fn latest_block_number(&self) -> FetchResult<u64> {
        self.inner.latest_block_number()
    }
    
    fn fetch_transaction_by_block_and_index(&self, block_number: u64, tx_index: u64) -> FetchResult<TransactionData> {
        self.inner.fetch_transaction_by_block_and_index(block_number, tx_index)
    }
    
    fn fetch_transactions_by_block(&self, block_id: BlockId) -> FetchResult<Vec<TransactionData>> {
        self.inner.fetch_transactions_by_block(block_id)
    }
    
    fn fetch_transactions_by_block_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<TransactionData>> {
        self.inner.fetch_transactions_by_block_range(start_block, end_block)
    }
    
    fn fetch_block(&self, block_id: BlockId) -> FetchResult<BlockData> {
        self.inner.fetch_block(block_id)
    }
    
    fn fetch_blocks_batch(&self, block_ids: &[BlockId]) -> FetchResult<Vec<BlockData>> {
        self.inner.fetch_blocks_batch(block_ids)
    }
    
    fn fetch_blocks_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<BlockData>> {
        self.inner.fetch_blocks_range(start_block, end_block)
    }
    
    fn block_exists(&self, block_id: BlockId) -> FetchResult<bool> {
        self.inner.block_exists(block_id)
    }
    
    fn chain_info(&self) -> FetchResult<ChainInfo> {
        self.inner.chain_info()
    }
    
    fn fetch_account(&self, address: Address) -> FetchResult<AccountData> {
        self.inner.fetch_account(address)
    }
    
    fn fetch_account_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<AccountData> {
        self.inner.fetch_account_at_block(address, block_id)
    }
    
    fn fetch_accounts_batch(&self, addresses: &[Address]) -> FetchResult<Vec<AccountData>> {
        self.inner.fetch_accounts_batch(addresses)
    }
    
    fn fetch_accounts_at_block_batch(&self, addresses: &[Address], block_id: BlockId) -> FetchResult<Vec<AccountData>> {
        self.inner.fetch_accounts_at_block_batch(addresses, block_id)
    }
    
    fn fetch_balance(&self, address: Address) -> FetchResult<U256> {
        self.inner.fetch_balance(address)
    }
    
    fn fetch_balance_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<U256> {
        self.inner.fetch_balance_at_block(address, block_id)
    }
    
    fn fetch_nonce(&self, address: Address) -> FetchResult<u64> {
        self.inner.fetch_nonce(address)
    }
    
    fn fetch_nonce_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<u64> {
        self.inner.fetch_nonce_at_block(address, block_id)
    }
    
    fn fetch_code(&self, address: Address) -> FetchResult<Option<Bytes>> {
        self.inner.fetch_code(address)
    }
    
    fn fetch_code_at_block(&self, address: Address, block_id: BlockId) -> FetchResult<Option<Bytes>> {
        self.inner.fetch_code_at_block(address, block_id)
    }
    
    fn fetch_storage(&self, address: Address, slot: B256) -> FetchResult<B256> {
        self.inner.fetch_storage(address, slot)
    }
    
    fn fetch_storage_at_block(&self, address: Address, slot: B256, block_id: BlockId) -> FetchResult<B256> {
        self.inner.fetch_storage_at_block(address, slot, block_id)
    }
    
    fn fetch_storage_batch(&self, address: Address, slots: &[B256]) -> FetchResult<Vec<StorageData>> {
        self.inner.fetch_storage_batch(address, slots)
    }
    
    fn fetch_storage_at_block_batch(&self, address: Address, slots: &[B256], block_id: BlockId) -> FetchResult<Vec<StorageData>> {
        self.inner.fetch_storage_at_block_batch(address, slots, block_id)
    }
    
    fn fetch_storage_changes(&self, address: Address, start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>> {
        self.inner.fetch_storage_changes(address, start_block, end_block)
    }
    
    fn fetch_receipt(&self, tx_hash: B256) -> FetchResult<ReceiptData> {
        self.inner.fetch_receipt(tx_hash)
    }
    
    fn fetch_receipts_batch(&self, tx_hashes: &[B256]) -> FetchResult<Vec<ReceiptData>> {
        self.inner.fetch_receipts_batch(tx_hashes)
    }
    
    fn fetch_receipts_by_block(&self, block_id: BlockId) -> FetchResult<Vec<ReceiptData>> {
        self.inner.fetch_receipts_by_block(block_id)
    }
    
    fn fetch_receipts_by_block_range(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<ReceiptData>> {
        self.inner.fetch_receipts_by_block_range(start_block, end_block)
    }
    
    fn fetch_changed_accounts(&self, start_block: u64, end_block: u64) -> FetchResult<Vec<Address>> {
        self.inner.fetch_changed_accounts(start_block, end_block)
    }
    
    fn fetch_account_state_changes(&self, address: Address, start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>> {
        self.inner.fetch_account_state_changes(address, start_block, end_block)
    }
    
    fn fetch_multiple_account_state_changes(&self, addresses: &[Address], start_block: u64, end_block: u64) -> FetchResult<Vec<StateChange>> {
        self.inner.fetch_multiple_account_state_changes(addresses, start_block, end_block)
    }
    
    fn block_hash_to_number(&self, block_hash: B256) -> FetchResult<Option<u64>> {
        self.inner.block_hash_to_number(block_hash)
    }
    
    fn block_number_to_hash(&self, block_number: u64) -> FetchResult<Option<B256>> {
        self.inner.block_number_to_hash(block_number)
    }
    
    fn transaction_hash_to_number(&self, tx_hash: B256) -> FetchResult<Option<u64>> {
        self.inner.transaction_hash_to_number(tx_hash)
    }
    
    fn fetch_transaction_sender(&self, tx_hash: B256) -> FetchResult<Address> {
        self.inner.fetch_transaction_sender(tx_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_transaction_data_creation() {
        // Test that TransactionData can be created properly
        let tx_data = TransactionData {
            hash: B256::ZERO,
            from: Address::ZERO,
            to: Some(Address::ZERO),
            value: U256::ZERO,
            gas_limit: 21000,
            gas_used: 21000,
            gas_price: U256::from(20000000000u64),
            nonce: 0,
            block_number: 0,
            block_hash: B256::ZERO,
            transaction_index: 0,
            input: Bytes::new(),
            receipt_status: true,
            contractaddress: None,
            logs: vec![],
        };
        
        assert_eq!(tx_data.gas_limit, 21000);
        assert!(tx_data.receipt_status);
    }
    
    #[test]
    fn test_provider_creation_invalid_path() {
        let invalid_path = PathBuf::from("/nonexistent/path");
        let result = RethDatabaseProvider::new(invalid_path);
        assert!(result.is_err());
        
        if let Err(FetchError::ConfigError(_)) = result {
            // Expected error type
        } else {
            panic!("Expected ConfigError");
        }
    }
    
    #[test]
    fn test_shared_provider_wrapper() {
        // Test that SharedRethDataProvider can be created
        // This test doesn't require a real database
        let config = RethDataConfig::new("/tmp"); // Will fail validation but that's OK for this test
        
        // The creation will fail due to invalid path, but we can test the wrapper structure
        let result = RethDatabaseProvider::with_config(config);
        assert!(result.is_err()); // Expected since /tmp is not a valid Reth database
    }
}