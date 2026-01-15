use std::{path::PathBuf, sync::Arc};

use eyre::Result;
use reth_chain_query::RethQueryProvider;

use crate::live_pipeline::{
    block_logger::BlockProcessingLogger,
    block_notifier::RedisBlockNotifier,
    block_snapshot::build_live_block_snapshot,
    live_block_processor::{LiveBlockProcessor, LiveBlockProcessorConfig},
    redis_block_publisher::RedisBlockPublisher,
};

/// High-level service that wires the live block processor with storage + notifications.
pub struct LiveBlockService {
    processor: LiveBlockProcessor,
    publisher: Option<RedisBlockPublisher>,
    notifier: Option<RedisBlockNotifier>,
    logger: Option<BlockProcessingLogger>,
}

impl LiveBlockService {
    pub async fn new(
        provider: Arc<RethQueryProvider>,
        processor_config: LiveBlockProcessorConfig,
        redis_url: Option<String>,
        notifier_channel: Option<String>,
        log_path: Option<PathBuf>,
    ) -> Result<Self> {
        let processor = LiveBlockProcessor::connect(provider, processor_config).await?;
        let publisher = match redis_url.as_ref() {
            Some(url) => Some(RedisBlockPublisher::new(url, None)?),
            None => None,
        };
        let notifier = match (redis_url.as_ref(), notifier_channel.as_ref()) {
            (Some(url), Some(channel)) => Some(RedisBlockNotifier::new(url, channel.clone())?),
            _ => None,
        };
        let logger_path = log_path.unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("logs")
                .join("block_processor")
                .join("live_block_processor.log")
        });
        let logger = Some(BlockProcessingLogger::new(logger_path)?);

        Ok(Self {
            processor,
            publisher,
            notifier,
            logger,
        })
    }

    pub async fn run(self) -> Result<()> {
        self.run_internal(None).await
    }

    pub async fn run_for_blocks(self, limit: usize) -> Result<()> {
        self.run_internal(Some(limit)).await
    }

    async fn run_internal(mut self, limit: Option<usize>) -> Result<()> {
        let mut processed_count: usize = 0;
        loop {
            let processed = self.processor.next_processed_block().await?;

            if let Some(publisher) = &self.publisher {
                let snapshot = build_live_block_snapshot(&processed.processed_block)?;
                publisher.publish_snapshot(&snapshot).await?;
            }

            if let Some(notifier) = &self.notifier {
                notifier
                    .notify_block_processed(processed.execution_info.block_number)
                    .await?;
            }

            if let Some(logger) = &self.logger {
                logger.log_block(&processed)?;
            }

            processed_count += 1;
            if let Some(max) = limit {
                if processed_count >= max {
                    break;
                }
            }
        }
        Ok(())
    }
}
