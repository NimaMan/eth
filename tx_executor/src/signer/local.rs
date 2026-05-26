use super::TransactionSigner;
use crate::{
    error::{EthTxExecutorError, Result},
    types::{PreparedDirectRawTransaction, SignedFlashbotsAuth, SignedTransaction},
};
use async_trait::async_trait;
use ethers_core::{
    types::{
        transaction::eip2718::TypedTransaction, Address, Bytes, Eip1559TransactionRequest,
        NameOrAddress, U64,
    },
    utils::keccak256,
};
use ethers_signers::{LocalWallet, Signer};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
pub struct LocalTransactionSigner {
    wallet: LocalWallet,
}

impl LocalTransactionSigner {
    pub fn from_private_key(private_key: &str, chain_id: u64) -> Result<Self> {
        let wallet = LocalWallet::from_str(private_key.trim())
            .map_err(|err| EthTxExecutorError::Signer(format!("invalid private key: {err}")))?
            .with_chain_id(chain_id);
        Ok(Self { wallet })
    }

    pub fn from_env(env_var: &str, chain_id: u64) -> Result<Self> {
        let private_key = std::env::var(env_var).map_err(|_| {
            EthTxExecutorError::Config(format!("environment variable {env_var} is not set"))
        })?;
        Self::from_private_key(&private_key, chain_id)
    }

    pub fn from_keystore<P, S>(keypath: P, password: S, chain_id: u64) -> Result<Self>
    where
        P: AsRef<Path>,
        S: AsRef<[u8]>,
    {
        let wallet = LocalWallet::decrypt_keystore(keypath, password)
            .map_err(|err| EthTxExecutorError::Signer(format!("invalid keystore: {err}")))?
            .with_chain_id(chain_id);
        Ok(Self { wallet })
    }
}

#[async_trait]
impl TransactionSigner for LocalTransactionSigner {
    fn address(&self) -> Address {
        self.wallet.address()
    }

    async fn sign_direct_raw(
        &self,
        request: &PreparedDirectRawTransaction,
    ) -> Result<SignedTransaction> {
        let tx = Eip1559TransactionRequest {
            from: Some(request.from),
            to: Some(NameOrAddress::Address(request.to)),
            gas: Some(request.gas_limit),
            value: Some(request.value),
            data: Some(Bytes::from(request.data.clone())),
            nonce: request.nonce,
            access_list: Default::default(),
            max_priority_fee_per_gas: Some(request.max_priority_fee_per_gas),
            max_fee_per_gas: Some(request.max_fee_per_gas),
            chain_id: Some(U64::from(request.chain_id)),
        };
        let typed = TypedTransaction::Eip1559(tx);
        let signature = self.wallet.sign_transaction(&typed).await?;
        let raw = typed.rlp_signed(&signature);
        let tx_hash = keccak256(raw.as_ref()).into();

        Ok(SignedTransaction {
            tx_hash,
            raw_tx_hex: format!("0x{}", hex::encode(raw.as_ref())),
        })
    }

    async fn sign_flashbots_auth(&self, body_hash: &str) -> Result<SignedFlashbotsAuth> {
        validate_flashbots_body_hash(body_hash)?;
        let signature = self.wallet.sign_message(body_hash.to_string()).await?;
        let signature = flashbots_signature_hex(signature);
        Ok(SignedFlashbotsAuth {
            body_hash: body_hash.to_string(),
            signature,
        })
    }
}

fn validate_flashbots_body_hash(body_hash: &str) -> Result<()> {
    let trimmed = body_hash.trim();
    if trimmed.len() == 66 && trimmed.starts_with("0x") {
        Ok(())
    } else {
        Err(EthTxExecutorError::Signer(
            "Flashbots auth body hash must be a 32-byte 0x-prefixed hex value".to_string(),
        ))
    }
}

fn flashbots_signature_hex(signature: ethers_core::types::Signature) -> String {
    let mut bytes = [0_u8; 65];
    signature.r.to_big_endian(&mut bytes[..32]);
    signature.s.to_big_endian(&mut bytes[32..64]);
    bytes[64] = normalized_recovery_byte(signature.v);
    format!("0x{}", hex::encode(bytes))
}

fn normalized_recovery_byte(v: u64) -> u8 {
    match v {
        0..=26 => (v % 4) as u8,
        27..=34 => ((v - 27) % 4) as u8,
        _ => ((v - 1) % 2) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sign_flashbots_auth_uses_normalized_recovery_byte() {
        let signer = LocalTransactionSigner::from_private_key(
            "0x59c6995e998f97a5a0044966f094538b8ba11502bc50c61e1722e5fb2935b8f1",
            1,
        )
        .unwrap();
        let body_hash = format!("0x{}", "11".repeat(32));

        let signed = signer.sign_flashbots_auth(&body_hash).await.unwrap();
        let bytes = hex::decode(signed.signature.trim_start_matches("0x")).unwrap();

        assert_eq!(signed.body_hash, body_hash);
        assert_eq!(bytes.len(), 65);
        assert!(matches!(bytes[64], 0 | 1));
    }
}
