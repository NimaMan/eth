import numpy as np
from typing import Dict, Any, List
from web3 import Web3
from eth_block_processor.data_models.trace_models import InternalTransaction


class TransactionTraceAnalyzer:
    def __init__(self, w3: Web3):
        self.w3 = w3
        
    def process_trace(self, trace: Dict[str, Any], depth: int = 0):
        internal_transactions = []
        # Process all internal transactions
        if trace['type'] in ['CALL', 'DELEGATECALL', 'STATICCALL', 'CREATE', 'CREATE2']:
            value = int(trace.get('value', '0'), 16)
            
            # Handle different call types
            if trace['type'] in ['CREATE', 'CREATE2']:
                to_address = trace.get('result', {}).get('address') if 'error' not in trace else None
            elif trace['type'] == 'DELEGATECALL':
                # For DELEGATECALL, use the original caller's address
                to_address = trace.get('to')  # This might need parent context
            else:
                # For normal calls, use the 'to' address
                to_address = trace.get('to')

            if trace.get("from"):
                internal_transactions.append(
                    InternalTransaction(
                        from_address=self.w3.to_checksum_address(trace['from']),
                        to_address=self.w3.to_checksum_address(to_address) if self.w3.is_address(to_address) else None,
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

