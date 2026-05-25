use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use eth_live_feed::{LiveTokenEvent, LiveTokenReader};
use futures_util::stream;
use futures_util::Stream;
use tokio::sync::broadcast;
use warp::sse::Event;

use crate::http::ServerState;
use crate::ranges::RangeIndexJob;
use crate::read_models as views;

pub fn progress_stream(
    run: Arc<RangeIndexJob>,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    stream::unfold(run, |run| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let progress = views::run::progress(&run).await;
        let payload = serde_json::to_string(&progress).unwrap_or_else(|_| "{}".to_string());
        Some((Ok(Event::default().event("progress").data(payload)), run))
    })
}

struct LiveStateFrameStream {
    state: ServerState,
    updates: broadcast::Receiver<LiveTokenEvent>,
    emitted_initial: bool,
}

pub fn live_state_frame_stream(
    state: ServerState,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    let updates = state.live_tracker.subscribe();
    stream::unfold(
        LiveStateFrameStream {
            state,
            updates,
            emitted_initial: false,
        },
        |mut stream| async move {
            if !stream.emitted_initial {
                stream.emitted_initial = true;
                if let Some(event) = latest_live_state_event(&stream.state) {
                    return Some((Ok(event), stream));
                }
            }

            loop {
                match stream.updates.recv().await {
                    Ok(LiveTokenEvent::BlockApplied { .. }) => {
                        if let Some(event) = latest_live_state_event(&stream.state) {
                            return Some((Ok(event), stream));
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if let Some(event) = latest_live_state_event(&stream.state) {
                            return Some((Ok(event), stream));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    )
}

fn latest_live_state_event(state: &ServerState) -> Option<Event> {
    let response = views::live::latest_live_state_frame(&state.recent_live_state_frames);
    let block_id = response
        .state
        .as_ref()
        .map(|state| state.frame.header.number.to_string())?;
    let payload = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    Some(
        Event::default()
            .event("live_state")
            .id(block_id)
            .data(payload),
    )
}
