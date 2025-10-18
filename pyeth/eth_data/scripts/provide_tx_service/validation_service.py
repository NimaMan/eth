#!/usr/bin/env python3
"""
Transaction Validation Service for Rust Integration

A FastAPI service that exposes the Python transaction processor
for validation and comparison with Rust implementations.

Usage:
    python validation_service.py

API Endpoints:
    POST /validate/transaction/{tx_hash} - Process single transaction
    POST /validate/batch - Process multiple transactions
    GET /health - Service health check
"""

import asyncio
import logging
import sys
import traceback
from pathlib import Path
from typing import List, Dict, Any, Optional
from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.responses import JSONResponse
from pydantic import BaseModel, Field
from web3 import Web3
import uvicorn
import orjson

# Add parent package to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_batch_processor import TransactionBatchProcessor
from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction

# Configure logging directly since we may not have logger utils
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("validation_service")

# Initialize FastAPI app
app = FastAPI(
    title="Transaction Validation Service",
    description="Python transaction processor for Rust validation",
    version="1.0.0"
)

# Global Web3 connection
w3: Optional[Web3] = None
tx_processor: Optional[TransactionProcessor] = None
tx_data_fetcher: Optional[TransactionDataFetcher] = None
batch_processor: Optional[TransactionBatchProcessor] = None

# Request/Response Models
class ValidationRequest(BaseModel):
    tx_hash: str = Field(..., description="Transaction hash to process")
    include_state_changes: bool = Field(default=True, description="Calculate state changes")
    include_trace: bool = Field(default=True, description="Include internal transactions")

class BatchValidationRequest(BaseModel):
    tx_hashes: List[str] = Field(..., description="List of transaction hashes")
    include_state_changes: bool = Field(default=True, description="Calculate state changes")
    include_trace: bool = Field(default=True, description="Include internal transactions")

class ValidationResponse(BaseModel):
    success: bool
    tx_hash: str
    processed_transaction: Optional[Dict[str, Any]] = None
    error: Optional[str] = None
    processing_time_ms: float

class BatchValidationResponse(BaseModel):
    success: bool
    total_count: int
    successful_count: int
    failed_count: int
    results: List[ValidationResponse]
    total_processing_time_ms: float

@app.on_event("startup")
async def startup_event():
    """Initialize Web3 connection and processors on startup"""
    global w3, tx_processor, tx_data_fetcher, batch_processor
    
    try:
        # Connect to local Reth node
        w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        
        if not w3.is_connected():
            raise Exception("Failed to connect to Ethereum node at http://127.0.0.1:8545")
        
        # Initialize processors
        tx_processor = TransactionProcessor(
            w3=w3, 
            calculate_address_balance_changes=True,
        )
        tx_data_fetcher = TransactionDataFetcher(w3)
        batch_processor = TransactionBatchProcessor(
            w3=w3,
            calculate_address_balance_changes=True,
            logger=logger
        )
        
        logger.info(f"✅ Connected to Ethereum node: {w3.client_version}")
        logger.info(f"✅ Latest block: {w3.eth.get_block('latest')['number']}")
        
    except Exception as e:
        logger.error(f"❌ Failed to initialize service: {e}")
        raise

def convert_address_balance_changes_for_rust(changes: Dict[str, Any]) -> Dict[str, Any]:
    """Convert numeric values in address balance changes to strings for Rust compatibility."""
    if not changes:
        return {}

    converted = {}
    for address, change in changes.items():
        converted[address] = {
            "eth_net": str(change.get("eth_net", 0)),
            "token_net": {},
            "movements": {}
        }

        # Convert token_net values to strings
        for token, amount in change.get("token_net", {}).items():
            converted[address]["token_net"][token] = str(amount)

        # Convert movements - deep copy with numeric conversion
        movements = change.get("movements", {})
        if movements:
            converted[address]["movements"] = {
                "tokens": {},
                "denom": {}
            }
            
            # Convert token movements
            if "tokens" in movements:
                for token_addr, token_movements in movements["tokens"].items():
                    converted[address]["movements"]["tokens"][token_addr] = {
                        "in": {k: str(v) for k, v in token_movements.get("in", {}).items()},
                        "out": {k: str(v) for k, v in token_movements.get("out", {}).items()}
                    }
            
            # Convert denom movements
            if "denom" in movements:
                converted[address]["movements"]["denom"] = {
                    "in": {k: str(v) for k, v in movements["denom"].get("in", {}).items()},
                    "out": {k: str(v) for k, v in movements["denom"].get("out", {}).items()}
                }
    
    return converted

def serialize_processed_transaction(ptx: ProcessedTransaction) -> Dict[str, Any]:
    """Convert ProcessedTransaction to JSON-serializable dict"""
    try:
        # Custom serialization for dataclass with sets and complex types
        result = {
            # Core transaction data
            "hash": ptx.hash,
            "block_number": ptx.block_number,
            "block_timestamp": ptx.block_timestamp,
            "tx_index": ptx.tx_index,
            "from_address": ptx.from_address,
            "to_address": ptx.to_address,
            "contract_address": ptx.contract_address,
            "value": ptx.value,
            "status": ptx.status,
            "nonce": ptx.nonce,
            "input": ptx.input,
            
            # Classification
            "tx_type": ptx.tx_type,
            "actions": ptx.actions,
            
            # Financial data
            "fees": {
                "gas_price": ptx.fees.gas_price,
                "gas_used": ptx.fees.gas_used,
                "tx_fee": ptx.fees.tx_fee
            } if ptx.fees else None,
            "bribe_amount": ptx.bribe_amount,
            
            # Participants
            "unique_addresses": list(ptx.unique_addresses),
            "erc20_contracts": list(ptx.erc20_contracts),
            
            # Event counts (summary for validation)
            "event_counts": {
                "erc20_transfers": len(ptx.erc20_transfers),
                "erc721_transfers": len(ptx.erc721_transfers),
                "erc1155_transfers": len(ptx.erc1155_transfers),
                "internal_transactions": len(ptx.internal_transactions),
                "uniswap_v2_swaps": len(ptx.uniswap_v2_swaps),
                "uniswap_v2_syncs": len(ptx.uniswap_v2_syncs),
                "uniswap_v3_swaps": len(ptx.uniswap_v3_swaps),
                "uniswap_v4_swaps": len(ptx.uniswap_v4_swaps),
                "erc20_approval_events": len(ptx.erc20_approval_events),
                "erc721_approval_events": len(ptx.erc721_approval_events),
                "uniswap_v2_mints": len(ptx.uniswap_v2_mints),
                "uniswap_v2_burns": len(ptx.uniswap_v2_burns),
                "deposit_events": len(ptx.deposit_events),
                "withdraw_events": len(ptx.withdraw_events),
                "permit2_events": len(ptx.permit2_events),
                "trading_enabled_events": len(ptx.trading_enabled_events),
                "trading_disabled_events": len(ptx.trading_disabled_events),
            },
            
            # Detailed events (for deep validation)
            "erc20_transfers": [
                {
                    "token_address": transfer.token_address,
                    "from_address": transfer.from_address,
                    "to_address": transfer.to_address,
                    "amount": str(transfer.amount),  # Convert to string for Rust
                    "log_index": transfer.log_index
                } for transfer in ptx.erc20_transfers
            ],
            
            "internal_transactions": [
                {
                    "from_address": itx.from_address,
                    "to_address": itx.to_address,
                    "value": str(itx.value),  # Convert to string for Rust
                    "trace_type": getattr(itx, 'trace_type', 'call'),
                    "call_type": getattr(itx, 'call_type', 'call')
                } for itx in ptx.internal_transactions
            ],
            
            "uniswap_v2_swaps": [
                {
                    "pair_address": swap.pair_address,
                    "sender": swap.sender,
                    "to": swap.to,
                    "amount0In": str(swap.amount0In),  # Convert to string for Rust
                    "amount1In": str(swap.amount1In),  # Convert to string for Rust
                    "amount0Out": str(swap.amount0Out),  # Convert to string for Rust
                    "amount1Out": str(swap.amount1Out),  # Convert to string for Rust
                    "log_index": swap.log_index
                } for swap in ptx.uniswap_v2_swaps
            ],
            
            "uniswap_v4_swaps": [
                {
                    "pool_manager_address": swap.pool_manager_address,
                    "event_id": swap.event_id.hex() if hasattr(swap.event_id, 'hex') else str(swap.event_id),
                    "sender": swap.sender,
                    "amount0": str(swap.amount0),  # Convert to string for Rust
                    "amount1": str(swap.amount1),  # Convert to string for Rust
                    "sqrt_price_x96": str(swap.sqrt_price_x96),  # Convert to string for Rust
                    "liquidity": str(swap.liquidity),  # Convert to string for Rust
                    "tick": swap.tick,
                    "fee": swap.fee,
                    "log_index": swap.log_index
                } for swap in ptx.uniswap_v4_swaps
            ],
            
            # Address balance changes - convert numeric values to strings for Rust compatibility
            "address_balance_changes": convert_address_balance_changes_for_rust(ptx.address_balance_changes),
        }
        
        return result
        
    except Exception as e:
        logger.error(f"Serialization error: {e}")
        logger.error(f"Traceback: {traceback.format_exc()}")
        raise HTTPException(status_code=500, detail=f"Serialization failed: {str(e)}")

@app.get("/health")
async def health_check():
    """Service health check"""
    try:
        latest_block = w3.eth.get_block('latest')['number']
        return {
            "status": "healthy",
            "node_connected": w3.is_connected(),
            "latest_block": latest_block,
            "node_version": w3.client_version
        }
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"Service unhealthy: {str(e)}")

@app.post("/validate/transaction/{tx_hash}", response_model=ValidationResponse)
async def validate_transaction(tx_hash: str, request: ValidationRequest = None):
    """Process a single transaction for validation"""
    import time
    start_time = time.time()
    
    try:
        logger.info(f"🔍 Processing transaction: {tx_hash}")
        
        # Use request parameters or defaults
        include_state_changes = request.include_state_changes if request else True
        include_trace = request.include_trace if request else True
        
        # Fetch transaction data
        tx_data = tx_data_fetcher.get_transaction_data(
            tx_hash, 
            receipt=True, 
            trace=include_trace,
            state_diff=include_state_changes
        )
        
        if not tx_data.get('transaction'):
            raise HTTPException(status_code=404, detail=f"Transaction {tx_hash} not found")
        
        # Process transaction
        processed_tx = await tx_processor.process_transaction_async(
            tx_data['transaction'],
            tx_data.get('receipt'),
            tx_data.get('trace'),
            state_diff=include_state_changes
        )
        
        # Serialize result
        result_dict = serialize_processed_transaction(processed_tx)
        
        processing_time = (time.time() - start_time) * 1000
        logger.info(f"✅ Processed {tx_hash} in {processing_time:.1f}ms")
        
        return ValidationResponse(
            success=True,
            tx_hash=tx_hash,
            processed_transaction=result_dict,
            processing_time_ms=processing_time
        )
        
    except HTTPException:
        raise
    except Exception as e:
        processing_time = (time.time() - start_time) * 1000
        error_msg = f"Processing failed: {str(e)}"
        logger.error(f"❌ {error_msg}")
        logger.error(f"Traceback: {traceback.format_exc()}")
        
        return ValidationResponse(
            success=False,
            tx_hash=tx_hash,
            error=error_msg,
            processing_time_ms=processing_time
        )

@app.post("/validate/batch", response_model=BatchValidationResponse)
async def validate_batch(request: BatchValidationRequest):
    """Process multiple transactions for validation"""
    import time
    start_time = time.time()
    
    try:
        logger.info(f"🔍 Processing batch of {len(request.tx_hashes)} transactions")
        
        results = []
        successful_count = 0
        failed_count = 0
        
        # Process transactions in parallel using the batch processor
        try:
            processed_txs = await batch_processor.process_transactions_async(
                request.tx_hashes,
                include_trace=request.include_trace,
                state_diff=request.include_state_changes
            )
            
            # Convert results
            for tx_hash in request.tx_hashes:
                tx_start_time = time.time()
                
                if tx_hash in processed_txs:
                    try:
                        result_dict = serialize_processed_transaction(processed_txs[tx_hash])
                        results.append(ValidationResponse(
                            success=True,
                            tx_hash=tx_hash,
                            processed_transaction=result_dict,
                            processing_time_ms=(time.time() - tx_start_time) * 1000
                        ))
                        successful_count += 1
                    except Exception as e:
                        results.append(ValidationResponse(
                            success=False,
                            tx_hash=tx_hash,
                            error=f"Serialization failed: {str(e)}",
                            processing_time_ms=(time.time() - tx_start_time) * 1000
                        ))
                        failed_count += 1
                else:
                    results.append(ValidationResponse(
                        success=False,
                        tx_hash=tx_hash,
                        error="Transaction not found or processing failed",
                        processing_time_ms=(time.time() - tx_start_time) * 1000
                    ))
                    failed_count += 1
            
        except Exception as e:
            logger.error(f"Batch processing failed: {e}")
            raise HTTPException(status_code=500, detail=f"Batch processing failed: {str(e)}")
        
        total_processing_time = (time.time() - start_time) * 1000
        logger.info(f"✅ Processed batch in {total_processing_time:.1f}ms: {successful_count} success, {failed_count} failed")
        
        return BatchValidationResponse(
            success=True,
            total_count=len(request.tx_hashes),
            successful_count=successful_count,
            failed_count=failed_count,
            results=results,
            total_processing_time_ms=total_processing_time
        )
        
    except HTTPException:
        raise
    except Exception as e:
        total_processing_time = (time.time() - start_time) * 1000
        error_msg = f"Batch processing failed: {str(e)}"
        logger.error(f"❌ {error_msg}")
        logger.error(f"Traceback: {traceback.format_exc()}")
        
        return BatchValidationResponse(
            success=False,
            total_count=len(request.tx_hashes),
            successful_count=0,
            failed_count=len(request.tx_hashes),
            results=[],
            total_processing_time_ms=total_processing_time
        )

@app.get("/validate/transaction/{tx_hash}/summary")
async def get_transaction_summary(tx_hash: str):
    """Get quick transaction summary for lightweight validation"""
    try:
        # Faster endpoint that only returns key metrics
        tx_data = tx_data_fetcher.get_transaction_data(tx_hash, receipt=True, trace=False)
        
        if not tx_data.get('transaction'):
            raise HTTPException(status_code=404, detail=f"Transaction {tx_hash} not found")
        
        tx = tx_data['transaction']
        receipt = tx_data['receipt']
        
        return {
            "tx_hash": tx_hash,
            "block_number": tx.get('blockNumber', 0),
            "from_address": tx.get('from'),
            "to_address": tx.get('to'),
            "value": tx.get('value', 0),
            "gas_used": receipt.get('gasUsed', 0) if receipt else 0,
            "status": receipt.get('status', 0) if receipt else 0,
            "log_count": len(receipt.get('logs', [])) if receipt else 0
        }
        
    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Summary failed: {str(e)}")

if __name__ == "__main__":
    # Run the service on port 18000 to avoid conflicts
    uvicorn.run(
        "validation_service:app",
        host="127.0.0.1",
        port=18000,
        reload=False,  # Disable reload for production service
        log_level="info"
    )
