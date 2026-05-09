use std::time::{Duration, Instant};

use crate::ProcessedBlock;
use eyre::{Result, WrapErr};
use reth_chain_query::{BlockHeader, RethQueryProvider};

use super::store::{ProcessedBlockDiskCacheKey, ProcessedBlockDiskCacheStore};

const DEFAULT_MAX_PARALLEL_CACHE_READS: usize = 8;
const HEADER_FETCH_RETRY_ATTEMPTS: usize = 5;
const HEADER_FETCH_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub struct ProcessedBlockDiskCacheReader {
    store: ProcessedBlockDiskCacheStore,
}

#[derive(Debug)]
pub struct ProcessedBlockDiskCacheRead {
    pub key: ProcessedBlockDiskCacheKey,
    pub block: Option<ProcessedBlock>,
    pub read_ms: f64,
}

#[derive(Debug, Clone)]
pub struct ProcessedBlockDiskCacheRangePlan {
    pub keys: Vec<ProcessedBlockDiskCacheKey>,
    pub missing_keys: Vec<ProcessedBlockDiskCacheKey>,
}

impl ProcessedBlockDiskCacheReader {
    pub fn new(store: ProcessedBlockDiskCacheStore) -> Self {
        Self { store }
    }

    pub fn get(&self, key: &ProcessedBlockDiskCacheKey) -> Result<Option<ProcessedBlock>> {
        self.store.get(key)
    }

    pub fn missing_keys(
        &self,
        keys: &[ProcessedBlockDiskCacheKey],
    ) -> Vec<ProcessedBlockDiskCacheKey> {
        keys.iter()
            .filter(|key| !self.store.contains(key))
            .cloned()
            .collect()
    }

    pub async fn plan_range(
        &self,
        provider: &RethQueryProvider,
        start_block: u64,
        end_block: u64,
    ) -> Result<ProcessedBlockDiskCacheRangePlan> {
        if end_block < start_block {
            eyre::bail!("end_block must be greater than or equal to start_block");
        }

        let chain_id = provider.chain_id();
        let mut keys = Vec::with_capacity((end_block - start_block + 1) as usize);
        for block_number in start_block..=end_block {
            let key = match provider.fetch_block_header_only(block_number).await {
                Ok(header) => ProcessedBlockDiskCacheKey::new(chain_id, &header),
                Err(initial_error) if is_missing_header_error(&initial_error) => {
                    match self
                        .store
                        .cached_key_for_block_number(chain_id, block_number)?
                    {
                        Some(key) => {
                            tracing::debug!(
                                block_number,
                                "planned processed block disk cache key from disk cache"
                            );
                            key
                        }
                        None => {
                            let header =
                                fetch_header_with_tip_retry(provider, block_number).await?;
                            ProcessedBlockDiskCacheKey::new(chain_id, &header)
                        }
                    }
                }
                Err(error) => {
                    return Err(error).wrap_err_with(|| {
                        format!(
                            "failed to fetch processed block disk cache header for block={block_number}"
                        )
                    })
                }
            };
            keys.push(key);
        }

        let missing_keys = self.missing_keys(&keys);
        Ok(ProcessedBlockDiskCacheRangePlan { keys, missing_keys })
    }

    pub fn get_many_parallel(
        &self,
        keys: &[ProcessedBlockDiskCacheKey],
    ) -> Result<Vec<ProcessedBlockDiskCacheRead>> {
        let max_parallelism = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .min(DEFAULT_MAX_PARALLEL_CACHE_READS);
        self.get_many_parallel_with_limit(keys, max_parallelism)
    }

    pub fn get_many_parallel_with_limit(
        &self,
        keys: &[ProcessedBlockDiskCacheKey],
        max_parallelism: usize,
    ) -> Result<Vec<ProcessedBlockDiskCacheRead>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let workers = max_parallelism.max(1).min(keys.len());
        let chunk_size = keys.len().div_ceil(workers);
        let mut output: Vec<Option<ProcessedBlockDiskCacheRead>> =
            std::iter::repeat_with(|| None).take(keys.len()).collect();

        let join_result: Result<()> = std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for (chunk_index, chunk) in keys.chunks(chunk_size).enumerate() {
                let store = self.store.clone();
                let start_index = chunk_index * chunk_size;
                handles.push(scope.spawn(
                    move || -> Result<Vec<(usize, ProcessedBlockDiskCacheRead)>> {
                        let mut reads = Vec::with_capacity(chunk.len());
                        for (offset, key) in chunk.iter().cloned().enumerate() {
                            let read_started = Instant::now();
                            let block = store.get(&key).wrap_err_with(|| {
                                format!(
                                    "failed to read processed block disk cache block={} hash={:?}",
                                    key.block_number, key.block_hash
                                )
                            })?;
                            reads.push((
                                start_index + offset,
                                ProcessedBlockDiskCacheRead {
                                    key,
                                    block,
                                    read_ms: read_started.elapsed().as_secs_f64() * 1000.0,
                                },
                            ));
                        }
                        Ok(reads)
                    },
                ));
            }

            for handle in handles {
                let reads = handle.join().map_err(|_| {
                    eyre::eyre!("processed block disk cache reader worker panicked")
                })??;
                for (index, read) in reads {
                    output[index] = Some(read);
                }
            }
            Ok(())
        });
        join_result?;

        output
            .into_iter()
            .map(|read| {
                read.ok_or_else(|| eyre::eyre!("processed block disk cache read was not filled"))
            })
            .collect()
    }
}

async fn fetch_header_with_tip_retry(
    provider: &RethQueryProvider,
    block_number: u64,
) -> Result<BlockHeader> {
    let mut last_error = None;
    for attempt in 0..=HEADER_FETCH_RETRY_ATTEMPTS {
        match provider.fetch_block_header_only(block_number).await {
            Ok(header) => {
                if attempt > 0 {
                    tracing::info!(
                        block_number,
                        attempts = attempt + 1,
                        "resolved processed block disk cache header after retry"
                    );
                }
                return Ok(header);
            }
            Err(error)
                if is_missing_header_error(&error) && attempt < HEADER_FETCH_RETRY_ATTEMPTS =>
            {
                last_error = Some(error);
                tokio::time::sleep(HEADER_FETCH_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| eyre::eyre!("No header for block {block_number}")))
}

fn is_missing_header_error(error: &eyre::Report) -> bool {
    error.to_string().contains("No header for block")
}

impl ProcessedBlockDiskCacheRangePlan {
    pub fn is_complete(&self) -> bool {
        self.missing_keys.is_empty()
    }

    pub fn missing_block_numbers(&self) -> Vec<u64> {
        self.missing_keys
            .iter()
            .map(|key| key.block_number)
            .collect()
    }
}
