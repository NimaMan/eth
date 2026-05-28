use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream;
use futures_util::Stream;
use warp::sse::Event;

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
