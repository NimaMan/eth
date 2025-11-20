use crate::block_context::{self, BlockContext, BlockContextLoader, BlockStateProvider};
use crate::live_chain_cache::LiveChainCache;
use crate::types::SimulationDefaults;
use eyre::{eyre, Result};
use std::future::Future;
use std::path::Path;
/// Core transaction simulator implementation
///
/// This module contains the main TxSimulator struct and its core methods
/// for initialization and database access.
use std::sync::Arc;
use tokio::{runtime::Runtime, task};
use tracing::warn;

// Core Reth imports
use reth_chainspec::{ChainSpecBuilder, ChainSpecProvider};
use reth_db::{mdbx::DatabaseArguments, open_db_read_only, ClientVersion, DatabaseEnv};
use reth_node_ethereum::{EthEvmConfig, EthereumNode};
use reth_node_types::NodeTypesWithDBAdapter;
use reth_primitives::SealedHeader;
use reth_provider::{
    providers::StaticFileProvider, BlockNumReader, ProviderFactory, StateProvider,
};

/// Transaction Simulator with direct database access
#[derive(Clone)]
pub struct TxSimulator {
    pub(crate) provider_factory:
        ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    pub(crate) evm_config: EthEvmConfig,
    pub(crate) defaults: SimulationDefaults,
    pub(crate) live_chain_cache: Option<Arc<LiveChainCache>>,
    loader_runtime: Arc<Runtime>,
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

        let loader_runtime = Arc::new(
            Runtime::new().map_err(|err| eyre!("failed to create loader runtime: {}", err))?,
        );

        let mut simulator = Self {
            provider_factory,
            evm_config,
            defaults: SimulationDefaults::default(),
            live_chain_cache: None,
            loader_runtime,
        };
        simulator.attach_default_live_chain_cache();
        Ok(simulator)
    }

    /// Create new simulator with an existing provider factory
    ///
    /// This is useful when you want to share a database connection across multiple components
    pub fn with_provider_factory(
        provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    ) -> Result<Self> {
        let chain_spec = provider_factory.chain_spec();
        let evm_config = EthEvmConfig::new(chain_spec);

        let loader_runtime = Arc::new(
            Runtime::new().map_err(|err| eyre!("failed to create loader runtime: {}", err))?,
        );

        let mut simulator = Self {
            provider_factory,
            evm_config,
            defaults: SimulationDefaults::default(),
            live_chain_cache: None,
            loader_runtime,
        };
        simulator.attach_default_live_chain_cache();
        Ok(simulator)
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
        let context = self.load_block_context_blocking(block_number, None)?;
        let base_fee = context.header.base_fee_per_gas.ok_or_else(|| {
            eyre!(
                "No base fee for block {} (likely pre-London or header missing base fee)",
                block_number
            )
        })?;
        Ok(base_fee as u128)
    }

    /// Get chain state at a specific block
    /// Returns a StateProvider that gives access to all blockchain state at that block
    pub fn get_chain_state_at_block(&self, block_number: u64) -> Result<Box<dyn StateProvider>> {
        let context = self.load_block_context_blocking(block_number, None)?;
        match context.state {
            BlockStateProvider::Historical(state) => Ok(state),
            BlockStateProvider::LiveFork(_) => Err(eyre!(
                "State for block {} only exists in the live cache; use simulation APIs instead",
                block_number
            )),
        }
    }

    // === Block Simulation Methods (temporarily disabled) ===

    // Block simulation methods are temporarily disabled until the module is fixed
    // with proper revm imports and SystemCaller implementation.
    // These will be re-enabled once compilation issues are resolved.

    /// Get block metadata (timestamp, gas_limit, gas_used, base_fee)
    pub fn get_block_metadata(&self, block_number: u64) -> Result<(u64, u64, u64, Option<u128>)> {
        let context = self.load_block_context_blocking(block_number, None)?;
        Ok((
            context.header.timestamp,
            context.header.gas_limit,
            context.header.gas_used,
            context.header.base_fee_per_gas.map(|v| v as u128),
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

    /// Helper accessor for the shared chain data loader.
    pub fn block_context_loader(&self) -> BlockContextLoader<'_> {
        BlockContextLoader::new(self)
    }

    /// Execute a block-context future on either the caller's runtime (when safe) or the
    /// simulator's dedicated loader runtime.
    fn run_block_context_future<F>(&self, fut: F) -> F::Output
    where
        F: Future,
    {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            task::block_in_place(|| handle.block_on(fut))
        } else {
            self.loader_runtime.block_on(fut)
        }
    }

    pub(crate) fn load_block_context_blocking(
        &self,
        block_number: u64,
        header_hint: Option<SealedHeader>,
    ) -> Result<BlockContext> {
        self.run_block_context_future(
            self.block_context_loader()
                .load_block_context(block_number, header_hint),
        )
    }

    fn attach_default_live_chain_cache(&mut self) {
        if self.live_chain_cache.is_some() {
            return;
        }

        let redis_url = block_context::resolve_live_data_redis_url();
        match LiveChainCache::new(&redis_url) {
            Ok(cache) => {
                self.live_chain_cache = Some(Arc::new(cache));
            }
            Err(err) => {
                warn!(
                    "failed to initialize live chain cache at {}: {}",
                    redis_url, err
                );
            }
        }
    }
}

// Alias for compatibility
pub type RethTxSimulator = TxSimulator;
