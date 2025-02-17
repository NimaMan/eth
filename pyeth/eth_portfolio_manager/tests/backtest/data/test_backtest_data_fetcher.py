"""
Objective:
---------
Test the backtest data fetcher and portfolio metrics calculator.
This updated version prints sample results and includes basic assertions
to verify that key performance metrics are computed as expected.
"""

import asyncio
from eth_portfolio_manager.backtesting.db.backtest_data_fetcher import BacktestDataFetcher


def main():
    # Instantiate the data fetcher.
    fetcher = BacktestDataFetcher()

    print("Testing fetch_strategy_runs()")
    runs = fetcher.fetch_strategy_runs()
    if runs:
        print("Strategy Run Sample:", runs[0])
    else:
        print("No strategy runs found.")
    
    if not runs:
        return
    strategy_run_id = runs[0]['id']
    print("-"*100)
    print("\nTesting fetch_strategy_run_details() (sample) for strategy_run_id =", strategy_run_id)
    details = fetcher.fetch_strategy_run_details(strategy_run_id)
    print("Strategy Run Details Sample:", details)
    print("-"*100)
    print("\nTesting fetch_token_list_for_strategy() (sample) for strategy_run_id =", strategy_run_id)
    tokens = fetcher.fetch_token_list_for_strategy(strategy_run_id)
    if tokens:
        print("Token List Sample:", tokens[0])
    else:
        print("No tokens found for Strategy Run", strategy_run_id)
    print("-"*100)
    print("\nTesting fetch_strategy_positions() (sample) for strategy_run_id =", strategy_run_id)
    positions = fetcher.fetch_strategy_positions(strategy_run_id)
    if positions:
        print("Position Sample:", positions[0])
    else:
        print("No positions found for Strategy Run", strategy_run_id)
    print("-"*100)
    if positions:
        token_address = positions[0]['token_address']
        print("\nTesting fetch_token_position_history() (sample) for strategy_run_id =", strategy_run_id, "and token =", token_address)
        history = fetcher.fetch_token_position_history(strategy_run_id, token_address)
        if history:
            print("Token History Sample:", history[0])
        else:
            print("No token position history found for token:", token_address)
    else:
        print("\nNo positions found; skipping token position history test.")

    print("-"*100)
    print("\nTesting fetch_strategy_performance_metrics() for strategy_run_id =", strategy_run_id)
    try:
        metrics = asyncio.run(fetcher.fetch_strategy_performance_metrics(strategy_run_id))
        print("Performance Metrics:", metrics)
        
        # Basic assertions to verify computed metrics.
        # Assert that total value equals active plus inactive value.
        assert abs(metrics.total_value - (metrics.active_value + metrics.inactive_value)) < 1e-6, "Total value mismatch"
        
        # Assert that the total position count equals the sum of active and inactive positions.
        assert metrics.total_position_count == metrics.active_position_count + metrics.inactive_position_count, "Position count mismatch"
        
        # Additional optional checks (if expected values are known, these can be extended)
        print("All metrics assertions passed.")
        
    except Exception as e:
        print("Error fetching performance metrics:", e)


if __name__ == "__main__":
    main()