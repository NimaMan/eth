//! Integration bridge: token-network addresses → tx_fund_flow traces.
//!
//! Delegates actual flow extraction to `tx_fund_flow::fundflownetwork`.
//! This module only handles:
//! - Selecting addresses of interest from the token graph
//! - Formatting traces for TokenNetworkView consumption
//! - Correlating token-network edges with fund-flow evidence

use alloy_primitives::Address;
use eyre::Result;

/// A single hop in a fund flow trace.
#[derive(Clone, Debug)]
pub struct FundFlowHop {
    pub from: Address,
    pub to: Address,
    pub block_number: u64,
    pub tx_hash: String,
    pub amount: f64,
    pub asset: String,
}

/// Complete fund flow trace from source to target.
#[derive(Clone, Debug)]
pub struct FundFlowTrace {
    pub source: Address,
    pub target: Address,
    pub hops: Vec<FundFlowHop>,
    pub total_amount: f64,
    pub first_block: u64,
    pub last_block: u64,
}

/// Bridges token network addresses to tx_fund_flow traces.
pub struct TokenFundFlowTracer;

impl TokenFundFlowTracer {
    pub fn new() -> Self {
        Self
    }

    /// Trace direct transfers between two addresses of interest.
    /// Delegates to `tx_fund_flow::FundFlowAnalyzer` for actual extraction.
    pub fn trace_direct(&self, _source: Address, _target: Address) -> Result<Vec<FundFlowTrace>> {
        // TODO: Use tx_fund_flow::extract_fund_flows_from_processed_tx + address block index
        Ok(vec![])
    }

    /// Trace multi-hop flows up to max_hops.
    /// Delegates to `tx_fund_flow::GraphExplorer` for BFS discovery.
    pub fn trace_multi_hop(
        &self,
        _source: Address,
        _target: Address,
        _max_hops: usize,
    ) -> Result<Vec<FundFlowTrace>> {
        // TODO: Use tx_fund_flow::graph_discovery::GraphExplorer
        Ok(vec![])
    }
}
