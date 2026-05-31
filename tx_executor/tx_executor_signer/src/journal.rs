use std::path::PathBuf;

use anyhow::{Context, Result};
use ethers_core::{types::Address, utils::keccak256};
use serde_json::{json, Value};
use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};
use tx_executor::types::PreparedDirectRawTransaction;

use super::policy::calldata_selector;

#[derive(Debug, Clone)]
pub struct EthSignerJournal {
    path: Option<PathBuf>,
}

impl EthSignerJournal {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub async fn record(
        &self,
        decision: &str,
        signer: Address,
        transaction: Option<&PreparedDirectRawTransaction>,
        reasons: &[String],
        extra: Value,
    ) -> Result<()> {
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create signer journal dir {}", parent.display())
            })?;
        }

        let event = json!({
            "schema": "eth_tx_signer_journal_v1",
            "ts": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "decision": decision,
            "signer": format!("{signer:?}"),
            "attempt_id": transaction.map(|tx| tx.attempt_id.clone()),
            "transaction": transaction.map(sanitized_transaction),
            "reasons": reasons,
            "extra": extra,
        });

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .with_context(|| format!("failed to open signer journal {}", path.display()))?;
        file.write_all(event.to_string().as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }
}

fn sanitized_transaction(transaction: &PreparedDirectRawTransaction) -> Value {
    json!({
        "chain_id": transaction.chain_id,
        "from": format!("{:?}", transaction.from),
        "to": format!("{:?}", transaction.to),
        "value": transaction.value.to_string(),
        "selector": calldata_selector(&transaction.data),
        "data_len": transaction.data.len(),
        "data_hash": format!("0x{}", hex::encode(keccak256(&transaction.data))),
        "gas_limit": transaction.gas_limit.to_string(),
        "max_fee_per_gas": transaction.max_fee_per_gas.to_string(),
        "max_priority_fee_per_gas": transaction.max_priority_fee_per_gas.to_string(),
        "nonce": transaction.nonce.map(|nonce| nonce.to_string()),
        "simulation": transaction.simulation,
        "metadata": transaction.metadata,
    })
}
