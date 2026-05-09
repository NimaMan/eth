use crate::{
    error::{EthTxExecutorError, Result},
    types::{PreparedDirectRawTransaction, SignedTransaction},
};
use ethers_core::{
    types::{
        transaction::eip2718::TypedTransaction, Address, Bytes, Eip1559TransactionRequest,
        NameOrAddress, U64,
    },
    utils::keccak256,
};
use ethers_signers::{LocalWallet, Signer};
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

    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    pub async fn sign_direct_raw(
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
}
