use std::collections::VecDeque;
use revm_primitives::{Address, U256};
use revm_context::Database;
use revm_inspector::Inspector;
use revm_interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome};

/// Represents an internal ETH transfer from call tracing
#[derive(Debug, Clone)]
pub struct InternalTransfer {
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub depth: usize,
    pub call_type: String,
    pub success: bool,
}

/// Call tracer that captures internal ETH transfers similar to Python's trace processing
#[derive(Debug, Clone, Default)]
pub struct CallTracer {
    pub internal_transfers: Vec<InternalTransfer>,
    pub call_stack: VecDeque<CallFrame>,
    pub current_depth: usize,
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    from: Address,
    to: Address,
    value: U256,
    depth: usize,
    call_type: String,
}

impl CallTracer {
    pub fn new() -> Self {
        Self {
            internal_transfers: Vec::new(),
            call_stack: VecDeque::new(),
            current_depth: 0,
        }
    }

    pub fn clear(&mut self) {
        self.internal_transfers.clear();
        self.call_stack.clear();
        self.current_depth = 0;
    }

    /// Get all internal transfers with non-zero value
    pub fn get_internal_transfers(&self) -> &[InternalTransfer] {
        &self.internal_transfers
    }
}

impl<CTX: Database> Inspector<CTX> for CallTracer {
    /// Called when a call to a contract is about to start
    fn call(&mut self, _context: &mut CTX, inputs: &mut CallInputs) -> Option<CallOutcome> {
        // Only track calls with value > 0 (ETH transfers)
        if inputs.call_value() > U256::ZERO {
            let call_frame = CallFrame {
                from: inputs.caller,
                to: inputs.target_address,
                value: inputs.call_value(),
                depth: self.current_depth,
                call_type: format!("{:?}", inputs.scheme),
            };
            
            self.call_stack.push_back(call_frame);
        }
        
        self.current_depth += 1;
        None // Don't override execution
    }

    /// Called when a call to a contract has concluded
    fn call_end(&mut self, _context: &mut CTX, inputs: &CallInputs, outcome: &mut CallOutcome) {
        self.current_depth = self.current_depth.saturating_sub(1);
        
        // Check if this was a value transfer call
        if inputs.call_value() > U256::ZERO {
            if let Some(call_frame) = self.call_stack.pop_back() {
                // Record the internal transfer
                let transfer = InternalTransfer {
                    from: call_frame.from,
                    to: call_frame.to,
                    value: call_frame.value,
                    depth: call_frame.depth,
                    call_type: call_frame.call_type,
                    success: outcome.result.result.is_ok(),
                };
                
                self.internal_transfers.push(transfer);
            }
        }
    }

    /// Called when a contract is about to be created
    fn create(&mut self, _context: &mut CTX, inputs: &mut CreateInputs) -> Option<CreateOutcome> {
        // Track CREATE operations with value
        if inputs.value > U256::ZERO {
            let call_frame = CallFrame {
                from: inputs.caller,
                to: Address::ZERO, // Will be set in create_end when address is known
                value: inputs.value,
                depth: self.current_depth,
                call_type: "CREATE".to_string(),
            };
            
            self.call_stack.push_back(call_frame);
        }
        
        self.current_depth += 1;
        None
    }

    /// Called when a contract has been created
    fn create_end(
        &mut self,
        _context: &mut CTX,
        inputs: &CreateInputs,
        outcome: &mut CreateOutcome,
    ) {
        self.current_depth = self.current_depth.saturating_sub(1);
        
        if inputs.value > U256::ZERO {
            if let Some(mut call_frame) = self.call_stack.pop_back() {
                // Set the created contract address if creation was successful
                if let Some(created_address) = outcome.address {
                    call_frame.to = created_address;
                }
                
                let transfer = InternalTransfer {
                    from: call_frame.from,
                    to: call_frame.to,
                    value: call_frame.value,
                    depth: call_frame.depth,
                    call_type: call_frame.call_type,
                    success: outcome.result.result.is_ok(),
                };
                
                self.internal_transfers.push(transfer);
            }
        }
    }
}