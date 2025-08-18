#!/bin/bash

# Run the Live Block Processor directly (for testing/development)
# This script sets up the environment and runs the processor without systemd

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$( cd "${SCRIPT_DIR}/.." && pwd )"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   Ethereum Live Block Processor${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Set up environment variables
export PYTHONPATH="${PROJECT_DIR}:${PYTHONPATH}"
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/eth_db"
export RABBITMQ_URL="amqp://guest:guest@127.0.0.1/"
export LOG_LEVEL="${LOG_LEVEL:-INFO}"
export PYTHONUNBUFFERED=1

# Check if qw conda environment is activated
if [[ "$CONDA_DEFAULT_ENV" != "qw" ]]; then
    echo -e "${YELLOW}[!] Conda environment 'qw' is not activated${NC}"
    echo -e "${YELLOW}[!] Activating 'qw' environment...${NC}"
    
    # Try to activate conda environment
    if [ -f "/home/nima/miniconda3/etc/profile.d/conda.sh" ]; then
        source "/home/nima/miniconda3/etc/profile.d/conda.sh"
        conda activate qw
        echo -e "${GREEN}[✓] Activated 'qw' environment${NC}"
    else
        echo -e "${YELLOW}[!] Could not activate conda environment${NC}"
        echo -e "${YELLOW}[!] Using system Python instead${NC}"
    fi
fi

# Display configuration
echo -e "${BLUE}Configuration:${NC}"
echo "  Working Directory: ${PROJECT_DIR}"
echo "  Python Path: ${PYTHONPATH}"
echo "  Database: ${DATABASE_URL}"
echo "  RabbitMQ: ${RABBITMQ_URL}"
echo "  Log Level: ${LOG_LEVEL}"
echo "  Python: $(which python)"
echo ""

# Check dependencies
echo -e "${BLUE}Checking dependencies...${NC}"

# Check if PostgreSQL is running
if pg_isready -h localhost -p 5432 > /dev/null 2>&1; then
    echo -e "${GREEN}[✓] PostgreSQL is running${NC}"
else
    echo -e "${YELLOW}[!] PostgreSQL is not running${NC}"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check if RabbitMQ is running (optional)
if rabbitmqctl status > /dev/null 2>&1; then
    echo -e "${GREEN}[✓] RabbitMQ is running${NC}"
else
    echo -e "${YELLOW}[!] RabbitMQ is not running (optional)${NC}"
fi

# Check if process_blocks_live.py exists
if [ -f "${PROJECT_DIR}/scripts/process_blocks_live.py" ]; then
    echo -e "${GREEN}[✓] Script found: process_blocks_live.py${NC}"
else
    echo -e "${YELLOW}[!] Script not found: process_blocks_live.py${NC}"
    exit 1
fi

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Starting Live Block Processor...${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Press Ctrl+C to stop"
echo ""

# Change to project directory
cd "${PROJECT_DIR}"

# Run the processor
if [[ "$CONDA_DEFAULT_ENV" == "qw" ]]; then
    # Use conda environment Python
    python scripts/process_blocks_live.py
else
    # Use explicit path to qw environment Python
    /home/nima/miniconda3/envs/qw/bin/python scripts/process_blocks_live.py
fi