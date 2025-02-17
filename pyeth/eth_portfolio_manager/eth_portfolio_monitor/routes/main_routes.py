from flask import Blueprint, render_template
from eth_portfolio_manager.utils.logger import get_monitoring_logger
from utils.file_loader import StaticLoader


logger = get_monitoring_logger(name="portfolio_monitor")



main = Blueprint(
    'main', 
    __name__,
    template_folder='../templates',
    static_folder='../static',
    static_url_path='/static'
)


static_loader = StaticLoader(main.static_folder)



@main.route('/')
def index():
    logger.info("Accessing index route")
    try:
        return render_template(
            'index.html',
            **static_loader.get_static_files()
        )
    except Exception as e:
        logger.error(f"Error rendering index: {str(e)}", exc_info=True)
        raise

@main.route('/token-positions')
def token_positions_table():
    logger.info("Accessing token positions table route")
    try:
        return render_template(
            'token_positions_table.html',
            **static_loader.get_static_files()
        )
    except Exception as e:
        logger.error(f"Error rendering token positions table: {str(e)}", exc_info=True)
        raise

@main.route('/backtest-results')
def backtest_results():
    """Render the backtest results page"""
    logger.info("Accessing backtest results route")
    try:
        return render_template(
            'backtest_results.html',
            **static_loader.get_static_files()
        )
    except Exception as e:
        logger.error(f"Error rendering backtest results: {str(e)}", exc_info=True)
        raise

@main.route('/token-position')
def token_position():
    """Render the token position analysis page"""
    try:
        return render_template(
            'token_position.html',
            **static_loader.get_static_files()
        )
    except Exception as e:
        raise
