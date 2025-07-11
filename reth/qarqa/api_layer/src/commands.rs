//! Command implementations

use qarqa_core_types::QarqaResult;

/// Command handler interface
pub trait CommandHandler {
    /// Execute the command
    fn execute(&self) -> QarqaResult<()>;
}

/// Fund flow command
pub struct FundFlowCommand {
    pub address: String,
    pub depth: u32,
    pub currency: String,
    pub min_value: f64,
    pub limit: usize,
}

impl CommandHandler for FundFlowCommand {
    fn execute(&self) -> QarqaResult<()> {
        // Implementation handled in main binary
        Ok(())
    }
}