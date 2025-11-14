"""
Trade Signal Publisher - Send Trading Signals to eth_kartal

Purpose:
--------
Publishes trading signals from Python strategies to the Rust eth_kartal execution engine
via ZeroMQ. Handles signal tracking, execution confirmations, and failure notifications.

Architecture:
------------
Python Strategy -> TradeSignal -> ZMQ PUB -> eth_kartal (Rust)
                                     ^
                                     |
                              ZMQ SUB (confirmations)

Signal Types:
------------
1. BUY Signal: Purchase tokens with ETH
2. SELL Signal: Sell tokens for ETH  
3. CANCEL Signal: Cancel pending order

Confirmation Types:
------------------
1. SUBMITTED: Signal received by eth_kartal
2. PENDING: Transaction submitted to blockchain
3. CONFIRMED: Transaction mined successfully
4. FAILED: Transaction failed or reverted
"""

import asyncio
import json
import time
import uuid
from typing import Dict, Any, Optional, Callable
from dataclasses import dataclass
from enum import Enum

import zmq
import zmq.asyncio

from eth_portfolio_manager.utils.logger import get_logger
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision


class ExecutionStatus(Enum):
    """Status of trade execution"""
    SUBMITTED = "SUBMITTED"
    PENDING = "PENDING"
    CONFIRMED = "CONFIRMED"
    FAILED = "FAILED"
    CANCELLED = "CANCELLED"


@dataclass
class ExecutionConfirmation:
    """Confirmation message from eth_kartal"""
    signal_id: str
    status: ExecutionStatus
    tx_hash: Optional[str] = None
    block_number: Optional[int] = None
    gas_used: Optional[int] = None
    actual_amount: Optional[float] = None
    error: Optional[str] = None
    timestamp: float = 0


class TradeSignalPublisher:
    """
    Publishes trade signals to eth_kartal and tracks execution status.
    
    Uses ZeroMQ PUB/SUB pattern for bi-directional communication:
    - PUB socket: Send trade signals to eth_kartal
    - SUB socket: Receive execution confirmations
    """
    
    def __init__(
        self,
        pub_endpoint: str = "tcp://*:5559",  # Signals to eth_kartal
        sub_endpoint: str = "tcp://localhost:5560",  # Confirmations from eth_kartal
        logger=None
    ):
        """
        Initialize the trade signal publisher.
        
        Args:
            pub_endpoint: ZMQ PUB endpoint for sending signals
            sub_endpoint: ZMQ SUB endpoint for receiving confirmations
            logger: Optional logger instance
        """
        self.logger = logger or get_logger("trade_signal_publisher")
        self.pub_endpoint = pub_endpoint
        self.sub_endpoint = sub_endpoint
        
        # ZMQ context and sockets
        self._context: Optional[zmq.asyncio.Context] = None
        self._pub_socket: Optional[zmq.asyncio.Socket] = None
        self._sub_socket: Optional[zmq.asyncio.Socket] = None
        
        # Signal tracking
        self._pending_signals: Dict[str, TradeSignal] = {}
        self._signal_callbacks: Dict[str, Callable] = {}
        
        # State
        self._is_running = False
        self._confirmation_task = None
        self._lock = asyncio.Lock()
        
        # Statistics
        self._signals_sent = 0
        self._signals_confirmed = 0
        self._signals_failed = 0
    
    async def start(self):
        """Start the publisher and confirmation listener."""
        if self._is_running:
            return
            
        self.logger.info("Starting trade signal publisher")
        self._is_running = True
        
        try:
            # Initialize ZMQ
            self._context = zmq.asyncio.Context()
            
            # Set up PUB socket for signals
            self._pub_socket = self._context.socket(zmq.PUB)
            self._pub_socket.bind(self.pub_endpoint)
            self.logger.info(f"Signal PUB socket bound to {self.pub_endpoint}")
            
            # Set up SUB socket for confirmations
            self._sub_socket = self._context.socket(zmq.SUB)
            self._sub_socket.connect(self.sub_endpoint)
            self._sub_socket.subscribe(b"")  # Subscribe to all messages
            self.logger.info(f"Confirmation SUB socket connected to {self.sub_endpoint}")
            
            # Start confirmation listener
            self._confirmation_task = asyncio.create_task(self._listen_for_confirmations())
            
        except Exception as e:
            self.logger.error(f"Error starting trade signal publisher: {e}")
            await self.stop()
            raise
    
    async def stop(self):
        """Stop the publisher and cleanup."""
        self.logger.info("Stopping trade signal publisher")
        self._is_running = False
        
        # Cancel confirmation listener
        if self._confirmation_task and not self._confirmation_task.done():
            self._confirmation_task.cancel()
            try:
                await self._confirmation_task
            except asyncio.CancelledError:
                pass
        
        # Close sockets
        if self._pub_socket:
            self._pub_socket.close()
            self._pub_socket = None
            
        if self._sub_socket:
            self._sub_socket.close()
            self._sub_socket = None
            
        # Terminate context
        if self._context:
            self._context.term()
            self._context = None
            
        self.logger.info("Trade signal publisher stopped")
    
    async def publish_signal(
        self, 
        signal: TradeSignal,
        wallet_address: str,
        pool_address: str,
        pool_type: str = "V2",
        max_slippage: float = 0.03,
        deadline_seconds: int = 300,
        confirmation_callback: Optional[Callable] = None
    ) -> str:
        """
        Publish a trade signal to eth_kartal.
        
        Args:
            signal: The trade signal from strategy
            wallet_address: Wallet to execute trade from
            pool_address: DEX pool address
            pool_type: Pool type (V2, V3, V4)
            max_slippage: Maximum acceptable slippage (0.03 = 3%)
            deadline_seconds: Transaction deadline in seconds
            confirmation_callback: Optional callback for execution confirmation
            
        Returns:
            signal_id: Unique identifier for tracking this signal
        """
        if not self._is_running or not self._pub_socket:
            raise RuntimeError("Trade signal publisher not started")
        
        # Generate unique signal ID
        signal_id = str(uuid.uuid4())
        
        # Convert signal to eth_kartal format
        trade_message = {
            "type": "trade_signal",
            "signal_id": signal_id,
            "timestamp": time.time(),
            "signal": {
                "token_address": signal.token_address,
                "action": "BUY" if signal.decision in [TradingDecision.SUBMIT_BUY, TradingDecision.CONFIRM_BUY] else "SELL",
                "amount_eth": signal.quantity if signal.decision in [TradingDecision.SUBMIT_BUY, TradingDecision.CONFIRM_BUY] else None,
                "amount_tokens": signal.quantity if signal.decision in [TradingDecision.SUBMIT_SELL, TradingDecision.CONFIRM_SELL] else None,
                "strategy": signal.strategy_name,
                "wallet": wallet_address,
                "pool_address": pool_address,
                "pool_type": pool_type,
                "max_slippage": max_slippage,
                "deadline": int(time.time()) + deadline_seconds
            }
        }
        
        try:
            # Track signal
            async with self._lock:
                self._pending_signals[signal_id] = signal
                if confirmation_callback:
                    self._signal_callbacks[signal_id] = confirmation_callback
                self._signals_sent += 1
            
            # Serialize and send
            json_data = json.dumps(trade_message)
            await self._pub_socket.send_string(json_data)
            
            self.logger.info(
                f"Published {trade_message['signal']['action']} signal for "
                f"{signal.token_address[:10]}... with {signal.quantity} "
                f"{'ETH' if 'BUY' in trade_message['signal']['action'] else 'tokens'}"
            )
            
            return signal_id
            
        except Exception as e:
            self.logger.error(f"Error publishing trade signal: {e}")
            # Clean up on error
            async with self._lock:
                self._pending_signals.pop(signal_id, None)
                self._signal_callbacks.pop(signal_id, None)
            raise
    
    async def _listen_for_confirmations(self):
        """Listen for execution confirmations from eth_kartal."""
        self.logger.info("Started listening for execution confirmations")
        
        while self._is_running:
            try:
                # Wait for confirmation message
                message = await self._sub_socket.recv_string()
                data = json.loads(message)
                
                if data.get("type") == "execution_confirmation":
                    await self._handle_confirmation(data)
                    
            except asyncio.CancelledError:
                break
            except Exception as e:
                self.logger.error(f"Error processing confirmation: {e}")
                await asyncio.sleep(1)  # Avoid tight loop on error
    
    async def _handle_confirmation(self, data: Dict[str, Any]):
        """Handle execution confirmation from eth_kartal."""
        try:
            # Parse confirmation
            confirmation = ExecutionConfirmation(
                signal_id=data["signal_id"],
                status=ExecutionStatus(data["status"]),
                tx_hash=data.get("tx_hash"),
                block_number=data.get("block_number"),
                gas_used=data.get("gas_used"),
                actual_amount=data.get("actual_amount"),
                error=data.get("error"),
                timestamp=data.get("timestamp", time.time())
            )
            
            async with self._lock:
                # Get original signal
                signal = self._pending_signals.get(confirmation.signal_id)
                if not signal:
                    self.logger.warning(f"Received confirmation for unknown signal: {confirmation.signal_id}")
                    return
                
                # Update statistics
                if confirmation.status == ExecutionStatus.CONFIRMED:
                    self._signals_confirmed += 1
                elif confirmation.status == ExecutionStatus.FAILED:
                    self._signals_failed += 1
                
                # Log confirmation
                self.logger.info(
                    f"Execution {confirmation.status.value} for signal {confirmation.signal_id[:8]}... "
                    f"{'tx: ' + confirmation.tx_hash[:10] + '...' if confirmation.tx_hash else ''}"
                )
                
                # Call callback if registered
                callback = self._signal_callbacks.get(confirmation.signal_id)
                if callback:
                    try:
                        await callback(signal, confirmation)
                    except Exception as e:
                        self.logger.error(f"Error in confirmation callback: {e}")
                
                # Clean up completed signals
                if confirmation.status in [ExecutionStatus.CONFIRMED, ExecutionStatus.FAILED, ExecutionStatus.CANCELLED]:
                    self._pending_signals.pop(confirmation.signal_id, None)
                    self._signal_callbacks.pop(confirmation.signal_id, None)
                    
        except Exception as e:
            self.logger.error(f"Error handling confirmation: {e}")
    
    def get_pending_signals(self) -> Dict[str, TradeSignal]:
        """Get all pending signals awaiting confirmation."""
        return dict(self._pending_signals)
    
    def get_stats(self) -> Dict[str, Any]:
        """Get publisher statistics."""
        return {
            "signals_sent": self._signals_sent,
            "signals_confirmed": self._signals_confirmed,
            "signals_failed": self._signals_failed,
            "pending_count": len(self._pending_signals),
            "success_rate": (
                self._signals_confirmed / self._signals_sent 
                if self._signals_sent > 0 else 0
            )
        }