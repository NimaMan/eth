use std::sync::Arc;
use std::time::Duration;

use eyre::{eyre, Result, WrapErr};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::time;
use tracing::{info, warn};
use tx_processor::{sealed_header_from_processed_block_header, LiveBlockStateFrame};
use tx_simulator::{InMemoryLiveBlockStateProvider, LiveBlockState, TxSimulator};

use super::TokenServerClient;

const LIVE_STATE_STREAM_RECONNECT_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Deserialize)]
pub(super) struct LiveStateFrameResponse {
    pub available: bool,
    pub state: Option<RecentLiveStateFrame>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct RecentLiveStateFrame {
    pub frame: LiveBlockStateFrame,
    pub applied_at_unix_ms: u64,
}

pub(super) fn spawn_live_state_publisher(
    client: TokenServerClient,
    provider: InMemoryLiveBlockStateProvider,
    simulator: Arc<TxSimulator>,
) {
    tokio::spawn(async move {
        let mut last_published_block = None;
        loop {
            match publish_live_state_stream(
                &client,
                &provider,
                simulator.clone(),
                &mut last_published_block,
            )
            .await
            {
                Ok(()) => warn!("chain-server live state stream ended"),
                Err(error) => {
                    warn!(error = %error, "chain-server live state stream failed");
                }
            }
            time::sleep(LIVE_STATE_STREAM_RECONNECT_DELAY).await;
        }
    });
}

async fn publish_live_state_stream(
    client: &TokenServerClient,
    provider: &InMemoryLiveBlockStateProvider,
    simulator: Arc<TxSimulator>,
    last_published_block: &mut Option<u64>,
) -> Result<()> {
    let response = client.live_state_stream().await?;
    let mut chunks = response.bytes_stream();
    let mut buffer = Vec::new();

    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.wrap_err("failed to read chain-server live state stream")?;
        buffer.extend_from_slice(&chunk);
        while let Some(event) = take_sse_event(&mut buffer) {
            let Some(data) = sse_event_data(&event)? else {
                continue;
            };
            let response = serde_json::from_str::<LiveStateFrameResponse>(&data)
                .wrap_err("failed to decode chain-server live state stream event")?;
            if let Some(block_number) = publish_live_state_response(
                response,
                provider,
                simulator.clone(),
                last_published_block,
            )
            .await?
            {
                info!(block_number, "published chain-server live state frame");
            }
        }
    }

    Ok(())
}

async fn publish_live_state_response(
    response: LiveStateFrameResponse,
    provider: &InMemoryLiveBlockStateProvider,
    simulator: Arc<TxSimulator>,
    last_published_block: &mut Option<u64>,
) -> Result<Option<u64>> {
    if !response.available {
        return Ok(None);
    }
    let state = response
        .state
        .ok_or_else(|| eyre!("chain-server live state response was available without state"))?;
    let applied_at_unix_ms = state.applied_at_unix_ms;
    let block_number = state.frame.header.number;
    if Some(block_number) == *last_published_block {
        return Ok(None);
    }
    let live_state = live_block_state_from_frame(simulator, state.frame)
        .await
        .wrap_err_with(|| format!("failed to build live block state for {block_number}"))?;
    provider.publish_latest(live_state)?;
    *last_published_block = Some(block_number);
    tracing::debug!(
        block_number,
        applied_at_unix_ms,
        "accepted chain-server live state frame"
    );
    Ok(Some(block_number))
}

fn take_sse_event(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let (event_end, boundary_len) = find_sse_event_boundary(buffer)?;
    let event = buffer.drain(..event_end).collect::<Vec<_>>();
    buffer.drain(..boundary_len);
    Some(event)
}

fn find_sse_event_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf_boundary = buffer
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|index| (index, 2));
    let crlf_boundary = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| (index, 4));

    match (lf_boundary, crlf_boundary) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(boundary), None) | (None, Some(boundary)) => Some(boundary),
        (None, None) => None,
    }
}

fn sse_event_data(event: &[u8]) -> Result<Option<String>> {
    let event =
        std::str::from_utf8(event).wrap_err("chain-server live state stream was not utf8")?;
    let mut data_lines = Vec::new();
    for line in event.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        data_lines.push(data.strip_prefix(' ').unwrap_or(data));
    }

    if data_lines.is_empty() {
        Ok(None)
    } else {
        Ok(Some(data_lines.join("\n")))
    }
}

async fn live_block_state_from_frame(
    simulator: Arc<TxSimulator>,
    frame: LiveBlockStateFrame,
) -> Result<LiveBlockState> {
    let block_number = frame.header.number;
    let state_diffs = frame
        .state_diffs
        .ok_or_else(|| eyre!("chain-server live state frame {block_number} has no state diffs"))?;
    if state_diffs.len() != frame.transaction_count {
        return Err(eyre!(
            "chain-server live state frame {} has {} state diffs for {} transactions",
            block_number,
            state_diffs.len(),
            frame.transaction_count
        ));
    }
    let header = sealed_header_from_processed_block_header(&frame.header);
    let session = simulator
        .block_state_session_from_prestate_diffs(
            block_number,
            frame.header.hash,
            frame.header.parent_hash,
            header,
            &state_diffs,
        )
        .await?;
    Ok(LiveBlockState::new(session).with_block_hash(frame.header.hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_complete_sse_events() {
        let mut buffer = b"event: live_state\ndata: {\"available\":true}\n\n".to_vec();

        let event = take_sse_event(&mut buffer).expect("event");

        assert_eq!(
            sse_event_data(&event).expect("data").as_deref(),
            Some("{\"available\":true}")
        );
        assert!(buffer.is_empty());
    }

    #[test]
    fn extracts_crlf_sse_events() {
        let mut buffer = b"event: live_state\r\ndata: first\r\ndata: second\r\n\r\nrest".to_vec();

        let event = take_sse_event(&mut buffer).expect("event");

        assert_eq!(
            sse_event_data(&event).expect("data").as_deref(),
            Some("first\nsecond")
        );
        assert_eq!(buffer, b"rest");
    }

    #[test]
    fn ignores_keepalive_sse_comments() {
        let event = b": keep-alive\n\n";

        assert!(sse_event_data(event).expect("data").is_none());
    }
}
