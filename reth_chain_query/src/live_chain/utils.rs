use alloy_primitives::B256;
use serde_json::Value;

pub fn transaction_hashes_from_block(block: &Value) -> eyre::Result<Vec<B256>> {
    let transactions = block
        .get("transactions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| eyre::eyre!("transactions missing in block payload"))?;

    let mut hashes = Vec::with_capacity(transactions.len());
    for tx in transactions {
        let hash = tx
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| eyre::eyre!("transaction hash missing from block payload"))?;
        hashes.push(parse_hash(hash)?);
    }
    Ok(hashes)
}

fn parse_hash(value: &str) -> eyre::Result<B256> {
    let trimmed = value.strip_prefix("0x").unwrap_or(value);
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(trimmed, &mut bytes)
        .map_err(|err| eyre::eyre!("invalid transaction hash {}: {}", value, err))?;
    Ok(B256::from(bytes))
}
