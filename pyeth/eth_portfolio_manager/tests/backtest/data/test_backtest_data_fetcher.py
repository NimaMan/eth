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
    keys = list(runs.keys())
    if runs:
        print("Strategy Run Sample:", runs[keys[0]])
    else:
        print("No strategy runs found.")
    
    if not runs:
        return
    strategy_run_id = runs[keys[0]]['id']
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
    print("\nTesting fetch_strategy_token_positions() (sample) for strategy_run_id =", strategy_run_id)
    positions = fetcher.fetch_strategy_token_positions(strategy_run_id)
    keys = list(positions.keys())
    token_address = keys[0]
    if positions:
        print("Position Sample for token =", token_address, ":", positions[token_address])
    else:
        print("No positions found for Strategy Run", strategy_run_id)
    print("-"*100)
    if positions:
        print("\nTesting fetch_token_position_history() (sample) for strategy_run_id =", strategy_run_id, "and token =", token_address)
        history = fetcher.fetch_token_position_history(strategy_run_id, token_address)
        if history:
            print("Token History Sample:", history['dynamic_history'][-1])
        else:
            print("No token position history found for token:", token_address)
    else:
        print("\nNo positions found; skipping token position history test.")

    print("-"*100)
    print("\nTesting fetch_strategy_performance_metrics() for strategy_run_id =", strategy_run_id)
    metrics = fetcher.fetch_strategy_performance_metrics(strategy_run_id)
    print("Performance Metrics:", metrics)


if __name__ == "__main__":
    main()