use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalTransaction {
    pub from_address: Address,
    pub to_address: Option<Address>,
    pub value: U256,
    pub gas: u64,
    pub gas_used: u64,
    pub trace_type: String,
    pub call_type: Option<String>,
    pub depth: u32,
    pub error: Option<String>,
}

/// Which standard ERC-20 mutating function an [`InternalErc20Call`] decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Erc20CallKind {
    Transfer,
    TransferFrom,
    Approve,
}

impl Erc20CallKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transfer => "transfer",
            Self::TransferFrom => "transfer_from",
            Self::Approve => "approve",
        }
    }
}

/// A standard ERC-20 mutating call (`transfer` / `transferFrom` / `approve`)
/// decoded from the transaction call trace, at any depth.
///
/// This captures balance-affecting calls that emit **no** `Transfer` event and
/// that the top-level-only `transfer_from_calls` path misses (internal calls
/// where `tx.to` is not the token). Consumers cross-reference these against the
/// emitted `Transfer` events (`erc20_transfers`) to find event-less moves — the
/// custody-drain signature.
///
/// Field meaning by `kind`:
/// - `Transfer`: `from_address` = caller (token holder), `to_address` =
///   recipient.
/// - `TransferFrom`: `from_address` = arg0 (the holder whose balance moves),
///   `to_address` = recipient. `caller` is `msg.sender` of the call.
/// - `Approve`: `from_address` = caller (owner), `to_address` = spender;
///   `amount` is the allowance set (no balance move).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalErc20Call {
    /// The token contract the call targeted (the call frame's `to`).
    pub token_address: Address,
    /// `msg.sender` of the call (the call frame's `from`).
    pub caller: Address,
    pub kind: Erc20CallKind,
    pub from_address: Address,
    pub to_address: Address,
    pub amount: U256,
    pub depth: u32,
    pub call_type: Option<String>,
    /// Whether the call frame completed without error/revert.
    pub succeeded: bool,
}
