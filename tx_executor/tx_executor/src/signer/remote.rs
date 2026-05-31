use std::path::{Path, PathBuf};

use async_trait::async_trait;
use ethers_core::types::Address;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use super::{SignerWireRequest, SignerWireResponse, TransactionSigner, ETH_SIGNER_WIRE_SCHEMA};
use crate::{
    error::{EthTxExecutorError, Result},
    types::{PreparedDirectRawTransaction, SignedTransaction},
};

#[derive(Debug, Clone)]
pub struct UnixSocketTransactionSigner {
    socket_path: PathBuf,
    signer_address: Address,
}

impl UnixSocketTransactionSigner {
    pub fn new(socket_path: impl Into<PathBuf>, signer_address: Address) -> Self {
        Self {
            socket_path: socket_path.into(),
            signer_address,
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    async fn send_request(&self, request: SignerWireRequest) -> Result<SignerWireResponse> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|err| {
                EthTxExecutorError::Signer(format!(
                    "failed to connect to signer socket {}: {err}",
                    self.socket_path.display()
                ))
            })?;

        let mut encoded = serde_json::to_vec(&request)?;
        encoded.push(b'\n');
        stream.write_all(&encoded).await.map_err(|err| {
            EthTxExecutorError::Signer(format!(
                "failed to write signer request to {}: {err}",
                self.socket_path.display()
            ))
        })?;
        stream.shutdown().await.map_err(|err| {
            EthTxExecutorError::Signer(format!(
                "failed to shutdown signer request to {}: {err}",
                self.socket_path.display()
            ))
        })?;

        let mut reader = BufReader::new(stream);
        let mut response = Vec::new();
        let read = reader
            .read_until(b'\n', &mut response)
            .await
            .map_err(|err| {
                EthTxExecutorError::Signer(format!(
                    "failed to read signer response from {}: {err}",
                    self.socket_path.display()
                ))
            })?;
        if read == 0 {
            return Err(EthTxExecutorError::Signer(format!(
                "signer socket {} closed without a response",
                self.socket_path.display()
            )));
        }

        serde_json::from_slice(response.trim_ascii()).map_err(Into::into)
    }
}

#[async_trait]
impl TransactionSigner for UnixSocketTransactionSigner {
    fn address(&self) -> Address {
        self.signer_address
    }

    async fn sign_direct_raw(
        &self,
        request: &PreparedDirectRawTransaction,
    ) -> Result<SignedTransaction> {
        let response = self
            .send_request(SignerWireRequest::sign_direct_raw(request.clone()))
            .await?;

        match response {
            SignerWireResponse::Signed {
                schema,
                signer,
                signed,
            } => {
                if schema != ETH_SIGNER_WIRE_SCHEMA {
                    return Err(EthTxExecutorError::Signer(format!(
                        "unexpected signer response schema {schema:?}"
                    )));
                }
                if signer != self.signer_address {
                    return Err(EthTxExecutorError::Signer(format!(
                        "signer response address {signer:?} does not match configured signer {:?}",
                        self.signer_address
                    )));
                }
                Ok(signed)
            }
            SignerWireResponse::Error { error, .. } => Err(EthTxExecutorError::Signer(error)),
            other => Err(EthTxExecutorError::Signer(format!(
                "unexpected signer response kind: {other:?}"
            ))),
        }
    }
}

trait TrimAscii {
    fn trim_ascii(&self) -> &[u8];
}

impl TrimAscii for Vec<u8> {
    fn trim_ascii(&self) -> &[u8] {
        let mut start = 0;
        let mut end = self.len();
        while start < end && self[start].is_ascii_whitespace() {
            start += 1;
        }
        while end > start && self[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        &self[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::LocalTransactionSigner;
    use ethers_core::types::U256;
    use serde_json::json;
    use tokio::net::UnixListener;

    #[tokio::test]
    async fn unix_socket_signer_round_trips_signed_transaction() {
        let path =
            std::env::temp_dir().join(format!("tx-executor-signer-{}.sock", uuid::Uuid::new_v4()));
        let listener = UnixListener::bind(&path).unwrap();
        let local = LocalTransactionSigner::from_private_key(
            "0x59c6995e998f97a5a0044966f094538b8ba11502bc50c61e1722e5fb2935b8f1",
            1,
        )
        .unwrap();
        let signer_address = local.address();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(stream);
            let mut request = Vec::new();
            reader.read_until(b'\n', &mut request).await.unwrap();
            let request: SignerWireRequest = serde_json::from_slice(request.trim_ascii()).unwrap();
            let response = match request {
                SignerWireRequest::SignDirectRaw { transaction, .. } => {
                    let signed = local.sign_direct_raw(&transaction).await.unwrap();
                    SignerWireResponse::signed(local.address(), signed)
                }
                _ => SignerWireResponse::error("unexpected request"),
            };
            let mut stream = reader.into_inner();
            let mut encoded = serde_json::to_vec(&response).unwrap();
            encoded.push(b'\n');
            stream.write_all(&encoded).await.unwrap();
        });

        let transaction = PreparedDirectRawTransaction {
            attempt_id: "attempt-1".to_string(),
            chain_id: 1,
            from: signer_address,
            to: "0x0000000000000000000000000000000000000002"
                .parse()
                .unwrap(),
            value: U256::zero(),
            data: vec![0x5f, 0x41, 0x3d, 0x10],
            gas_limit: U256::from(100_000),
            max_fee_per_gas: U256::from(100),
            max_priority_fee_per_gas: U256::from(10),
            nonce: Some(U256::from(1)),
            simulation: None,
            metadata: json!({}),
        };
        let remote = UnixSocketTransactionSigner::new(&path, signer_address);
        let signed = remote.sign_direct_raw(&transaction).await.unwrap();

        assert!(signed.raw_tx_hex.starts_with("0x"));
        assert_ne!(signed.raw_tx_hex, "0x");
        let _ = tokio::fs::remove_file(path).await;
    }
}
