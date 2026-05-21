use crate::block_context::{BlockContext, BlockContextLoader};
use crate::types::SimulationDefaults;
use eyre::{eyre, Report, Result};
use std::future::Future;
use std::path::Path;
/// Core transaction simulator implementation
///
/// This module contains the main TxSimulator struct and its core methods
/// for initialization and database access.
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;
use tokio::{runtime::Runtime, task};

// Core Reth imports
use reth_chainspec::{ChainSpec, ChainSpecProvider, MAINNET};
use reth_db::{mdbx::DatabaseArguments, open_db_read_only, ClientVersion, DatabaseEnv};
use reth_ethereum_engine_primitives::EthEngineTypes;
use reth_ethereum_primitives::EthPrimitives;
use reth_evm_ethereum::EthEvmConfig;
use reth_node_types::{AnyNodeTypes, NodeTypesWithDBAdapter};
use reth_primitives_traits::SealedHeader;
use reth_provider::{
    providers::{RocksDBProvider, StaticFileProvider},
    BlockNumReader, EthStorage, ProviderFactory, StateProviderBox,
};

const DB_OPEN_RETRY_ATTEMPTS: usize = 50;
const DB_OPEN_RETRY_DELAY: Duration = Duration::from_millis(100);

type EthereumProviderTypes = AnyNodeTypes<EthPrimitives, ChainSpec, EthStorage, EthEngineTypes>;
pub(crate) type EthereumProviderFactory =
    ProviderFactory<NodeTypesWithDBAdapter<EthereumProviderTypes, Arc<DatabaseEnv>>>;

/// Transaction Simulator with direct database access
#[derive(Clone)]
pub struct TxSimulator {
    pub(crate) provider_factory: EthereumProviderFactory,
    pub(crate) evm_config: EthEvmConfig,
    pub(crate) defaults: SimulationDefaults,
}

impl TxSimulator {
    /// Create new simulator with direct database access
    ///
    /// # Arguments
    /// * `reth_datadir` - Path to Reth data directory (e.g., `/home/nima/storage/samsung8tb/ethereum/reth`)
    ///
    /// # Returns
    /// * `Result<Self>` - New simulator instance or error
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let datadir = Path::new(reth_datadir);
        let db_path = datadir.join("db");
        let static_files_path = datadir.join("static_files");

        // Open database read-only
        let db = Arc::new(open_db_read_only_with_retry(db_path.as_path())?);

        let chain_spec = MAINNET.clone();

        let provider_factory = EthereumProviderFactory::new(
            db.clone(),
            chain_spec.clone(),
            StaticFileProvider::read_only(static_files_path)?,
            RocksDBProvider::builder(datadir.join("rocksdb"))
                .with_default_tables()
                .with_read_only(true)
                .build()?,
            reth_tasks::Runtime::test(),
        )?
        .with_read_only_sync(true);

        let evm_config = EthEvmConfig::new(chain_spec.clone());

        let simulator = Self {
            provider_factory,
            evm_config,
            defaults: SimulationDefaults::default(),
        };
        Ok(simulator)
    }

    /// Create new simulator with an existing provider factory
    ///
    /// This is useful when you want to share a database connection across multiple components
    pub fn with_provider_factory(provider_factory: EthereumProviderFactory) -> Result<Self> {
        let chain_spec = provider_factory.chain_spec();
        let evm_config = EthEvmConfig::new(chain_spec);

        let simulator = Self {
            provider_factory,
            evm_config,
            defaults: SimulationDefaults::default(),
        };
        Ok(simulator)
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

    /// Get the latest block number reported by Reth's Finish stage.
    ///
    /// This is raw node progress. During live runs the Finish checkpoint can be
    /// one block ahead of the read-only static-file header view, so callers that
    /// need to build a full simulation context should prefer
    /// [`TxSimulator::latest_historical_context_block_number`].
    pub fn get_latest_block(&self) -> Result<u64> {
        self.refresh_static_file_provider()?;
        let provider = self.provider_factory.provider()?;
        let block_number = provider.best_block_number()?;
        Ok(block_number)
    }

    /// Latest block whose header is readable from Reth static files.
    pub fn latest_static_header_block_number(&self) -> Result<u64> {
        self.refresh_static_file_provider()?;
        Ok(self.provider_factory.last_block_number()?)
    }

    /// Latest block for which the simulator can build context directly from
    /// local Reth providers.
    ///
    /// Historical simulation needs both executed state and the block header. In
    /// live mode those two views do not always advance at the same instant:
    /// `best_block_number()` follows the Finish stage, while headers are read
    /// through static files. The minimum is the highest DB-backed block that is
    /// safe to select as local historical context.
    pub fn latest_historical_context_block_number(&self) -> Result<u64> {
        let latest_reth_finished = self.get_latest_block()?;
        let latest_static_header = self.latest_static_header_block_number()?;
        Ok(latest_reth_finished.min(latest_static_header))
    }

    /// Get the provider factory for direct database access
    pub fn provider_factory(&self) -> &EthereumProviderFactory {
        &self.provider_factory
    }

    /// Refresh the read-only static-file view after the live Reth node advances.
    pub fn refresh_static_file_provider(&self) -> Result<()> {
        self.provider_factory.caught_up_static_file_provider()?;
        Ok(())
    }

    /// Ensure the requested block can be used as local historical simulation context.
    pub fn assert_block_available(&self, block_number: u64) -> Result<()> {
        let latest = self.latest_historical_context_block_number()?;
        if block_number > latest {
            return Err(eyre::eyre!(
                "State for block {} not yet available as local historical context (latest historical context block {})",
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
    pub fn get_chain_state_at_block(&self, block_number: u64) -> Result<StateProviderBox> {
        let context = self.load_block_context_blocking(block_number, None)?;
        Ok(context.state)
    }

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
            loader_runtime().block_on(fut)
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
}

// Alias for compatibility
pub type RethTxSimulator = TxSimulator;

fn open_db_read_only_with_retry(db_path: &Path) -> Result<DatabaseEnv> {
    let mut last_error = None;
    for attempt in 0..DB_OPEN_RETRY_ATTEMPTS {
        match open_db_read_only(db_path, DatabaseArguments::new(ClientVersion::default())) {
            Ok(db) => return Ok(db),
            Err(error) if is_transient_mdbx_open_error(&error) => {
                last_error = Some(error);
                if attempt + 1 < DB_OPEN_RETRY_ATTEMPTS {
                    thread::sleep(DB_OPEN_RETRY_DELAY);
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(last_error
        .expect("retry loop must store the final transient database-open error")
        .into())
}

fn is_transient_mdbx_open_error(error: &Report) -> bool {
    error
        .chain()
        .any(|cause| is_transient_mdbx_open_error_text(&cause.to_string()))
        || is_transient_mdbx_open_error_text(&format!("{error:?}"))
}

fn is_transient_mdbx_open_error_text(message: &str) -> bool {
    message.contains("another write transaction is running")
}

fn loader_runtime() -> &'static Runtime {
    static LOADER_RUNTIME: OnceLock<Runtime> = OnceLock::new();
    LOADER_RUNTIME.get_or_init(|| {
        Runtime::new().expect("failed to create tx_simulator block-context loader runtime")
    })
}

#[cfg(test)]
mod tests {
    use super::is_transient_mdbx_open_error_text;

    #[test]
    fn detects_transient_mdbx_writer_open_error() {
        assert!(is_transient_mdbx_open_error_text(
            "failed to open the database: another write transaction is running (-30778)"
        ));
        assert!(!is_transient_mdbx_open_error_text("permission denied"));
    }
}
