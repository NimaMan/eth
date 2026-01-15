from dataclasses import dataclass, field
from typing import List, Optional
from eth_typing import ChecksumAddress


@dataclass
class TraceCall:
    type: str
    from_address: str
    to_address: str
    value: Optional[int]
    gas: int
    gas_used: int
    input: str
    output: str
    error: Optional[str]
    calls: List['TraceCall'] = field(default_factory=list)


@dataclass
class TransactionTrace:
    trace: TraceCall


@dataclass
class InternalTransaction:
    from_address: ChecksumAddress
    to_address: Optional[ChecksumAddress]
    value: int
    gas: int
    gas_used: int
    depth: int
    trace_type: str
    call_type: Optional[str]
    error: Optional[str]
    

@dataclass
class TraceOperation:
    type: str
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    value: int 
    gas: int
    gas_used: int
    input: str
