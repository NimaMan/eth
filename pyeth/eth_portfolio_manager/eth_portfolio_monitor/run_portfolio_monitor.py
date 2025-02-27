from flask import Flask
import os
from routes import main, backtest_api
from eth_portfolio_manager.utils.logger import get_monitoring_logger, setup_flask_logger


def create_app():
    logger = get_monitoring_logger()
    logger.info("Creating Flask app")
    
    # Get absolute paths for templates and static folders
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    template_dir = os.path.join(base_dir, 'eth_portfolio_monitor', 'templates')
    static_dir = os.path.join(base_dir, 'eth_portfolio_monitor', 'static')
    
    app = Flask(
        __name__,
        template_folder=template_dir,
        static_folder=static_dir
    )
    
    # Setup Flask logger
    setup_flask_logger(app, name="portfolio_monitor")
    
    logger.info("Registering blueprints")
    app.register_blueprint(main)
    #app.register_blueprint(live_position_api, url_prefix='/api')
    app.register_blueprint(backtest_api, url_prefix='/api/backtest')
    
    return app


if __name__ == '__main__':
    app = create_app()
    PORT = 40019  # Use our tested working port
    
    app.run(
        debug=True,
        host='0.0.0.0',  # Bind to all interfaces as we learned
        port=PORT
    )