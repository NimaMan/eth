import numpy as np
from typing import Dict, Any, List
from web3 import Web3
from eth_block_processor.data_models.trace_models import InternalTransaction


class TransactionTraceProcessor:
    def __init__(self, w3: Web3):
        self.w3 = w3
        
    def process_trace(self, trace: Dict[str, Any], depth: int = 0, parent_failed: bool = False):
        """Process transaction trace to extract meaningful internal transactions.
        Includes:
        1. ETH transfers with non-zero value
        2. Contract creations (CREATE/CREATE2)
        3. Failed transactions and their children
        4. Initial transaction (depth == 0)
        5. STATICCALL operations in failed transaction chains
        """
        internal_transactions = []
        error = trace.get('error', None)
        is_failed = error is not None or parent_failed
        
        if trace['type'] in ['CALL', 'STATICCALL', 'DELEGATECALL', 'CREATE', 'CREATE2']:
            value = int(trace.get('value', '0'), 16)
            
            # Include if:
            # 1. It's the initial transaction, or
            # 2. It's a contract creation, or
            # 3. Has non-zero value, or
            # 4. Has an error, or
            # 5. Parent transaction failed
            if (depth == 0 or 
                trace['type'] in ['CREATE', 'CREATE2'] or 
                value > 0 or 
                is_failed):
                
                to_address = None
                if trace['type'] in ['CREATE', 'CREATE2']:
                    to_address = trace.get('result', {}).get('address') if 'error' not in trace else None
                else:
                    to_address = trace.get('to')

                if trace.get("from"):
                    internal_transactions.append(
                        InternalTransaction(
                            from_address=self.w3.to_checksum_address(trace['from']),
                            to_address=self.w3.to_checksum_address(to_address) if to_address and self.w3.is_address(to_address) else None,
                            value=float(self.w3.from_wei(value, 'ether')),
                            depth=depth,
                            type=trace['type'],
                            gas=int(trace.get('gas', 0), 16),
                            gas_used=int(trace.get('gasUsed', 0), 16),
                            error=error,
                        )
                    )
        
        # Recursively process subcalls, passing down the failure state
        for call in trace.get('calls', []):
            internal_transactions.extend(self.process_trace(call, depth + 1, is_failed))
        
        return internal_transactions