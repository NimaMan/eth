use alloy_rpc_types_trace::geth::{GethTrace, TraceResult};
use std::time::Duration;

pub(super) fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

pub(super) fn trace_result_node_count(traces: &[TraceResult]) -> usize {
    traces.iter().map(trace_nodes).sum()
}

pub(super) fn trace_result_error_count(traces: &[TraceResult]) -> usize {
    traces
        .iter()
        .filter(|trace| matches!(trace, TraceResult::Error { .. }))
        .count()
}

fn trace_nodes(trace: &TraceResult) -> usize {
    match trace {
        TraceResult::Success { result, .. } => geth_trace_node_count(result),
        TraceResult::Error { .. } => 0,
    }
}

pub(super) fn geth_trace_node_count(trace: &GethTrace) -> usize {
    match trace {
        GethTrace::CallTracer(frame) => alloy_call_frame_nodes(frame),
        GethTrace::FlatCallTracer(frames) => frames.len(),
        _ => 0,
    }
}

pub(super) fn alloy_call_frame_nodes(frame: &alloy_rpc_types_trace::geth::CallFrame) -> usize {
    1 + frame
        .calls
        .iter()
        .map(alloy_call_frame_nodes)
        .sum::<usize>()
}
