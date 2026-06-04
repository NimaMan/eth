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

/// A trace-derived ERC-20 **transfer** that moved a balance but emitted **no**
/// matching `Transfer` event — the event-less complement of `erc20_transfers`.
///
/// This is the normalized, balance-relevant projection of [`InternalErc20Call`]:
/// only the value-moving kinds (`Transfer` / `TransferFrom`), only succeeded
/// calls with a non-zero amount, and only those with no emitted `Transfer` event.
/// `erc20_transfers` ∪ `internal_erc20_transfers` is the COMPLETE transfer set
/// for the transaction, and both feed `address_balance_changes` so an event-less
/// custody drain shows up in net movement. See `data_models/README.md`
/// ("Capturing all transfers").
///
/// Note: `amount` is the call argument, not a measured balance delta — exact for
/// plain transfers, approximate for fee-on-transfer / rebasing tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalErc20Transfer {
    /// The token contract whose balance moved.
    pub token_address: Address,
    /// The holder whose balance left (`from_address` of the move).
    pub from_address: Address,
    /// The recipient.
    pub to_address: Address,
    pub amount: U256,
    /// `msg.sender` of the originating call (provenance).
    pub caller: Address,
    /// Whether the move came from a `transfer` or a `transferFrom` call.
    pub kind: Erc20CallKind,
    /// Call-trace depth of the originating call (provenance).
    pub depth: u32,
}

impl InternalErc20Transfer {
    /// Project the event-less, balance-moving subset of `calls`, deduplicated
    /// against the emitted `Transfer` events in `events`.
    pub fn event_less_complement(
        calls: &[InternalErc20Call],
        events: &[super::receipt_models::ERC20TransferEvent],
    ) -> Vec<InternalErc20Transfer> {
        calls
            .iter()
            .filter(|call| {
                call.succeeded
                    && !call.amount.is_zero()
                    && matches!(
                        call.kind,
                        Erc20CallKind::Transfer | Erc20CallKind::TransferFrom
                    )
            })
            .filter(|call| {
                // Event-less: no emitted Transfer event with the same token, from,
                // to, and amount. Standard transfers DO emit an event and are kept
                // out of this set so the union with `erc20_transfers` never
                // double-counts them.
                !events.iter().any(|event| {
                    event.token_address == call.token_address
                        && event.from_address == call.from_address
                        && event.to_address == call.to_address
                        && event.amount == call.amount
                })
            })
            .map(|call| InternalErc20Transfer {
                token_address: call.token_address,
                from_address: call.from_address,
                to_address: call.to_address,
                amount: call.amount,
                caller: call.caller,
                kind: call.kind,
                depth: call.depth,
            })
            .collect()
    }
}
