import time
import sys
import os

# Prioritize local release build for development verification
local_lib_path = "/home/nima/code/crypto/blockchains/eth/reth/target/release"
if os.path.exists(local_lib_path):
    sys.path.insert(0, local_lib_path)

try:
    import pyreth
except ImportError:
    print("pyreth not found. Make sure it is built (cargo build --release -p pyreth) and renamed to pyreth.so")
    sys.exit(1)

print(f"Loaded pyreth from: {pyreth.__file__}")

def main():
    print("🚀 Starting Python CEX Price Verification...")
    
    # Define pairs to watch
    pairs = ["ETH-USDT", "ETH-USD"]
    print(f"   Subscribing to: {pairs}")

    try:
        # Initialize the feeder
        # This spawns the Rust background tasks for Binance, Coinbase, Kraken, OKX, etc.
        feeder = pyreth.PyCexPriceFeeder(pairs)
        
        print("✅ Feeder initialized. Polling for updates (Ctrl+C to stop)...")
        print(f"{ 'Exchange':<12} | { 'Pair':<10} | { 'Bid':<10} | { 'Ask':<10} | {'Exch Delta':<10}")
        print("-" * 65)

        start_time = time.time()
        
        while True:
            # Non-blocking poll
            update = feeder.get_update()
            
            if update:
                ts_str = "None"
                if update.exchange_timestamp:
                    # Calculate latency (now - exchange_ts) in ms
                    delta = (time.time() - update.exchange_timestamp) * 1000
                    ts_str = f"{delta:.1f}ms"
                
                print(f"{update.exchange:<12} | {update.pair:<10} | ${update.bid:<9.2f} | ${update.ask:<9.2f} | {ts_str:<10}")
            else:
                # Avoid busy loop
                time.sleep(0.001)

            # Optional: Stop after 20 seconds for automated testing
            # if time.time() - start_time > 20:
            #     break

    except KeyboardInterrupt:
        print("\n🛑 Stopped by user.")
    except Exception as e:
        print(f"\n❌ Error: {e}")

if __name__ == "__main__":
    main()
