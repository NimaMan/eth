# pipeline

Confirmed market-data orchestration.

`MarketDataPipeline` ties one processed block to one token-state update, optional live-state publication, and one downstream event. It is deliberately small so concrete live services can own scheduling, retries, and shutdown.
