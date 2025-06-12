use std::collections::VecDeque;
use revm_primitives::{Address, U256};
use revm_inspector::Inspector;
use revm_interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome, CallScheme};
use revm_interpreter::interpreter::EthInterpreter;
use alloy_primitives::Bytes;

/// Represents an internal ETH transfer from call tracing
#[derive(Debug, Clone)]
pub struct InternalTransfer {
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub depth: usize,
    pub call_type: CallType,
}

/// Call type enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallType {
    Call,
    DelegateCall,
    StaticCall,
    Create,
    Create2,
}

impl std::fmt::Display for CallType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallType::Call => write!(f, "CALL"),
            CallType::DelegateCall => write!(f, "DELEGATECALL"),
            CallType::StaticCall => write!(f, "STATICCALL"),
            CallType::Create => write!(f, "CREATE"),
            CallType::Create2 => write!(f, "CREATE2"),
        }
    }
}

/// Represents a call trace
#[derive(Debug, Clone)]
pub struct CallTrace {
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub input: Bytes,
    pub output: Bytes,
    pub gas_used: u64,
    pub call_type: CallType,
    pub status: bool,
    pub error: Option<String>,
    pub calls: Vec<CallTrace>,
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
    call_type: CallType,
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
    
    /// Extract traces from the call tracer
    pub fn extract_traces(&self) -> Vec<CallTrace> {
        // TODO: Implement trace extraction
        vec![]
    }
    
    /// Get all internal transfers
    pub fn get_internal_transfers(&self) -> Vec<InternalTransfer> {
        self.internal_transfers.clone()
    }
    
    /// Get traces
    pub fn get_traces(&self) -> Vec<CallTrace> {
        self.extract_traces()
    }
    
    /// Manually record an internal transfer (for testing)
    pub fn record_internal_transfer(&mut self, from: Address, to: Address, value: U256) {
        self.internal_transfers.push(InternalTransfer {
            from,
            to,
            value,
            depth: self.current_depth,
            call_type: CallType::Call,
        });
    }
    
    /// Get internal transfers by reference
    pub fn internal_transfers_ref(&self) -> &[InternalTransfer] {
        &self.internal_transfers
    }
}

impl<CTX> Inspector<CTX, EthInterpreter> for CallTracer {
    /// Called when a call to a contract is about to start
    fn call(&mut self, _context: &mut CTX, inputs: &mut CallInputs) -> Option<CallOutcome> {
        // Debug logging
        eprintln!("🔍 CallTracer: CALL from {:?} to {:?} with value {:?} at depth {}", 
            inputs.caller, inputs.target_address, inputs.call_value(), self.current_depth);
        
        // Only track calls with value > 0 (ETH transfers)
        if inputs.call_value() > U256::ZERO {
            eprintln!("💰 Recording internal transfer of {} wei", inputs.call_value());
            let call_frame = CallFrame {
                from: inputs.caller,
                to: inputs.target_address,
                value: inputs.call_value(),
                depth: self.current_depth,
                call_type: match inputs.scheme {
                    CallScheme::Call | CallScheme::CallCode => CallType::Call,
                    CallScheme::DelegateCall => CallType::DelegateCall,
                    CallScheme::StaticCall => CallType::StaticCall,
                    CallScheme::ExtCall => CallType::Call,
                    CallScheme::ExtStaticCall => CallType::StaticCall,
                    CallScheme::ExtDelegateCall => CallType::DelegateCall,
                },
            };
            
            self.call_stack.push_back(call_frame);
        }
        
        self.current_depth += 1;
        None // Don't override execution
    }

    /// Called when a call to a contract has concluded
    fn call_end(&mut self, _context: &mut CTX, inputs: &CallInputs, _outcome: &mut CallOutcome) {
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
                call_type: match inputs.scheme {
                    revm_interpreter::CreateScheme::Create => CallType::Create,
                    revm_interpreter::CreateScheme::Create2 { .. } => CallType::Create2,
                    revm_interpreter::CreateScheme::Custom { .. } => CallType::Create,
                },
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
                };
                
                self.internal_transfers.push(transfer);
            }
        }
    }
}