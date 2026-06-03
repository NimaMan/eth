use alloy_primitives::{address, Address, U256};
use serde::{Deserialize, Serialize};

pub const WETH_ADDRESS: Address = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxAsset {
    NativeEth,
    Erc20(Address),
}

impl TxAsset {
    pub const fn native_eth() -> Self {
        Self::NativeEth
    }

    pub const fn erc20(address: Address) -> Self {
        Self::Erc20(address)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetFamily {
    Denom,
    Token,
    NativeEth,
    WrappedNative,
    OtherErc20(Address),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TxLedgerContext {
    pub token_address: Address,
    pub denom_address: Address,
    pub weth_address: Address,
}

impl TxLedgerContext {
    pub const fn new(token_address: Address, denom_address: Address) -> Self {
        Self {
            token_address,
            denom_address,
            weth_address: WETH_ADDRESS,
        }
    }

    pub const fn with_weth_address(
        token_address: Address,
        denom_address: Address,
        weth_address: Address,
    ) -> Self {
        Self {
            token_address,
            denom_address,
            weth_address,
        }
    }

    pub fn asset_family(&self, asset: TxAsset) -> AssetFamily {
        match asset {
            TxAsset::NativeEth if self.denom_address == self.weth_address => AssetFamily::Denom,
            TxAsset::NativeEth => AssetFamily::NativeEth,
            TxAsset::Erc20(address) if address == self.denom_address => AssetFamily::Denom,
            TxAsset::Erc20(address) if address == self.token_address => AssetFamily::Token,
            TxAsset::Erc20(address) if address == self.weth_address => AssetFamily::WrappedNative,
            TxAsset::Erc20(address) => AssetFamily::OtherErc20(address),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawSourceKind {
    ProcessedEthTransfer,
    InternalTrace,
    Erc20TransferLog,
    InternalErc20Transfer,
    WethDepositEvent,
    WethWithdrawEvent,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct MovementSource {
    pub kind: RawSourceKind,
    pub row_index: Option<usize>,
    pub log_index: Option<u64>,
    pub trace_index: Option<usize>,
    pub call_type: Option<String>,
}

impl MovementSource {
    pub const fn new(kind: RawSourceKind) -> Self {
        Self {
            kind,
            row_index: None,
            log_index: None,
            trace_index: None,
            call_type: None,
        }
    }

    pub const fn with_row_index(mut self, row_index: usize) -> Self {
        self.row_index = Some(row_index);
        self
    }

    pub const fn with_log_index(mut self, log_index: Option<u64>) -> Self {
        self.log_index = log_index;
        self
    }

    pub const fn with_trace_index(mut self, trace_index: usize) -> Self {
        self.trace_index = Some(trace_index);
        self
    }

    pub fn with_call_type(mut self, call_type: Option<String>) -> Self {
        self.call_type = call_type;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TxMovement {
    pub from: Address,
    pub to: Address,
    pub asset: TxAsset,
    pub amount: U256,
    pub source: MovementSource,
}

impl TxMovement {
    pub const fn new(
        from: Address,
        to: Address,
        asset: TxAsset,
        amount: U256,
        source: MovementSource,
    ) -> Self {
        Self {
            from,
            to,
            asset,
            amount,
            source,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawDelta {
    pub incoming: U256,
    pub outgoing: U256,
}

impl RawDelta {
    pub fn record_incoming(&mut self, amount: U256) {
        self.incoming = self.incoming.saturating_add(amount);
    }

    pub fn record_outgoing(&mut self, amount: U256) {
        self.outgoing = self.outgoing.saturating_add(amount);
    }

    pub fn is_net_zero(&self) -> bool {
        self.incoming == self.outgoing
    }

    pub fn net(&self) -> SignedRawAmount {
        if self.incoming >= self.outgoing {
            SignedRawAmount {
                sign: 1,
                amount: self.incoming - self.outgoing,
            }
        } else {
            SignedRawAmount {
                sign: -1,
                amount: self.outgoing - self.incoming,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SignedRawAmount {
    pub sign: i8,
    pub amount: U256,
}

impl SignedRawAmount {
    pub fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }

    pub fn raw_string(&self) -> String {
        if self.amount.is_zero() || self.sign >= 0 {
            self.amount.to_string()
        } else {
            format!("-{}", self.amount)
        }
    }
}
