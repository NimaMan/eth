document.addEventListener('DOMContentLoaded', function() {
    loadStrategyRuns();
    
    // Add event listener for strategy selection
    document.getElementById('strategy-run-selector').addEventListener('change', function(e) {
        const strategyRunId = e.target.value;
        if (strategyRunId) {
            loadStrategyDetails(strategyRunId);
            loadPositions(strategyRunId);
        } else {
            clearStrategyInfo();
            clearTables();
        }
    });
});

function loadStrategyRuns() {
    fetch('/api/backtest/strategy-runs')
        .then(response => response.json())
        .then(data => {
            const selector = document.getElementById('strategy-run-selector');
            selector.innerHTML = '<option value="">Select a strategy run...</option>';
            
            data.strategy_runs.forEach(run => {
                const option = document.createElement('option');
                option.value = run.id;
                
                // Format parameters for display
                const params = Object.entries(run.parameters)
                    .map(([key, value]) => {
                        // Format the key by replacing underscores with spaces and capitalizing
                        const formattedKey = key.split('_')
                            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
                            .join(' ');
                        // Format the value (handle numbers with precision)
                        const formattedValue = typeof value === 'number' ? 
                            (value % 1 === 0 ? value : value.toFixed(2)) : 
                            value;
                        return `${formattedKey}: ${formattedValue}`;
                    })
                    .join(' | ');
                
                const createdAt = new Date(run.created_at).toLocaleString();
                option.textContent = `${run.name} | ${params} | Blocks: ${run.start_block}-${run.end_block} | ${createdAt}`;
                selector.appendChild(option);
            });
        })
        .catch(error => {
            console.error('Error loading strategy runs:', error);
            showError('Failed to load strategy runs');
        });
}

async function loadStrategyDetails(strategyRunId) {
    try {
        const response = await fetch(`/api/backtest/strategy-run/${strategyRunId}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        console.log("Strategy details:", data); // Debug log
        
        if (!data || !data.strategy_run) {
            throw new Error('Invalid strategy run data format');
        }
        
        updateStrategyInfo(data.strategy_run);
    } catch (error) {
        console.error('Error loading strategy details:', error);
        showError('Failed to load strategy details');
    }
}

function updateStrategyInfo(run) {
    document.getElementById('strategy-start-time').textContent = new Date(run.start_time || run.created_at).toLocaleString();
    document.getElementById('strategy-end-time').textContent = run.end_time ? new Date(run.end_time).toLocaleString() : 'Running';
    document.getElementById('strategy-total-trades').textContent = run.total_trades || 0;
    document.getElementById('strategy-success-rate').textContent = (run.success_rate || 0).toFixed(1);
}

async function loadPositions(strategyRunId) {
    try {
        // Fetch both positions and metrics
        const [positionsResponse, metricsResponse] = await Promise.all([
            fetch(`/api/backtest/positions/${strategyRunId}`),
            fetch(`/api/backtest/metrics/${strategyRunId}`)
        ]);

        const positionsData = await positionsResponse.json();
        const metricsData = await metricsResponse.json();

        console.log("Positions data:", positionsData);
        console.log("Metrics data:", metricsData);

        if (!positionsData.positions || !metricsData.metrics) {
            throw new Error('Invalid data format');
        }

        // Update portfolio stats with currency-specific metrics
        updatePortfolioStats(metricsData.metrics);
        
        // Group and update tables
        const groupedPositions = groupPositionsByState(positionsData.positions);
        updatePositionTables(groupedPositions);

    } catch (error) {
        console.error('Error loading data:', error);
        showError('Failed to load portfolio data');
    }
}

function groupPositionsByState(positions) {
    // Initialize groups based on TokenPositionState enum values
    const grouped = {
        init: [],           // TokenPositionState.INIT
        active: [],         // BUY_SUBMITTED and BUY_CONFIRMED
        closed: [],         // SELL_SUBMITTED and SELL_CONFIRMED
        scammed: []         // SCAMMED
    };
    
    positions.forEach(position => {
        const state = position.position_state;
        
        // Categorize based on the latest state
        if (state === 'Init') {
            grouped.init.push(position);
        }
        else if (state === 'Buy Submitted' || state === 'Buy Confirmed') {
            grouped.active.push(position);
        }
        else if (state === 'Sell Submitted' || state === 'Sell Confirmed') {
            grouped.closed.push(position);
        }
        else if (state === 'Scammed') {
            grouped.scammed.push(position);
        }
        else {
            console.warn(`Unknown position state: ${state}`);
            grouped.init.push(position);
        }
    });
    
    return grouped;
}

function updatePositionTables(groupedPositions) {
    // Update tables with their respective positions
    updateTable('init-positions-table', groupedPositions.init, 'Initialization');
    updateTable('buy-positions-table', groupedPositions.active, 'Active');
    updateTable('sell-positions-table', [...groupedPositions.closed, ...groupedPositions.scammed], 'Closed/Scammed');
    
    // Update position counts
    document.getElementById('init-count').textContent = groupedPositions.init.length;
    document.getElementById('buy-count').textContent = groupedPositions.active.length;
    document.getElementById('sell-count').textContent = 
        groupedPositions.closed.length + groupedPositions.scammed.length;
}

function updateTable(tableId, positions, tableType) {
    const tbody = document.getElementById(tableId).getElementsByTagName('tbody')[0];
    tbody.innerHTML = '';
    
    positions.forEach(position => {
        const row = tbody.insertRow();
        
        // Add a class for scammed positions
        if (position.position_state === 'Scammed') {
            row.classList.add('table-danger');
        }
        
        row.innerHTML = `
            <td><a href="https://dexscreener.com/ethereum/${position.token_address}" target="_blank">
                ${position.token_address.substring(0, 8)}...
            </a></td>
            <td>${position.symbol || ''}</td>
            <td>${position.token_age_blocks || 0}</td>
            <td>${position.token_age_hours ? formatTradingAge(parseFloat(position.token_age_hours)) : ''}</td>
            <td>${position.position_state || 'Init'}</td>
            <td>${formatNumber(position.current_value, 2)}</td>
            <td>${formatNumber(position.current_price_ratio, 2)}</td>
            <td class="${position.realized_profit > 0 ? 'text-success' : position.realized_profit < 0 ? 'text-danger' : ''}">${formatNumber(position.realized_profit, 2)}</td>
            <td class="${position.unrealized_profit > 0 ? 'text-success' : position.unrealized_profit < 0 ? 'text-danger' : ''}">${formatNumber(position.unrealized_profit, 2)}</td>
            <td>${position.num_greys || 0}</td>
            <td>${position.num_greens || 0}</td>
            <td>${formatNumber(position.scam_probability * 100, 2)}%</td>
            <td>${Array.isArray(position.scam_reason) ? position.scam_reason.join(', ') : (position.scam_reason || 'NA')}</td>
            <td>${position.currency || ''}</td>
        `;
    });
}

function updatePortfolioStats(metrics) {
    // Get the primary currency (usually WETH)
    const primaryCurrency = 'WETH';
    const currencyMetrics = metrics.currency_metrics[primaryCurrency] || {};

    // Update main portfolio stats
    document.getElementById('total-value').textContent = 
        `${currencyMetrics.total_value?.toFixed(2) || '0.00'} ${primaryCurrency}`;
    document.getElementById('realized-profit').textContent = 
        `${currencyMetrics.realized_profit?.toFixed(2) || '0.00'} ${primaryCurrency}`;
    document.getElementById('unrealized-profit').textContent = 
        `${currencyMetrics.unrealized_profit?.toFixed(2) || '0.00'} ${primaryCurrency}`;
    document.getElementById('total-pl').textContent = 
        `${currencyMetrics.total_profit_loss?.toFixed(2) || '0.00'} ${primaryCurrency}`;

    // Update position counts
    document.getElementById('init-count').textContent = metrics.init_position_count || 0;
    document.getElementById('buy-count').textContent = metrics.buy_position_count || 0;
    document.getElementById('sell-count').textContent = metrics.sell_position_count || 0;

    // Update currency-specific metrics if we want to show them
    updateCurrencyMetrics(metrics.currency_metrics);
}

function updateCurrencyMetrics(currencyMetrics) {
    const currencyStatsDiv = document.getElementById('currency-stats');
    if (!currencyStatsDiv) return;

    currencyStatsDiv.innerHTML = '';
    
    Object.entries(currencyMetrics).forEach(([currency, metrics]) => {
        if (currency === 'Unknown') return;
        
        currencyStatsDiv.innerHTML += `
            <div class="col-xl-12 mb-4">
                <div class="card border-left-info shadow h-100 py-2">
                    <div class="card-body">
                        <h6 class="font-weight-bold text-info">${currency} Metrics</h6>
                        <div class="row">
                            <div class="col-md-3">
                                <div class="text-xs text-uppercase mb-1">Total Value</div>
                                <div class="h6 mb-0 font-weight-bold">${metrics.total_value.toFixed(2)}</div>
                            </div>
                            <div class="col-md-3">
                                <div class="text-xs text-uppercase mb-1">Realized P/L</div>
                                <div class="h6 mb-0 font-weight-bold">${metrics.realized_profit.toFixed(2)}</div>
                            </div>
                            <div class="col-md-3">
                                <div class="text-xs text-uppercase mb-1">Position Count</div>
                                <div class="h6 mb-0 font-weight-bold">${metrics.position_count}</div>
                            </div>
                            <div class="col-md-3">
                                <div class="text-xs text-uppercase mb-1">Active Positions</div>
                                <div class="h6 mb-0 font-weight-bold">${metrics.active_position_count}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>`;
    });
}

function clearStrategyInfo() {
    document.getElementById('strategy-start-time').textContent = '-';
    document.getElementById('strategy-end-time').textContent = '-';
    document.getElementById('strategy-total-trades').textContent = '-';
    document.getElementById('strategy-success-rate').textContent = '-';
}

function clearTables() {
    // Clear all three tables
    ['init-positions-table', 'buy-positions-table', 'sell-positions-table'].forEach(tableId => {
        const tbody = document.getElementById(tableId).getElementsByTagName('tbody')[0];
        tbody.innerHTML = '';
    });
    
    // Reset counts
    document.getElementById('init-count').textContent = '0';
    document.getElementById('buy-count').textContent = '0';
    document.getElementById('sell-count').textContent = '0';
    
    // Reset portfolio stats
    document.getElementById('total-value').textContent = '0.00';
    document.getElementById('total-pl').textContent = '0.00';
}

function showError(message) {
    const alertDiv = document.createElement('div');
    alertDiv.className = 'alert alert-danger';
    alertDiv.textContent = message;
    document.querySelector('.container').prepend(alertDiv);
    setTimeout(() => alertDiv.remove(), 5000);
} 