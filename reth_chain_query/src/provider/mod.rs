use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use reth_chainspec::{ChainSpec, ChainSpecProvider, EthChainSpec};
use reth_db::DatabaseEnv;
use reth_ethereum_engine_primitives::EthEngineTypes;
use reth_ethereum_primitives::EthPrimitives;
use reth_node_types::{AnyNodeTypes, NodeTypesWithDBAdapter};
use reth_provider::BlockReader;
use reth_provider::{EthStorage, ProviderFactory};
/// RethQueryProvider - Central provider for all blockchain queries
///
/// This provider wraps Reth's database access and provides a unified interface
/// for all query operations. It manages shared resources like database connections,
/// caches, and the TxSimulator instance.
use std::sync::Arc;
use tx_simulator::TxSimulator;

// Import our modules
use crate::reth_index::RethIndexDB;

type EthereumProviderTypes = AnyNodeTypes<EthPrimitives, ChainSpec, EthStorage, EthEngineTypes>;
pub type RethProviderFactory =
    ProviderFactory<NodeTypesWithDBAdapter<EthereumProviderTypes, Arc<DatabaseEnv>>>;

// Re-export submodules
mod address_index;
mod address_state;
mod batch_ops;
pub mod block;
mod caching;
mod gas;
mod transactions;
pub mod types;

pub use crate::contracts::erc20::*;
pub use address_state::*;
pub use block::{types::*, BlockDataFetcher, RpcBlockDataFetcher};
pub use caching::*;
pub use types::*;

/// Central provider for all Reth database queries
pub struct RethQueryProvider {
    /// Core TxSimulator for database access and view function simulation
    tx_simulator: Arc<TxSimulator>,

    /// Provider factory for direct database access
    provider_factory: Arc<RethProviderFactory>,

    /// Optional RPC provider for trace data (temporary until local tracing)
    rpc_provider: Option<Arc<dyn std::any::Any + Send + Sync>>,

    /// Optional RethIndex database for fast entity-centric queries
    reth_index: Option<Arc<RethIndexDB>>,
}

impl RethQueryProvider {
    /// Create a new RethQueryProvider with the given Reth data directory
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let tx_simulator = Arc::new(TxSimulator::new(reth_datadir)?);
        let provider_factory = Arc::new(tx_simulator.provider_factory().clone());

        Ok(Self {
            tx_simulator,
            provider_factory,
            rpc_provider: None,
            reth_index: None,
        })
    }

    /// Create with an existing TxSimulator (for resource sharing)
    pub fn with_simulator(simulator: Arc<TxSimulator>) -> Result<Self> {
        let provider_factory = Arc::new(simulator.provider_factory().clone());

        Ok(Self {
            tx_simulator: simulator,
            provider_factory,
            rpc_provider: None,
            reth_index: None,
        })
    }

    /// Create with an existing provider factory
    pub fn with_provider_factory(provider_factory: Arc<RethProviderFactory>) -> Result<Self> {
        let simulator = Arc::new(TxSimulator::with_provider_factory(
            (*provider_factory).clone(),
        )?);

        Ok(Self {
            tx_simulator: simulator,
            provider_factory,
            rpc_provider: None,
            reth_index: None,
        })
    }

    // RPC-based tracing intentionally omitted for direct DB-only usage.

    /// Enable RethIndex for fast entity-centric queries
    pub fn with_reth_index(mut self, reth_index_path: &str) -> Result<Self> {
        self.reth_index = Some(Arc::new(RethIndexDB::open_read_only(reth_index_path)?));
        Ok(self)
    }

    /// Attach an existing RethIndex handle (useful when sharing within the same process).
    pub fn with_reth_index_db(mut self, reth_index_db: Arc<RethIndexDB>) -> Self {
        self.reth_index = Some(reth_index_db);
        self
    }

    /// Get the underlying TxSimulator for advanced operations
    pub fn simulator(&self) -> &Arc<TxSimulator> {
        &self.tx_simulator
    }

    /// Refresh the read-only static-file view after the live Reth node has advanced.
    pub fn refresh_static_file_provider(&self) -> Result<()> {
        self.provider_factory.caught_up_static_file_provider()?;
        Ok(())
    }

    /// Return the configured chain id.
    pub fn chain_id(&self) -> u64 {
        self.provider_factory.chain_spec().chain_id()
    }

    /// Get the provider factory for direct database access
    pub fn provider_factory(&self) -> &Arc<RethProviderFactory> {
        &self.provider_factory
    }

    /// Get the RethIndex database if available
    pub fn reth_index(&self) -> Option<&Arc<RethIndexDB>> {
        self.reth_index.as_ref()
    }

    /// Get the latest block number
    pub fn get_latest_block(&self) -> Result<u64> {
        self.tx_simulator.get_latest_block()
    }

    /// Get block timestamp for a specific block number
    pub fn get_block_timestamp(&self, block_number: u64) -> Result<u64> {
        let block = self
            .provider_factory
            .block_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("Invalid block number {}", block_number))?;
        Ok(block.timestamp)
    }

    // === Internal Helper Methods ===

    /// Get account from PlainAccountState table (internal helper)
    async fn get_account_internal(&self, address: Address, block: Option<u64>) -> Result<Account> {
        let block = block.unwrap_or(self.get_latest_block()?);

        // Get state provider for this block
        let state = self.tx_simulator.get_chain_state_at_block(block)?;

        // Get account info directly from PlainAccountState
        let account_opt = state.basic_account(&address)?;

        match account_opt {
            Some(account_info) => {
                // Check if it's a contract by looking for code
                let code_hash = if account_info.has_bytecode() {
                    Some(account_info.bytecode_hash.unwrap_or(B256::ZERO))
                } else {
                    None
                };

                Ok(Account {
                    nonce: account_info.nonce,
                    balance: account_info.balance,
                    code_hash,
                })
            }
            None => {
                // Account doesn't exist - return zero values
                Ok(Account {
                    nonce: 0,
                    balance: U256::ZERO,
                    code_hash: None,
                })
            }
        }
    }

    /// Get storage value from PlainStorageState table (internal helper)
    async fn get_storage_internal(
        &self,
        contract: Address,
        slot: B256,
        block: Option<u64>,
    ) -> Result<U256> {
        let block = block.unwrap_or(self.get_latest_block()?);

        // Get state provider for this block
        let state = self.tx_simulator.get_chain_state_at_block(block)?;

        // Read storage directly from PlainStorageState
        let value = state.storage(contract, slot.into())?;

        Ok(value.unwrap_or(U256::ZERO))
    }

    // === Core Query Methods ===

    /// Get account information (balance, nonce, code_hash)
    /// Uses PlainAccountState table
    pub async fn get_account(&self, address: Address, block: Option<u64>) -> Result<Account> {
        self.get_account_internal(address, block).await
    }

    /// Get storage value at specific slot
    /// Uses PlainStorageState table
    pub async fn get_storage(
        &self,
        contract: Address,
        slot: B256,
        block: Option<u64>,
    ) -> Result<U256> {
        self.get_storage_internal(contract, slot, block).await
    }

    /// Get token balance using view function
    /// Executes balanceOf(address) on the token contract
    pub async fn get_token_balance(
        &self,
        token: Address,
        holder: Address,
        block: Option<u64>,
    ) -> Result<U256> {
        self.get_token_balance_internal(token, holder, block).await
    }

    /// Get complete portfolio for an address
    /// Combines ETH balance with token balances
    pub async fn get_portfolio(
        &self,
        address: Address,
        tokens: Vec<Address>,
        block: Option<u64>,
    ) -> Result<Portfolio> {
        let eth_balance = self.get_account(address, block).await?.balance;

        let mut token_balances = std::collections::HashMap::new();
        for token in tokens {
            let balance = self.get_token_balance(token, address, block).await?;
            if balance > U256::ZERO {
                token_balances.insert(token, balance);
            }
        }

        Ok(Portfolio {
            address,
            eth_balance,
            token_balances,
            block_number: block.unwrap_or(self.get_latest_block()?),
        })
    }

    // === RethIndex Methods (when available) ===

    /// Get all indexed processed block numbers for an address (requires RethIndex).
    pub fn get_address_participation_blocks(&self, address: Address) -> Result<Vec<u64>> {
        if let Some(reth_index) = &self.reth_index {
            reth_index.get_address_participation_blocks(address)
        } else {
            Err(eyre::eyre!(
                "RethIndex not available. Enable with with_reth_index()"
            ))
        }
    }

    /// Get aggregated metrics for an address (requires RethIndex)
    pub fn get_address_metrics(
        &self,
        _address: Address,
    ) -> Result<crate::reth_index::AddressMetrics> {
        if let Some(_reth_index) = &self.reth_index {
            // TODO: Implement get_address_metrics in RethIndexDB
            Err(eyre::eyre!("get_address_metrics not yet implemented"))
        } else {
            Err(eyre::eyre!(
                "RethIndex not available. Enable with with_reth_index()"
            ))
        }
    }
}

/// Convenience: build and return a shared ProviderFactory (Arc) from a Reth datadir.
///
/// This uses RethQueryProvider::new under the hood to ensure we initialize the
/// TxSimulator and internal caches in a consistent way, then returns a clone of
/// the underlying ProviderFactory for components that only need the factory.
pub fn provider_factory_from_datadir(reth_datadir: &str) -> Result<Arc<RethProviderFactory>> {
    let rqp = RethQueryProvider::new(reth_datadir)?;
    Ok(rqp.provider_factory().clone())
}

/// Account information from PlainAccountState
#[derive(Debug, Clone)]
pub struct Account {
    pub nonce: u64,
    pub balance: U256,
    pub code_hash: Option<B256>,
}

/// Portfolio information combining ETH and token balances
#[derive(Debug, Clone)]
pub struct Portfolio {
    pub address: Address,
    pub eth_balance: U256,
    pub token_balances: std::collections::HashMap<Address, U256>,
    pub block_number: u64,
}
