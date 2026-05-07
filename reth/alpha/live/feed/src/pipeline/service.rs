use eth_live_state::{
    BlockReadyNotification, LiveStateWriter, SnapshotWriteOptions, TokenSnapshot,
};

use crate::{
    BlockProcessedEvent, BlockTokenProcessor, LiveFeedBlockInput, LiveFeedEvent, LiveFeedEventSink,
    Result,
};

#[derive(Clone, Debug)]
pub struct LiveFeedPipeline<P, W, S> {
    block_token_processor: P,
    live_state_writer: W,
    event_sink: S,
    token_write_options: SnapshotWriteOptions,
}

impl<P, W, S> LiveFeedPipeline<P, W, S> {
    pub fn new(block_token_processor: P, live_state_writer: W, event_sink: S) -> Self {
        Self {
            block_token_processor,
            live_state_writer,
            event_sink,
            token_write_options: SnapshotWriteOptions::default(),
        }
    }

    pub fn with_token_write_options(mut self, options: SnapshotWriteOptions) -> Self {
        self.token_write_options = options;
        self
    }
}

impl<P, W, S> LiveFeedPipeline<P, W, S>
where
    P: BlockTokenProcessor,
    W: LiveStateWriter,
    S: LiveFeedEventSink,
{
    pub async fn process_block(&self, input: LiveFeedBlockInput) -> Result<BlockProcessedEvent> {
        let block = input.block;
        let token_update = self
            .block_token_processor
            .process_block_tokens(&block)
            .await?;
        let updated_tokens = token_update.updated_token_addresses();
        let removed_tokens = token_update.removed_tokens.clone();
        let chain_state_available = input.chain_state.is_some();

        if let Some(chain_state) = input.chain_state {
            self.live_state_writer
                .write_chain_state_snapshot(chain_state)
                .await?;
        }

        for token in token_update.updated_tokens {
            self.write_token(token).await?;
        }
        for token_address in &removed_tokens {
            self.live_state_writer.delete_token(*token_address).await?;
        }

        self.live_state_writer
            .mark_block_ready(BlockReadyNotification::new(
                block.header.number,
                block.header.hash,
                block.transactions.len(),
                updated_tokens.len(),
            ))
            .await?;

        let event = BlockProcessedEvent {
            block_number: block.header.number,
            block_hash: block.header.hash,
            parent_hash: block.header.parent_hash,
            processed_transaction_count: block.transactions.len(),
            updated_tokens,
            removed_tokens,
            chain_state_available,
        };
        self.event_sink
            .publish(LiveFeedEvent::BlockProcessed(event.clone()))
            .await?;
        Ok(event)
    }

    async fn write_token(&self, token: TokenSnapshot) -> Result<()> {
        self.live_state_writer
            .write_token(token, self.token_write_options.clone())
            .await?;
        Ok(())
    }
}
