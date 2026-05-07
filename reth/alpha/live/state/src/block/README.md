# block

Confirmed block-ready notifications.

This module intentionally does not model processed blocks. The canonical processed block type is `tx_processor::ProcessedBlock`; live state only carries lightweight block-ready metadata for consumers that need to know what changed.
