'''
Logger Module Documentation Objective:
The primary objective of this logger module is to provide a centralized and consistent logging mechanism for the entire `eth_block_processor` project. It aims to:

1. Create a standardized logging format across all modules.
2. Enable file-based logging only, excluding console logging.
3. Allow for easy integration of logging in any part of the project.
4. Ensure that all logs are stored in a predefined location for easy access and analysis.

'''

import logging
import os
from datetime import datetime

# Default log directory; can be customized as needed
LOG_DIR = "/home/nima/code/crypto/logs"

def get_logger(name=None, log_dir=None):
    """
    Initializes and returns a logger with the specified name.
    
    Args:
        name (str): Name of the logger. Defaults to "block_processor" if not provided.
        log_dir (str): Directory where log files will be stored. Defaults to LOG_DIR.
        
    Returns:
        logging.Logger: Configured logger instance.
    """
    if name is None:
        name = "block_processor"
    if log_dir is None:
        log_dir = LOG_DIR

    # Ensure the log directory exists
    os.makedirs(log_dir, exist_ok=True)
    
    # Create a standardized logger name
    logger = logging.getLogger(name)
    
    # Prevent adding multiple handlers to the same logger
    if not logger.handlers:
        logger.setLevel(logging.INFO)

        formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')

        # Add timestamp to the log file name
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        log_file_path = os.path.join(log_dir, f"{name}_{timestamp}.log")
        file_handler = logging.FileHandler(log_file_path)
        file_handler.setFormatter(formatter)
        logger.addHandler(file_handler)

        # **Removed Console Handler**
        # Uncomment the following lines if you ever need console logging
        # console_handler = logging.StreamHandler()
        # console_handler.setFormatter(formatter)
        # logger.addHandler(console_handler)

    return logger
