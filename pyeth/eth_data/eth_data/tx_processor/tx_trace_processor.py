import numpy as np
from typing import Dict, Any, List
from web3 import Web3
from eth_data.tx_processor.data_models.trace_models import InternalTransaction


class TransactionTraceProcessor:
    def __init__(self, w3: Web3):
        self.w3 = w3
        
    def process_trace(self, trace: Dict[str, Any], depth: int = 0, parent_failed: bool = False, receipt_contract_address: str = None):
        """Process transaction trace to extract meaningful internal transactions.
        Includes:
        1. ETH transfers with non-zero value
        2. Contract creations (CREATE/CREATE2), attempting to resolve the created address
        3. Failed transactions and their children
        4. Initial transaction (depth == 0)
        5. STATICCALL operations in failed transaction chains
        """
        internal_transactions = []
        error = trace.get('error', None)
        is_failed = error is not None or parent_failed
        
        # Define to_address early to handle different types
        to_address = None

        if trace['type'] in ['CALL', 'STATICCALL', 'DELEGATECALL', 'CREATE', 'CREATE2']:
            value = int(trace.get('value', '0'), 16)
            
            # Determine the to_address based on type
            if trace['type'] in ['CREATE', 'CREATE2']:
                # For contract creation, first try the 'to' field which contains the created address
                to_address = trace.get('to')
                
                # If 'to' is None, try to get address from result field if no error
                if to_address is None and 'error' not in trace:
                    to_address = trace.get('result', {}).get('address')
                
                # If still no address AND it's top-level creation, use receipt address
                if to_address is None and depth == 0 and receipt_contract_address:
                    to_address = receipt_contract_address
                
                # Regardless of depth, if this is a CREATE operation, we want to track it
                is_contract_creation = True
            else: # CALL, STATICCALL, DELEGATECALL
                to_address = trace.get('to')
                is_contract_creation = False


            # Include if:
            # 1. It's the initial transaction, or
            # 2. It's a contract creation, or
            # 3. Has non-zero value, or
            # 4. Has an error, or
            # 5. Parent transaction failed
            if (depth == 0 or 
                is_contract_creation or 
                value > 0 or 
                is_failed):
                
                # Ensure from_address exists before proceeding
                from_address_raw = trace.get("from")
                if from_address_raw:
                    from_address_checksum = self.w3.to_checksum_address(from_address_raw)
                    to_address_checksum = self.w3.to_checksum_address(to_address) if to_address and self.w3.is_address(str(to_address)) else None
                    
                    internal_transactions.append(
                        InternalTransaction(
                            from_address=from_address_checksum,
                            to_address=to_address_checksum, # Use the potentially resolved address
                            value=float(self.w3.from_wei(value, 'ether')),
                            depth=depth,
                            type=trace['type'],
                            gas=int(trace.get('gas', 0), 16),
                            gas_used=int(trace.get('gasUsed', 0), 16),
                            error=error,
                        )
                    )
        
        # Recursively process subcalls, passing down the failure state and original receipt address
        for call in trace.get('calls', []):
            internal_transactions.extend(self.process_trace(call, depth + 1, is_failed, receipt_contract_address))
        
        return internal_transactions