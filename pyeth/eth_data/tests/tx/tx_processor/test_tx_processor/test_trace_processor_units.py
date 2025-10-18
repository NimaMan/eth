from eth_data.tx_processor.tx_trace_processor import TransactionTraceProcessor
from eth_data.tx_processor.data_models.trace_models import InternalTransaction


def _hex(value: int) -> str:
    return hex(value)


def test_process_trace_simple_value_transfer(w3):
    processor = TransactionTraceProcessor(w3)
    trace = {
        "type": "CALL",
        "from": "0x1111111111111111111111111111111111111111",
        "to": "0x2222222222222222222222222222222222222222",
        "value": _hex(10**18),
        "gas": _hex(21_000),
        "gasUsed": _hex(21_000),
        "callType": "call",
        "input": "0x",
        "output": "0x",
        "calls": [],
    }

    internals = processor.process_trace(trace)
    assert internals == [
        InternalTransaction(
            from_address=w3.to_checksum_address(trace["from"]),
            to_address=w3.to_checksum_address(trace["to"]),
            value=10**18,
            gas=21_000,
            gas_used=21_000,
            depth=0,
            trace_type="CALL",
            call_type="call",
            error=None,
        )
    ]


def test_process_trace_contract_creation_result_address(w3):
    processor = TransactionTraceProcessor(w3)
    created = "0x3333333333333333333333333333333333333333"
    trace = {
        "type": "CREATE",
        "from": "0x1111111111111111111111111111111111111111",
        "to": None,
        "value": _hex(0),
        "gas": _hex(500_000),
        "gasUsed": _hex(250_000),
        "callType": None,
        "input": "0x60006000",
        "result": {"address": created},
        "calls": [],
    }

    internals = processor.process_trace(trace)
    assert internals[0].to_address == w3.to_checksum_address(created)


def test_process_trace_contract_creation_receipt_fallback(w3):
    processor = TransactionTraceProcessor(w3)
    receipt_address = "0x4444444444444444444444444444444444444444"
    trace = {
        "type": "CREATE",
        "from": "0x1111111111111111111111111111111111111111",
        "to": None,
        "value": _hex(0),
        "gas": _hex(500_000),
        "gasUsed": _hex(250_000),
        "callType": None,
        "input": "0x60006000",
        "calls": [],
    }

    internals = processor.process_trace(
        trace,
        receipt_contract_address=receipt_address,
    )
    assert internals[0].to_address == w3.to_checksum_address(receipt_address)


def test_process_trace_failed_parent_includes_children(w3):
    processor = TransactionTraceProcessor(w3)
    child_call = {
        "type": "STATICCALL",
        "from": "0x2222222222222222222222222222222222222222",
        "to": "0x3333333333333333333333333333333333333333",
        "value": _hex(0),
        "gas": _hex(10_000),
        "gasUsed": _hex(1_000),
        "callType": "staticcall",
        "input": "0x",
        "output": "0x",
        "calls": [],
    }
    root_trace = {
        "type": "CALL",
        "from": "0x1111111111111111111111111111111111111111",
        "to": "0x2222222222222222222222222222222222222222",
        "value": _hex(0),
        "gas": _hex(50_000),
        "gasUsed": _hex(40_000),
        "callType": "call",
        "input": "0x",
        "output": "0x",
        "error": "execution reverted",
        "calls": [child_call],
    }

    internals = processor.process_trace(root_trace)
    assert len(internals) == 2
    root, child = internals
    assert root.error == "execution reverted"
    assert child.trace_type == "STATICCALL"
    assert child.error is None
