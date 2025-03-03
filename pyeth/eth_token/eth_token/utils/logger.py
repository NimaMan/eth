'''
Logger Module Documentation Objective:
The primary objective of this logger module is to provide a centralized and consistent logging mechanism for the entire `eth_block_processor` project. It aims to:

1. Create a standardized logging format across all modules.
2. Allow for easy integration of logging in any part of the project.
3. Ensure that all logs are stored in a predefined location for easy access and analysis.
4. Delete empty log files when the program exits.

'''
import os
from datetime import datetime
import logging
import atexit


# Default log directory; can be customized as needed
ETH_LOG_DIR = os.getenv('ETH_LOG_DIR', '/home/nima/code/crypto/logs')

# Track all created log files
_log_files = set()

def cleanup_empty_logs():
    """
    Delete log files that are empty or contain fewer than two lines.
    Scans both tracked files and the entire log directory structure.
    """
    # First clean up tracked files
    for log_file in _log_files:
        try:
            if os.path.exists(log_file):
                with open(log_file, 'r') as f:
                    line_count = sum(1 for _ in f)
                if line_count < 2:
                    os.remove(log_file)
                    print(f"Removed tracked empty log file: {log_file}")
        except Exception as e:
            print(f"Error cleaning up tracked log file {log_file}: {str(e)}")
    
    # Then scan the entire log directory to catch any untracked log files
    try:
        for root, _, files in os.walk(ETH_LOG_DIR):
            for file in files:
                if file.endswith('.log'):
                    log_file_path = os.path.join(root, file)
                    try:
                        with open(log_file_path, 'r') as f:
                            line_count = sum(1 for _ in f)
                        if line_count < 2:
                            os.remove(log_file_path)
                            print(f"Removed untracked empty log file: {log_file_path}")
                    except Exception as e:
                        print(f"Error cleaning up untracked log file {log_file_path}: {str(e)}")
    except Exception as e:
        print(f"Error scanning log directory: {str(e)}")

# Register cleanup function to run at exit
atexit.register(cleanup_empty_logs)


def get_logger(name="token_manager", log_folder="token_manager", base_log_dir=None):
    """
    Initializes and returns a logger with the specified name.
    
    Args:
        name (str): Name of the logger. Defaults to "eth_logger" if not provided.
        log_dir (str): Directory where log files will be stored. Defaults to LOG_DIR.
        
    Returns:
        logging.Logger: Configured logger instance.
    """
    if base_log_dir is None:
        if log_folder is None:
            base_log_dir = os.path.join(ETH_LOG_DIR, name)
        else:
            base_log_dir = os.path.join(ETH_LOG_DIR, log_folder)
        # Create the log directory if it doesn't exist
        os.makedirs(base_log_dir, exist_ok=True)
    
    # Create a standardized logger name
    logger = logging.getLogger(name)
    
    # Prevent adding multiple handlers to the same logger
    if not logger.handlers:
        logger.setLevel(logging.INFO)

        formatter = logging.Formatter('%(asctime)s - %(levelname)s - %(message)s')

        # Add timestamp to the log file name
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        log_file_path = os.path.join(base_log_dir, f"{name}_{timestamp}.log")
        
        # Track the log file
        _log_files.add(log_file_path)
        
        file_handler = logging.FileHandler(log_file_path)
        file_handler.setFormatter(formatter)
        logger.addHandler(file_handler)

    return logger
