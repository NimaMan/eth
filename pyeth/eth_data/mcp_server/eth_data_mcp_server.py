#!/usr/bin/env python3
"""
ETH Data MCP Server
===================

This MCP (Model Context Protocol) server provides Claude Code with direct access to
Ethereum blockchain data processing capabilities using the high-performance Rust
transaction processor backend.

Features:
- Investigate individual transactions by hash
- Analyze all transactions for an address
- Process entire blocks of transactions  
- Trace fund flows across addresses
- 91.5x performance improvement via Rust backend

The server communicates via stdio transport and provides structured JSON responses
with comprehensive transaction details including decoded events, internal transactions,
and state changes.
"""

import sys
import json
import asyncio
from typing import Dict, Any
import traceback
from eth_data.tx_provider.rs_processed_transaction_provider import RustProcessedTransactionProvider
from eth_data.database.db_fetchers.tx_meta_data_fetcher import TxMetaDataFetcher
from eth_data.utils.logger import get_logger


logger = get_logger("eth_data_mcp_server", log_folder="eth_mcp_server", log_level="info")


class EthDataMCPServer:
    """MCP server for Ethereum blockchain data processing and analysis."""
    
    def __init__(self):
        """Initialize the investigation server."""
        self.provider = None
        self.tx_fetcher = None
        self.initialize_providers()
        
    def initialize_providers(self):
        """Initialize data providers."""
        try:
            # Initialize Rust-based transaction provider for high performance
            self.provider = RustProcessedTransactionProvider(
                logger=logger,
                cache_size=1000
            )
            logger.info("Initialized RustProcessedTransactionProvider")
            
            # Initialize transaction metadata fetcher for additional queries
            self.tx_fetcher = TxMetaDataFetcher(logger=logger)
            logger.info("Initialized TxMetaDataFetcher")
            
        except Exception as e:
            logger.error(f"Failed to initialize providers: {e}")
            raise

    async def handle_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """
        Handle incoming MCP requests.
        
        Args:
            request: MCP request object with method and params
            
        Returns:
            MCP response object
        """
        method = request.get("method", "")
        params = request.get("params", {})
        request_id = request.get("id")
        
        try:
            if method == "tools/list":
                return self.list_tools(request_id)
            
            elif method == "tools/call":
                tool_name = params.get("name", "")
                tool_args = params.get("arguments", {})
                
                if tool_name == "get_processed_tx_from_hash":
                    result = await self.get_processed_tx_from_hash(tool_args)
                elif tool_name == "get_processed_txs_for_address":
                    result = await self.get_processed_txs_for_address(tool_args)
                elif tool_name == "process_block":
                    result = await self.process_block(tool_args)
                elif tool_name == "trace_fund_flow":
                    result = await self.trace_fund_flow(tool_args)
                else:
                    raise ValueError(f"Unknown tool: {tool_name}")
                
                return {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": json.dumps(result, indent=2)
                            }
                        ]
                    }
                }
                
            else:
                raise ValueError(f"Unknown method: {method}")
                
        except Exception as e:
            logger.error(f"Error handling request: {e}\n{traceback.format_exc()}")
            return {
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {
                    "code": -32000,
                    "message": str(e)
                }
            }
    
    def list_tools(self, request_id: Any) -> Dict[str, Any]:
        """Return list of available tools."""
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "tools": [
                    {
                        "name": "get_processed_tx_from_hash",
                        "description": "Get processed transaction dict from hash",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "tx_hash": {
                                    "type": "string",
                                    "description": "Transaction hash (0x...)"
                                }
                            },
                            "required": ["tx_hash"]
                        }
                    },
                    {
                        "name": "get_processed_txs_for_address",
                        "description": "Get all processed transactions for an address within a block range",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "address": {
                                    "type": "string",
                                    "description": "Ethereum address (0x...)"
                                },
                                "start_block": {
                                    "type": "integer",
                                    "description": "Starting block number (optional)"
                                },
                                "end_block": {
                                    "type": "integer",
                                    "description": "Ending block number (optional)"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Maximum number of transactions to return (default: 100)"
                                }
                            },
                            "required": ["address"]
                        }
                    },
                    {
                        "name": "process_block",
                        "description": "Process an entire block and get all transactions with full details",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "block_number": {
                                    "type": "integer",
                                    "description": "Block number to process"
                                },
                                "save_to_db": {
                                    "type": "boolean",
                                    "description": "Whether to save transactions to database (default: false)"
                                }
                            },
                            "required": ["block_number"]
                        }
                    },
                    {
                        "name": "trace_fund_flow",
                        "description": "Trace fund flow from a transaction through multiple hops",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "tx_hash": {
                                    "type": "string",
                                    "description": "Starting transaction hash (0x...)"
                                },
                                "depth": {
                                    "type": "integer",
                                    "description": "How many hops to trace (default: 2, max: 5)"
                                }
                            },
                            "required": ["tx_hash"]
                        }
                    }
                ]
            }
        }
    
    async def get_processed_tx_from_hash(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Get a processed transaction from hash.
        
        Args:
            args: Must contain 'tx_hash'
            
        Returns:
            Processed transaction dict
        """
        tx_hash = args.get("tx_hash", "").strip()
        if not tx_hash:
            raise ValueError("tx_hash is required")
        
        # Ensure proper formatting
        if not tx_hash.startswith("0x"):
            tx_hash = "0x" + tx_hash
        
        logger.info(f"Getting processed transaction: {tx_hash}")
        
        # Get processed transaction using Rust backend
        txs = self.provider.get_processed_transactions_from_tx_hashes([tx_hash])
        
        if not txs:
            return {
                "error": f"Transaction {tx_hash} not found or could not be processed"
            }
        
        tx = txs[0]
        
        # Return the processed transaction dict - convert sets to lists for JSON serialization
        tx_dict = tx.to_dict()
        return self._make_json_serializable(tx_dict)
    
    async def get_processed_txs_for_address(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Get all processed transactions for an address.
        
        Args:
            args: Must contain 'address', optionally 'start_block', 'end_block', 'limit'
            
        Returns:
            List of transactions involving the address
        """
        address = args.get("address", "").strip()
        if not address:
            raise ValueError("address is required")
        
        start_block = args.get("start_block")
        end_block = args.get("end_block")
        limit = args.get("limit", 100)
        
        logger.info(f"Investigating address: {address}")
        
        # Get transactions for address
        txs = self.provider.fetch_address_processed_transactions(
            address=address,
            start_block=start_block,
            end_block=end_block
        )
        
        # Limit results
        if len(txs) > limit:
            txs = txs[:limit]
        
        results = {
            "address": address,
            "transaction_count": len(txs),
            "transactions": []
        }
        
        for tx in txs:
            # Use to_dict for complete transaction data
            tx_dict = tx.to_dict()
            
            # Create summary with essential fields
            tx_summary = {
                "hash": tx_dict["hash"],
                "block_number": tx_dict["block_number"],
                "timestamp": tx_dict["block_timestamp"],
                "from": tx_dict["from_address"],
                "to": tx_dict["to_address"],
                "value": str(tx_dict["value"]),
                "type": tx_dict["tx_type"],
                "direction": "out" if tx_dict["from_address"] == address else "in"
            }
            
            # Add token transfers involving the queried address
            transfers = []
            for transfer in tx_dict["erc20_transfers"]:
                if (transfer["from_address"] == address or transfer["to_address"] == address):
                    transfers.append({
                        "token": transfer["token_address"],
                        "from": transfer["from_address"],
                        "to": transfer["to_address"],
                        "amount": str(transfer["amount"])
                    })
            
            if transfers:
                tx_summary["token_transfers"] = transfers
            
            results["transactions"].append(tx_summary)
        
        # Generate address summary
        results["summary"] = self._generate_address_summary(results)
        
        return results
    
    async def process_block(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process an entire block using the Python block processor.
        
        Args:
            args: Must contain 'block_number', optionally 'save_to_db'
            
        Returns:
            All processed transactions in the block
        """
        block_number = args.get("block_number")
        save_to_db = args.get("save_to_db", False)
        
        if not block_number:
            raise ValueError("block_number is required")
        
        logger.info(f"Processing block: {block_number} (index_address_txs: {save_to_db})")
        
        # Use Python block processor for full block processing
        from eth_data.blockchain.block_processor import BlockProcessor
        
        # Initialize block processor
        block_processor = BlockProcessor(
            logger=logger,
            index_address_txs=save_to_db
        )
        
        # Process the block
        try:
            processed_block_result = await block_processor.process_block(block_number)
            
            if not processed_block_result:
                return {
                    "error": f"Block {block_number} not found or has no transactions"
                }

            processed_txs = processed_block_result.transactions
            
            # Format results
            results = {
                "block_number": block_number,
                "transaction_count": len(processed_txs),
                "total_value": 0,
                "total_gas": 0,
                "transactions": [],
                "token_transfers": [],
                "dex_swaps": [],
                "failed_txs": []
            }
            
            for tx in processed_txs:
                tx_dict = tx.to_dict()
                
                results["total_value"] += int(tx_dict["value"])
                results["total_gas"] += int(tx_dict["fees"]["gas_used"])
                
                # Basic transaction info
                tx_data = {
                    "hash": tx_dict["hash"],
                    "from": tx_dict["from_address"],
                    "to": tx_dict["to_address"],
                    "value": str(tx_dict["value"]),
                    "gas_used": tx_dict["fees"]["gas_used"],
                    "type": tx_dict["tx_type"],
                    "status": "success" if tx_dict["status"] == 1 else "failed"
                }
                
                results["transactions"].append(tx_data)
                
                # Track failed transactions
                if tx_dict["status"] != 1:
                    results["failed_txs"].append(tx_dict["hash"])
                
                # Extract token transfers
                for transfer in tx_dict["erc20_transfers"]:
                    results["token_transfers"].append({
                        "tx_hash": tx_dict["hash"],
                        "token": transfer["token_address"],
                        "from": transfer["from_address"],
                        "to": transfer["to_address"],
                        "amount": str(transfer["amount"])
                    })
                        
                # Extract DEX swaps
                for swap in tx_dict["uniswap_v2_swaps"]:
                    results["dex_swaps"].append({
                        "tx_hash": tx_dict["hash"],
                        "pool": swap["pair_address"],
                        "type": "UniswapV2Swap"
                    })
            
            results["total_value"] = str(results["total_value"])
            results["average_gas"] = results["total_gas"] // len(processed_txs) if processed_txs else 0
            results["token_transfer_count"] = len(results["token_transfers"])
            results["dex_swap_count"] = len(results["dex_swaps"])
            results["failed_tx_count"] = len(results["failed_txs"])
            
            # Clean up
            await block_processor.close()
            
            return results
            
        except Exception as e:
            logger.error(f"Error processing block {block_number}: {e}")
            await block_processor.close()
            raise
    
    async def trace_fund_flow(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """
        Trace fund flow from a transaction through multiple hops.
        
        Args:
            args: Must contain 'tx_hash', optionally 'depth'
            
        Returns:
            Fund flow analysis results
        """
        tx_hash = args.get("tx_hash", "").strip()
        depth = min(args.get("depth", 2), 5)  # Max depth of 5
        
        if not tx_hash:
            raise ValueError("tx_hash is required")
        
        logger.info(f"Analyzing fund flow from: {tx_hash}, depth: {depth}")
        
        # Get initial transaction
        txs = self.provider.get_processed_transactions_from_tx_hashes([tx_hash])
        
        if not txs:
            return {
                "error": f"Transaction {tx_hash} not found"
            }
        
        initial_tx = txs[0]
        
        flow_analysis = {
            "initial_transaction": tx_hash,
            "depth": depth,
            "flows": [],
            "addresses_involved": set(),
            "total_value_moved": 0
        }
        
        # Track fund flows
        current_level = [(initial_tx, 0)]
        visited = {tx_hash}
        
        while current_level and depth > 0:
            next_level = []
            
            for tx, level in current_level:
                if level >= depth:
                    continue
                
                tx_dict = tx.to_dict()
                
                # Add to flows
                flow = {
                    "level": level,
                    "tx_hash": tx_dict["hash"],
                    "from": tx_dict["from_address"],
                    "to": tx_dict["to_address"],
                    "value": str(tx_dict["value"]),
                    "timestamp": tx_dict["block_timestamp"]
                }
                
                flow_analysis["flows"].append(flow)
                flow_analysis["addresses_involved"].add(tx_dict["from_address"])
                flow_analysis["addresses_involved"].add(tx_dict["to_address"])
                flow_analysis["total_value_moved"] += int(tx_dict["value"])
                
                # Get subsequent transactions from the recipient
                if tx_dict["to_address"] and level < depth - 1:
                    next_txs = self.provider.fetch_address_processed_transactions(
                        address=tx_dict["to_address"],
                        start_block=tx_dict["block_number"],
                        end_block=tx_dict["block_number"] + 100  # Look ahead 100 blocks
                    )
                    
                    for next_tx in next_txs[:5]:  # Limit to 5 transactions per address
                        next_tx_dict = next_tx.to_dict()
                        if next_tx_dict["hash"] not in visited:
                            visited.add(next_tx_dict["hash"])
                            next_level.append((next_tx, level + 1))
            
            current_level = next_level
        
        # Convert set to list for JSON serialization
        flow_analysis["addresses_involved"] = list(flow_analysis["addresses_involved"])
        flow_analysis["total_value_moved"] = str(flow_analysis["total_value_moved"])
        flow_analysis["unique_addresses"] = len(flow_analysis["addresses_involved"])
        flow_analysis["transaction_count"] = len(flow_analysis["flows"])
        
        return flow_analysis
    
    def _generate_transaction_summary(self, tx_data: Dict[str, Any]) -> Dict[str, Any]:
        """Generate a human-readable summary of a transaction."""
        summary = {
            "description": "",
            "tokens_moved": [],
            "contracts_called": [],
            "key_events": []
        }
        
        # Basic description
        if tx_data["to_address"]:
            if tx_data["value"] != "0":
                summary["description"] = f"Transfer of {tx_data['value']} wei from {tx_data['from_address'][:10]}... to {tx_data['to_address'][:10]}..."
            else:
                summary["description"] = f"Contract interaction from {tx_data['from_address'][:10]}... to {tx_data['to_address'][:10]}..."
        else:
            summary["description"] = f"Contract creation by {tx_data['from_address'][:10]}..."
        
        # Analyze events
        for event in tx_data.get("events", []):
            if event["name"] == "Transfer":
                summary["tokens_moved"].append({
                    "token": event["address"],
                    "amount": event.get("decoded", {}).get("value", "unknown")
                })
            
            if event["name"] in ["Swap", "Sync", "Mint", "Burn"]:
                summary["key_events"].append(event["name"])
        
        # Identify contract interactions
        if tx_data.get("internal_transactions"):
            for internal in tx_data["internal_transactions"]:
                if internal["to"] and internal["to"] not in summary["contracts_called"]:
                    summary["contracts_called"].append(internal["to"])
        
        return summary
    
    def _generate_address_summary(self, address_data: Dict[str, Any]) -> Dict[str, Any]:
        """Generate a summary of an address's activity."""
        summary = {
            "total_transactions": address_data["transaction_count"],
            "incoming": 0,
            "outgoing": 0,
            "total_received": 0,
            "total_sent": 0,
            "unique_interactions": set()
        }
        
        for tx in address_data["transactions"]:
            if tx["direction"] == "in":
                summary["incoming"] += 1
                summary["total_received"] += int(tx["value"])
            else:
                summary["outgoing"] += 1
                summary["total_sent"] += int(tx["value"])
            
            # Track unique addresses interacted with
            if tx["direction"] == "in":
                summary["unique_interactions"].add(tx["from"])
            else:
                summary["unique_interactions"].add(tx["to"])
        
        summary["unique_interactions"] = len(summary["unique_interactions"])
        summary["total_received"] = str(summary["total_received"])
        summary["total_sent"] = str(summary["total_sent"])
        
        return summary
    
    def _make_json_serializable(self, obj):
        """Convert Python objects to JSON-serializable format."""
        if isinstance(obj, set):
            return list(obj)
        elif isinstance(obj, dict):
            return {k: self._make_json_serializable(v) for k, v in obj.items()}
        elif isinstance(obj, list):
            return [self._make_json_serializable(item) for item in obj]
        else:
            return obj
    
    async def run(self):
        """Run the MCP server main loop."""
        logger.info("Starting ETH Data MCP Server")
        
        # Check if stdin is connected
        if sys.stdin.isatty():
            logger.warning("No input stream detected (running in terminal). Waiting for stdin...")
        
        # Main message loop - wait for initialize request from client
        while True:
            try:
                # Read line from stdin - blocks until data available
                line = await asyncio.get_event_loop().run_in_executor(None, sys.stdin.readline)
                if not line:
                    logger.info("EOF received, shutting down")
                    break
                
                line = line.strip()
                if not line:
                    continue
                
                # Parse JSON request
                request = json.loads(line)
                
                # Handle initialize method specially
                if request.get("method") == "initialize":
                    # Send initialize response
                    init_response = {
                        "jsonrpc": "2.0",
                        "id": request.get("id"),
                        "result": {
                            "protocolVersion": "2024-11-05",
                            "capabilities": {
                                "tools": {}
                            },
                            "serverInfo": {
                                "name": "eth-data-mcp-server",
                                "version": "1.0.0"
                            }
                        }
                    }
                    print(json.dumps(init_response), flush=True)
                    continue
                
                # Handle notifications/initialized method (sent after initialization)
                if request.get("method") == "notifications/initialized":
                    # No response needed for notifications
                    logger.info("Received initialization notification from client")
                    continue
                
                # Handle request
                response = await self.handle_request(request)
                
                # Send response
                if response:
                    print(json.dumps(response), flush=True)
                
            except json.JSONDecodeError as e:
                logger.error(f"Invalid JSON: {e}")
                error_response = {
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                }
                print(json.dumps(error_response), flush=True)
                
            except KeyboardInterrupt:
                logger.info("Server interrupted by user")
                break
                
            except Exception as e:
                logger.error(f"Unexpected error: {e}\n{traceback.format_exc()}")
                error_response = {
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": "Internal error"
                    }
                }
                print(json.dumps(error_response), flush=True)
        
        # Cleanup
        if self.provider:
            try:
                self.provider.close()
            except Exception as e:
                logger.warning(f"Error closing provider: {e}")
        if self.tx_fetcher:
            try:
                # TxMetaDataFetcher doesn't have a close method, just set to None
                self.tx_fetcher = None
            except Exception as e:
                logger.warning(f"Error closing TxMetaDataFetcher: {e}")
        logger.info("Server shutdown complete")


async def main():
    """Main entry point."""
    server = EthDataMCPServer()
    await server.run()


if __name__ == "__main__":
    asyncio.run(main())
