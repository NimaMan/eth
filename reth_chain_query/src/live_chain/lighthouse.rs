use std::pin::Pin;

use chrono::Utc;
use eventsource_stream::{Event, EventStreamError, Eventsource};
use futures::{Stream, StreamExt};
use reqwest::Client;

use crate::live_chain::models::{HeadObservation, LighthouseHeadPayload};

type StreamError = EventStreamError<reqwest::Error>;

pub struct LighthouseHeadListener {
    stream: Pin<Box<dyn Stream<Item = Result<Event, StreamError>> + Send>>,
    _client: Client,
    pub endpoint: String,
}

impl LighthouseHeadListener {
    pub async fn connect(endpoint: &str) -> eyre::Result<Self> {
        let client = Client::builder().build()?;
        let response = client.get(endpoint).send().await?.error_for_status()?;
        let stream = response.bytes_stream().eventsource();

        Ok(Self {
            stream: Box::pin(stream),
            _client: client,
            endpoint: endpoint.to_owned(),
        })
    }

    pub async fn next_observation(&mut self) -> eyre::Result<HeadObservation> {
        while let Some(event) = self.stream.next().await {
            let event = event?;
            if event.event.as_str() != "head" {
                continue;
            }
            let payload: LighthouseHeadPayload = serde_json::from_str(&event.data)?;
            return payload.to_observation(Utc::now());
        }
        Err(eyre::eyre!(
            "head stream from {} terminated unexpectedly",
            self.endpoint
        ))
    }
}
