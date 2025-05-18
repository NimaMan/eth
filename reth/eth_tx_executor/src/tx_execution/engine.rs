/*
* Transaction Execution Engine
*
* Main component that ties together all transaction execution functionality:
* - Transaction simulation
* - Transaction execution
* - Transaction monitoring
* - Status tracking
*
* Algorithm:
* 1. Receive transaction request
* 2. Simulate transaction to verify success and predict outcome
* 3. If simulation passes validation rules, execute transaction
* 4. Monitor transaction status until confirmation or failure
* 5. Report final status
*/

use crate::tx_execution::{
    types::{Transaction, TxStatus, SimulationResult},
    simulator::TxSimulator,
    monitor::TxMonitor,
    config::TxExecutionConfig,
    error::{TxError, TxResult},
};
use ethers::{
    providers::Middleware,
    signers::Signer,
    types::H256,
};
use std::sync::Arc;
use std::time::Duration;

/// Engine for coordinating transaction execution flow
pub struct TxExecutionEngine<S, M> 
where 
    S: Signer + Clone,
    M: Middleware + Clone
{
    /// Transaction simulator
    simulator: TxSimulator<M>,
    
    /// Transaction executor
    executor: crate::tx_execution::executor::TxExecutor<S, M>,
    
    /// Transaction monitor
    monitor: TxMonitor<M>,
    
    /// Configuration
    config: TxExecutionConfig,
}

impl<S, M> TxExecutionEngine<S, M> 
where 
    S: Signer + Clone,
    M: Middleware + Clone
{
    /// Create a new transaction execution engine
    pub fn new(provider: Arc<M>, signer: S, config: TxExecutionConfig) -> Self {
        let simulator = TxSimulator::new(provider.clone(), config.clone());
        let executor = crate::tx_execution::executor::TxExecutor::new(provider.clone(), signer, config.clone());
        let monitor = TxMonitor::new(provider.clone(), config.clone());
        
        Self {
            simulator,
            executor,
            monitor,
            config,
        }
    }
    
    /// Execute a transaction with simulation first
    pub async fn execute(&self, tx: &Transaction) -> TxResult<TxStatus> {
        // Step 1: Simulate the transaction
        let simulation = self.simulator.simulate(tx).await?;
        
        // Step 2: Validate the simulation results
        self.validate_simulation(&simulation)?;
        
        // Step 3: Execute the transaction
        let submission_status = self.executor.execute(tx).await?;
        
        // Get the transaction hash
        let tx_hash = match &submission_status {
            TxStatus::Submitted { hash, .. } => *hash,
            _ => return Err(TxError::InternalError("Expected Submitted status".to_string())),
        };
        
        // Step 4: Monitor for confirmation (if not urgent)
        if !tx.urgent {
            // Wait for confirmation with timeout
            return self.monitor.wait_for_confirmation(tx_hash, tx.confirmation_blocks, self.config.transaction_timeout).await;
        }
        
        Ok(submission_status)
    }
    
    /// Validate simulation results
    fn validate_simulation(&self, simulation: &SimulationResult) -> TxResult<()> {
        // Check simulation success
        if !simulation.success {
            return Err(TxError::SimulationFailed(
                format!("Transaction simulation failed: {:?}", simulation.error)
            ));
        }
        
        // Check gas used is within limit
        if simulation.gas_used > simulation.gas_limit {
            return Err(TxError::GasEstimationFailed(
                format!("Gas required ({}) exceeds limit ({})", 
                    simulation.gas_used, simulation.gas_limit)
            ));
        }
        
        // Additional validation logic can be added here
        
        Ok(())
    }
    
    /// Get transaction status
    pub async fn get_status(&self, tx_hash: H256) -> TxResult<TxStatus> {
        self.monitor.get_transaction_status(tx_hash).await
    }
    
    /// Simulate a transaction without executing it
    pub async fn simulate_only(&self, tx: &Transaction) -> TxResult<SimulationResult> {
        self.simulator.simulate(tx).await
    }
}
