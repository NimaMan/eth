// tx.rs — single processed-transaction inspector endpoint
//
// Algorithm:
//   GET /api/v1/eth/tx/{hash}
//     1. Normalize + parse the path hash into a 32-byte B256 (400 on bad input).
//     2. Build a ProcessedTxProvider from the server's ALREADY-OPEN reth provider
//        factory (no MDBX reopen — the factory is shared, same as the price
//        service in ServerState).
//     3. process_transaction_by_hash() loads the tx fresh from the reth DB and
//        replays its call trace (NO disk-cache involvement), producing the full
//        ProcessedTransaction — including `internal_erc20_calls`, the trace-decoded
//        ERC-20 moves that capture event-less custody drains.
//     4. Serialize the ProcessedTransaction to JSON (200), or map the error to
//        404 (tx not found) / 500.
//
// This endpoint is chain-neutral on the wire: the response is exactly the
// serialized ProcessedTransaction, which the shared inspector UI renders for any
// chain via the chain registry's explorer links.

use std::convert::Infallible;
use std::str::FromStr;

use alloy_primitives::B256;
use tx_processor::ProcessedTxProvider;
use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;

pub(super) async fn processed_tx(
    hash: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let trimmed = hash.trim();
    let body = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let normalized = format!("0x{body}");

    let tx_hash = match B256::from_str(&normalized) {
        Ok(tx_hash) => tx_hash,
        Err(error) => {
            return Ok(error_response(
                format!("invalid transaction hash: {error}"),
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    // Reuse the server's open reth provider factory (no MDBX reopen).
    let factory = (**state.provider.provider_factory()).clone();
    let provider = match ProcessedTxProvider::with_provider_factory(factory) {
        Ok(provider) => provider,
        Err(error) => {
            return Ok(error_response(
                format!("failed to initialize processed tx provider: {error}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    match provider.process_transaction_by_hash(tx_hash).await {
        Ok(processed) => Ok(json_response(&processed, StatusCode::OK)),
        Err(error) => {
            let message = error.to_string();
            let lowered = message.to_ascii_lowercase();
            let status = if lowered.contains("not found") || lowered.contains("no transaction") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Ok(error_response(
                format!("failed to process transaction {normalized}: {message}"),
                status,
            ))
        }
    }
}
