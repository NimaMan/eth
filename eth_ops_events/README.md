# eth_ops_events

Shared operational event contract for the ETH live pipeline.

This crate does not replace normal `tracing` logs. It gives the pipeline a
small shared schema for the records that dashboards and bottleneck tools should
trust:

- `PipelineHealth`: component liveness, lag, dependency status, and heartbeat
  metrics.
- `PipelineIssue`: structured runtime issues with severity, impact, stable
  dedupe key, chain coordinates, and component context.
- `PipelineBottleneckSample`: slow path samples with duration, threshold, work
  units, and timing breakdowns.

Severity and impact are separate on purpose. A pool-local simulator warning can
be `severity=warn` and `impact=pool_local` without blocking the live tracker or
trading gate. Fatal service failures should set `fatal=true` and use
`impact=service_down` or `impact=trading_blocked`.

Initial sinks:

- `JsonlOpsEventSink`: writes `pipeline_issues.jsonl`,
  `pipeline_health.jsonl`, and `pipeline_bottlenecks.jsonl`.
- `TracingOpsEventSink`: mirrors structured records through `tracing`.
- `MultiOpsEventSink`: fans out to several sinks.

`eth_chain_server` initializes the global sink for its run directory. Other ETH
services should use this crate directly and either initialize their own local
sink or emit through the process-global sink when hosted inside the token server.
