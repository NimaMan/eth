import numpy as np
from typing import Dict, Any, List
from web3 import Web3
from eth_block_processor.data_models.trace_models import InternalTransaction


class TransactionTraceAnalyzer:
    def __init__(self, w3: Web3):
        self.w3 = w3
        
    def process_trace(self, trace: Dict[str, Any], depth: int = 0):
        internal_transactions = []
        # Process internal transaction, skipping revert
        if trace['type'] in ['CALL', 'DELEGATECALL', 'STATICCALL', 'CREATE', 'CREATE2']:
            value = int(trace.get('value', '0'), 16)
            if value > 0 or trace['type'] in ['CREATE', 'CREATE2']:
                internal_transactions.append(
                    InternalTransaction(
                        from_address=trace['from'],
                        to_address=trace.get('to') or trace.get('result', {}).get('address', 'Contract Creation'),
                        value=np.float64(self.w3.from_wei(value, 'ether')),
                        depth=depth,
                        type=trace['type'],
                        #input=trace.get('input', ''),
                        gas=int(trace.get('gas', 0), 16),
                        gas_used=int(trace.get('gasUsed', 0), 16),
                        error=trace.get('error', None),
                    )
                )
            
        # Recursively process subcalls
        for call in trace.get('calls', []):
            internal_transactions.extend(self.process_trace(call, depth + 1))
        return internal_transactions



'''

class TransactionTraceAnalyzer:
    def __init__(self, w3: Web3):
        self.w3 = w3
        
    def process_trace(self, trace: Dict[str, Any], depth: int = 0) -> List[TraceOperation]:
        operations = []
        
        # Process the current trace operation
        op_type = trace['type']
        if op_type in ['CALL', 'DELEGATECALL', 'STATICCALL', 'CREATE', 'CREATE2']:
            value = int(trace.get('value', '0'), 16)
            operations.append(
                InternalTransaction(
                    from_address=trace['from'],
                    to_address=trace.get('to') or trace.get('result', {}).get('address', 'Contract Creation'),
                    value=self.w3.from_wei(value, 'ether'),
                    input=trace.get('input', ''),
                    gas=int(trace.get('gas', 0), 16),
                    gas_used=int(trace.get('gasUsed', 0), 16),
                    type=op_type,
                    error=trace.get('error', None),
                    depth=depth
                )
            )
        elif op_type == 'SELFDESTRUCT':
            operations.append(
                TraceOperation(
                    type=op_type,
                    from_address=trace['from'],
                    to_address=trace.get('to'),
                    value=self.w3.from_wei(int(trace.get('balance', '0'), 16), 'ether'),
                    depth=depth
                )
            )
        elif op_type in ['SSTORE', 'SLOAD']:
            operations.append(
                TraceOperation(
                    type=op_type,
                    address=trace['from'],
                    key=trace.get('key'),
                    value=trace.get('value'),
                    depth=depth
                )
            )
        elif op_type.startswith('LOG'):
            operations.append(
                TraceOperation(
                    type=op_type,
                    address=trace['from'],
                    data=trace.get('data'),
                    topics=trace.get('topics'),
                    depth=depth
                )
            )
        elif op_type in ['RETURN', 'REVERT']:
            operations.append(
                TraceOperation(
                    type=op_type,
                    address=trace['from'],
                    data=trace.get('data'),
                    depth=depth
                )
            )
        
        # Recursively process subcalls
        for call in trace.get('calls', []):
            operations.extend(self.process_trace(call, depth + 1))
        
        return operations
'''