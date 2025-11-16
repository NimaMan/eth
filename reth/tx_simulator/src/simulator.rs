use crate::header_utils::parse_sealed_header_from_json;
use crate::live_chain_cache::LiveChainCache;
use crate::single_tx::unsigned::UnsignedTransaction;
use crate::tx_builders::unsigned_tx_builder::build_unsigned_transaction_from_processed_tx_json;
use crate::tx_chain::sequential::ForkedState;
use crate::types::SimulationDefaults;
use eyre::{eyre, Result};
use std::path::Path;
/// Core transaction simulator implementation
///
/// This module contains the main TxSimulator struct and its core methods
/// for initialization and database access.
use std::sync::Arc;

// Core Reth imports
use reth_chainspec::{ChainSpecBuilder, ChainSpecProvider};
use reth_db::{mdbx::DatabaseArguments, open_db_read_only, ClientVersion, DatabaseEnv};
use reth_node_ethereum::{EthEvmConfig, EthereumNode};
use reth_node_types::NodeTypesWithDBAdapter;
use reth_provider::{
    providers::StaticFileProvider, BlockNumReader, HeaderProvider, ProviderFactory, StateProvider,
};
use serde_json::Value;

/// Transaction Simulator with direct database access
#[derive(Clone)]
pub struct TxSimulator {
    pub(crate) provider_factory:
        ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    pub(crate) evm_config: EthEvmConfig,
    pub(crate) defaults: SimulationDefaults,
    pub(crate) live_chain_cache: Option<Arc<LiveChainCache>>,
}

impl TxSimulator {
    /// Create new simulator with direct database access
    ///
    /// # Arguments
    /// * `reth_datadir` - Path to Reth data directory (e.g., `/home/user/.local/share/reth/mainnet`)
    ///
    /// # Returns
    /// * `Result<Self>` - New simulator instance or error
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let datadir = Path::new(reth_datadir);
        let db_path = datadir.join("db");
        let static_files_path = datadir.join("static_files");

        // Open database read-only
        let db = Arc::new(open_db_read_only(
            db_path.as_path(),
            DatabaseArguments::new(ClientVersion::default()),
        )?);

        let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());

        let provider_factory =
            ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
                db.clone(),
                chain_spec.clone(),
                StaticFileProvider::read_only(static_files_path, false)?,
            );

        let evm_config = EthEvmConfig::new(chain_spec.clone());

        Ok(Self {
            provider_factory,
            evm_config,
            defaults: SimulationDefaults::default(),
            live_chain_cache: None,
        })
    }

    /// Create new simulator with an existing provider factory
    ///
    /// This is useful when you want to share a database connection across multiple components
    pub fn with_provider_factory(
        provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    ) -> Result<Self> {
        let chain_spec = provider_factory.chain_spec();
        let evm_config = EthEvmConfig::new(chain_spec);

        Ok(Self {
            provider_factory,
            evm_config,
            defaults: SimulationDefaults::default(),
            live_chain_cache: None,
        })
    }

    pub fn with_live_chain_cache(mut self, cache: LiveChainCache) -> Self {
        self.live_chain_cache = Some(Arc::new(cache));
        self
    }

    /// Current simulator defaults (fees, view call behaviour)
    pub fn simulation_defaults(&self) -> &SimulationDefaults {
        &self.defaults
    }

    /// Mutable access to simulator defaults for configuration prior to use
    pub fn simulation_defaults_mut(&mut self) -> &mut SimulationDefaults {
        &mut self.defaults
    }

    /// Consume the simulator and replace defaults in a builder-style call
    pub fn with_simulation_defaults(mut self, defaults: SimulationDefaults) -> Self {
        self.defaults = defaults;
        self
    }

    /// Get latest block number from local database
    pub fn get_latest_block(&self) -> Result<u64> {
        let provider = self.provider_factory.provider()?;
        let block_number = provider.best_block_number()?;
        Ok(block_number)
    }

    /// Get the provider factory for direct database access
    pub fn provider_factory(
        &self,
    ) -> &ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>> {
        &self.provider_factory
    }

    /// Ensure the requested block is already persisted in the local database
    pub fn assert_block_available(&self, block_number: u64) -> Result<()> {
        let latest = self.get_latest_block()?;
        if block_number > latest {
            return Err(eyre::eyre!(
                "State for block {} not yet available (latest persisted block {})",
                block_number,
                latest
            ));
        }
        Ok(())
    }

    /// Get the base fee for a specific block
    ///
    /// This is needed for EIP-1559 transactions to ensure gas prices are set correctly.
    /// Returns the base fee in wei.
    pub fn get_base_fee_at_block(&self, block_number: u64) -> Result<u128> {
        self.assert_block_available(block_number)?;
        let provider = self.provider_factory.provider()?;
        let block_header = provider.header_by_number(block_number)?.ok_or_else(|| {
            eyre::eyre!(
                "No header for block whilst getting base-fee {}",
                block_number
            )
        })?;

        let base_fee = block_header
            .base_fee_per_gas
            .ok_or_else(|| eyre::eyre!("No base fee for block {} (pre-London?)", block_number))?;

        Ok(base_fee as u128)
    }

    /// Get chain state at a specific block
    /// Returns a StateProvider that gives access to all blockchain state at that block
    pub fn get_chain_state_at_block(&self, block_number: u64) -> Result<Box<dyn StateProvider>> {
        self.assert_block_available(block_number)?;
        let state = self
            .provider_factory
            .history_by_block_number(block_number)?;
        Ok(state)
    }

    // === Block Simulation Methods (temporarily disabled) ===

    // Block simulation methods are temporarily disabled until the module is fixed
    // with proper revm imports and SystemCaller implementation.
    // These will be re-enabled once compilation issues are resolved.

    /// Get block metadata (timestamp, gas_limit, gas_used, base_fee)
    pub fn get_block_metadata(&self, block_number: u64) -> Result<(u64, u64, u64, Option<u128>)> {
        self.assert_block_available(block_number)?;
        let provider = self.provider_factory.provider()?;
        let block_header = provider.header_by_number(block_number)?.ok_or_else(|| {
            eyre::eyre!(
                "No header for block whilst getting block metadata {}",
                block_number
            )
        })?;

        Ok((
            block_header.timestamp,
            block_header.gas_limit,
            block_header.gas_used,
            block_header.base_fee_per_gas.map(|v| v as u128),
        ))
    }
}

impl TxSimulator {
    pub fn live_chain_cache(&self) -> Option<Arc<LiveChainCache>> {
        self.live_chain_cache.clone()
    }

    /// Returns the latest block number advertised by the live cache, if any.
    pub async fn live_latest_block_number(&self) -> Result<Option<u64>> {
        let Some(cache) = self.live_chain_cache() else {
            return Ok(None);
        };
        cache.latest_block_number().await
    }

    pub(crate) async fn replay_block_from_live_data(
        &self,
        block_number: u64,
        header_hint: Option<reth_primitives::SealedHeader>,
    ) -> Result<Option<ForkedState>> {
        let Some(cache) = self.live_chain_cache() else {
            return Ok(None);
        };

        let Some(latest_live) = cache.latest_block_number().await? else {
            return Ok(None);
        };
        if block_number > latest_live {
            return Ok(None);
        }

        let header = match header_hint {
            Some(header) => header,
            None => {
                let header_json = cache
                    .fetch_block_header(block_number)
                    .await?
                    .ok_or_else(|| eyre!("missing live block header for {}", block_number))?;
                parse_sealed_header_from_json(&header_json)?
            }
        };

        let parent_block = header
            .number
            .checked_sub(1)
            .ok_or_else(|| eyre!("cannot replay state for genesis block"))?;

        self.assert_block_available(parent_block)?;

        let mut forked_state = self.create_forked_state(parent_block)?;
        forked_state.block_number = header.number;
        forked_state.block_header = header.clone();
        forked_state.nonces.clear();

        let Some(raw_txs) = cache.fetch_processed_block(block_number).await? else {
            return Ok(None);
        };
        let transactions = parse_snapshot_transactions(&raw_txs)?;
        if transactions.is_empty() {
            return Ok(Some(forked_state));
        }

        let simulator = Arc::new(self.clone());
        let mut chain =
            crate::tx_chain::unsigned::UnsignedTxChainSimulation::new(simulator, forked_state);
        for tx in transactions {
            chain.step(tx).await?;
        }

        Ok(Some(chain.into_forked_state()))
    }
}

fn parse_snapshot_transactions(values: &[Value]) -> Result<Vec<UnsignedTransaction>> {
    let mut txs = Vec::with_capacity(values.len());
    for value in values {
        txs.push(build_unsigned_transaction_from_processed_tx_json(value)?);
    }
    Ok(txs)
}

// Alias for compatibility
pub type RethTxSimulator = TxSimulator;
