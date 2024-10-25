'''
Logger Module Documentation Objective:
The primary objective of this logger module is to provide a centralized and consistent logging mechanism for the entire `eth_block_processor` project. It aims to:

1. Create a standardized logging format across all modules.
2. Enable both file-based and console-based logging.
3. Allow for easy integration of logging in any part of the project.
4. Ensure that all logs are stored in a predefined location for easy access and analysis.

'''

import logging
import os
from datetime import datetime

LOG_PATH = "/home/nima/code/crypto/logs"

def get_logger(name):
    logger = logging.getLogger(name)
    logger.setLevel(logging.INFO)

    formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')

    # Create logs directory if it doesn't exist
    os.makedirs(LOG_PATH, exist_ok=True)

    # Add timestamp to the log file name
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    file_handler = logging.FileHandler(f"{LOG_PATH}/{name}_{timestamp}.log")
    file_handler.setFormatter(formatter)
    logger.addHandler(file_handler)

    # Add console handler
    console_handler = logging.StreamHandler()
    console_handler.setFormatter(formatter)
    logger.addHandler(console_handler)

    return logger
