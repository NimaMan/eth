/// Unsigned transaction simulation methods
/// 
/// This module provides methods for simulating unsigned transactions (like debug_traceCall).
/// Most of the implementation is delegated to the call_simulator module.

use crate::{
    simulator::TxSimulator,
    types::SimulationResult,
    call_simulator::CallRequest,
};
use eyre::Result;

impl TxSimulator {
    /// Simulate an unsigned transaction (like debug_traceCall)
    /// 
    /// Alias for simulate_call_at_block with latest block
    pub async fn simulate_call(&self, call: CallRequest) -> Result<SimulationResult> {
        let block = self.get_latest_block()?;
        self.simulate_call_at_block(call, block).await
    }
    
    /// Simulate an unsigned transaction at specific block
    /// 
    /// This delegates to the implementation in call_simulator module
    pub async fn simulate_call_at_block(
        &self,
        call: CallRequest,
        block_number: u64,
    ) -> Result<SimulationResult> {
        // Delegate to simulate_unsigned_transaction_at_block
        self.simulate_unsigned_transaction_at_block(call, block_number).await
    }
}