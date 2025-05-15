from flask import Blueprint, jsonify, request
from eth_portfolio_manager.db.backtest_data_fetcher import BacktestDataFetcher
from eth_portfolio_manager.utils.logger import get_monitoring_logger
import asyncio


logger = get_monitoring_logger()
backtest_api = Blueprint('backtest_api', __name__)
backtest_data = BacktestDataFetcher()


@backtest_api.route('/strategy-runs', methods=['GET'])
def get_strategy_runs():
    """Get all strategy runs"""
    logger.info("API: Fetching strategy runs")
    try:
        runs = backtest_data.fetch_strategy_runs()
        # Convert dict to list for API response
        runs_list = list(runs.values())
        logger.info(f"Found {len(runs_list)} strategy runs")
        return jsonify({'strategy_runs': runs_list})
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/positions/<int:strategy_run_id>')
def get_strategy_token_positions(strategy_run_id):
    """Get all positions for a specific strategy run"""
    logger.info(f"API: Fetching positions for strategy run {strategy_run_id}")
    try:
        # Get the data
        strategy_token_positions = backtest_data.fetch_strategy_token_positions(strategy_run_id)
        for key, value in strategy_token_positions.items():
            print(f"{key}: {len(value)}")
        
        # Return the data as is for now
        return jsonify(strategy_token_positions)
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/latest-strategy-positions/<int:strategy_run_id>')
def get_latest_strategy_positions(strategy_run_id):
    """Get latest positions for a specific strategy run"""
    logger.info(f"API: Fetching latest positions for strategy run {strategy_run_id}")
    try:
        positions = backtest_data.fetch_latest_strategy_positions(strategy_run_id)
        return jsonify({'positions': positions})
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/strategy-run/<int:strategy_run_id>')
def get_strategy_run_details(strategy_run_id):
    """Get detailed information for a specific strategy run"""
    logger.info(f"API: Fetching details for strategy run {strategy_run_id}")
    try:
        details = backtest_data.fetch_strategy_run_details(strategy_run_id)
        if details:
            logger.info("API: Successfully retrieved strategy run details")
            return jsonify({'strategy_run': details})
        else:
            logger.error(f"Strategy run {strategy_run_id} not found or invalid data format")
            return jsonify({'error': 'Strategy run not found or invalid data format'}), 404
    except Exception as e:
        logger.error(f"API Error in get_strategy_run_details: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/token-list/<int:strategy_run_id>')
def get_token_list(strategy_run_id):
    """Get list of tokens for a strategy run"""
    logger.info(f"API: Fetching token list for strategy run {strategy_run_id}")
    try:
        tokens = backtest_data.fetch_token_list_for_strategy(strategy_run_id)
        return jsonify({'tokens': tokens})
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/position-history/<int:strategy_run_id>/<token_address>')
def get_token_position_history(strategy_run_id, token_address):
    """Get history for a specific position"""
    logger.info(f"API: Fetching history for token {token_address} in strategy run {strategy_run_id}")
    try:
        history = backtest_data.fetch_token_position_history(strategy_run_id, token_address)
        return jsonify({'history': history})
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/metrics/<int:strategy_run_id>')
def get_strategy_metrics(strategy_run_id):
    """Get metrics for a specific strategy run"""
    logger.info(f"API: Fetching metrics for strategy run {strategy_run_id}")
    try:
        # First get positions data
        positions_data = backtest_data.fetch_strategy_token_positions(strategy_run_id)
        metrics = backtest_data.metrics_calculator.calculate_metrics_for_api(positions_data)
        logger.info(f"Calculated metrics for strategy run {strategy_run_id}")
        return jsonify(metrics)
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/api/strategy/<int:strategy_id>/tokens', methods=['GET'])
def get_strategy_tokens(strategy_id):
    """Get list of tokens for a strategy"""
    logger.info(f"API: Fetching tokens for strategy {strategy_id}")
    try:
        tokens = backtest_data.fetch_token_list_for_strategy(strategy_id)
        return jsonify({'tokens': tokens})
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@backtest_api.route('/api/strategy/<int:strategy_id>/token/<token_address>/history', methods=['GET'])
def get_strategy_token_history(strategy_id, token_address):
    """Get position history for a specific token in a strategy"""
    logger.info(f"API: Fetching history for token {token_address} in strategy {strategy_id}")
    try:
        history = backtest_data.fetch_token_position_history(strategy_id, token_address)
        return jsonify({'history': history})
    except Exception as e:
        logger.error(f"API Error: {str(e)}", exc_info=True)
        return jsonify({'error': str(e)}), 500