use ethers_core::types::Address;
use serde::{Deserialize, Serialize};

use crate::types::{PreparedDirectRawTransaction, SignedTransaction};

pub const ETH_SIGNER_WIRE_SCHEMA: &str = "kartal_eth_signer_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SignerWireRequest {
    Status {
        schema: String,
    },
    SignDirectRaw {
        schema: String,
        transaction: PreparedDirectRawTransaction,
    },
}

impl SignerWireRequest {
    pub fn status() -> Self {
        Self::Status {
            schema: ETH_SIGNER_WIRE_SCHEMA.to_string(),
        }
    }

    pub fn sign_direct_raw(transaction: PreparedDirectRawTransaction) -> Self {
        Self::SignDirectRaw {
            schema: ETH_SIGNER_WIRE_SCHEMA.to_string(),
            transaction,
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SignerWireResponse {
    Status {
        schema: String,
        status: SignerStatus,
    },
    Signed {
        schema: String,
        signer: Address,
        signed: SignedTransaction,
    },
    Error {
        schema: String,
        error: String,
    },
}

impl SignerWireResponse {
    pub fn status(status: SignerStatus) -> Self {
        Self::Status {
            schema: ETH_SIGNER_WIRE_SCHEMA.to_string(),
            status,
        }
    }

    pub fn signed(signer: Address, signed: SignedTransaction) -> Self {
        Self::Signed {
            schema: ETH_SIGNER_WIRE_SCHEMA.to_string(),
            signer,
            signed,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self::Error {
            schema: ETH_SIGNER_WIRE_SCHEMA.to_string(),
            error: error.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerStatus {
    pub ready: bool,
    pub signer: Address,
    pub chain_id: u64,
}
