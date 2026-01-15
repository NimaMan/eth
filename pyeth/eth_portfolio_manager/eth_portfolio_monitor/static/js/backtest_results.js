document.addEventListener('DOMContentLoaded', function() {
    loadStrategyRuns();
    
    // Add event listener for strategy table row selection
    $('#strategy-selection-table tbody').on('click', 'tr', function() {
        $('.selected').removeClass('selected');
        $(this).addClass('selected');
        
        const strategyRunId = $(this).attr('data-id');
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
            if (!data.strategy_runs || !Array.isArray(data.strategy_runs)) {
                console.error('Invalid strategy runs data format:', data);
                showError('Failed to load strategy runs: Invalid data format');
                return;
            }
            
            // Reference to the table body
            const tbody = document.getElementById('strategy-selection-table').getElementsByTagName('tbody')[0];
            tbody.innerHTML = '';
            
            // Initialize DataTable if not already initialized
            let strategyTable;
            if ($.fn.DataTable.isDataTable('#strategy-selection-table')) {
                strategyTable = $('#strategy-selection-table').DataTable();
                strategyTable.clear();
            } else {
                strategyTable = $('#strategy-selection-table').DataTable({
                    paging: true,
                    searching: true,
                    ordering: true,
                    order: [[3, 'desc']], // Sort by created_at desc
                    pageLength: 10,
                    select: {
                        style: 'single'  // Allow only single row selection
                    }
                });
            }
            
            // Add rows to the table
            data.strategy_runs.forEach(run => {
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
                const blockRange = `${run.start_block}-${run.end_block}`;
                
                // Add row to DataTable
                strategyTable.row.add([
                    run.name,
                    params,
                    blockRange,
                    createdAt
                ]).node().setAttribute('data-id', run.id);
            });
            
            // Draw the table
            strategyTable.draw();
            
            // Select first row if available
            if (data.strategy_runs.length > 0) {
                const firstRow = strategyTable.row(0).node();
                $(firstRow).addClass('selected');
                const strategyRunId = data.strategy_runs[0].id;
                loadStrategyDetails(strategyRunId);
                loadPositions(strategyRunId);
            }
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

// Add a flag to track if we've already loaded data once
let initialLoadComplete = false;

async function loadPositions(strategyRunId) {
    try {
        // Show loading indicator
        showLoading();
        
        // Fetch positions data
        console.log(`Fetching positions for strategy run ID: ${strategyRunId}`);
        const positionsResponse = await fetch(`/api/backtest/positions/${strategyRunId}`);
        
        if (!positionsResponse.ok) {
            throw new Error(`HTTP error! status: ${positionsResponse.status}`);
        }
        
        const positionsData = await positionsResponse.json();
        console.log("Positions data:", positionsData);
        
        // Check if we have the new data format with positions grouped by state
        if (!positionsData.init_positions || !positionsData.buy_positions || 
            !positionsData.sell_positions || !positionsData.scam_positions) {
            throw new Error('Invalid positions data format');
        }
        
        if (!initialLoadComplete) {
            // First load - use DataTables
            clearTables();
            
            // Update tables with DataTables
            updateTable('init-positions-table', positionsData.init_positions, 'Initialization');
            updateTable('buy-positions-table', positionsData.buy_positions, 'Active');
            updateTable('sell-positions-table', positionsData.sell_positions, 'Closed');
            updateTable('scam-positions-table', positionsData.scam_positions, 'Scammed');
            
            initialLoadComplete = true;
        } else {
            // Subsequent loads - don't use DataTables, just update the content
            updateTableWithoutDataTables('init-positions-table', positionsData.init_positions, 'Initialization');
            updateTableWithoutDataTables('buy-positions-table', positionsData.buy_positions, 'Active');
            updateTableWithoutDataTables('sell-positions-table', positionsData.sell_positions, 'Closed');
            updateTableWithoutDataTables('scam-positions-table', positionsData.scam_positions, 'Scammed');
        }
        
        // Update position counts
        document.getElementById('init-count').textContent = positionsData.init_positions.length;
        document.getElementById('buy-count').textContent = positionsData.buy_positions.length;
        document.getElementById('sell-count').textContent = positionsData.sell_positions.length;
        document.getElementById('scam-count').textContent = positionsData.scam_positions.length;
        
        // Try to fetch metrics, but handle error gracefully
        try {
            const metricsResponse = await fetch(`/api/backtest/metrics/${strategyRunId}`);
            if (metricsResponse.ok) {
                const metricsData = await metricsResponse.json();
                console.log("Metrics data:", metricsData);
                
                if (metricsData.metrics) {
                    // Update portfolio stats with currency-specific metrics
                    updatePortfolioStats(metricsData.metrics);
                }
            } else {
                console.error('Error loading metrics:', await metricsResponse.text());
                showError('Failed to load metrics data. Positions will still be displayed.');
                
                // Set default metrics values based on positions
                updateBasicMetrics(positionsData);
            }
        } catch (metricsError) {
            console.error('Error loading metrics:', metricsError);
            showError('Failed to load metrics data. Positions will still be displayed.');
            
            // Set default metrics values based on positions
            updateBasicMetrics(positionsData);
        }
        
        hideLoading();
    } catch (error) {
        console.error('Error loading data:', error);
        showError('Failed to load portfolio data: ' + error.message);
        hideLoading();
    }
}

function updateTable(tableId, positions, tableType) {
    // Get the table element
    const table = document.getElementById(tableId);
    if (!table) {
        console.error(`Table with ID "${tableId}" not found`);
        return;
    }
    
    // Get the tbody element
    const tbody = table.querySelector('tbody');
    if (!tbody) {
        console.error(`Table body not found in table "${tableId}"`);
        return;
    }
    
    // Clear the table body
    tbody.innerHTML = '';
    
    // Add empty state message if no positions
    if (!positions || positions.length === 0) {
        const row = tbody.insertRow();
        const cell = row.insertCell();
        cell.colSpan = 13; // Adjust colspan to match the number of columns
        cell.textContent = `No ${tableType} positions found`;
        cell.className = 'text-center';
        
        // Initialize empty DataTable with minimal features
        safeInitDataTable(tableId, {
            paging: false,
            searching: false,
            info: false,
            ordering: true
        });
        
        return;
    }
    
    // Add position data rows
    positions.forEach(position => {
        const row = tbody.insertRow();
        
        // Add appropriate styling class based on position state
        if (position.position_state === 'Scammed') {
            row.classList.add('table-danger');
        } else if (position.position_state === 'Buy Confirmed' || position.position_state === 'Buy Submitted') {
            row.classList.add('table-success');
        } else if (position.position_state === 'Sell Confirmed' || position.position_state === 'Sell Submitted') {
            row.classList.add('table-warning');
        } else if (position.position_state === 'Init') {
            row.classList.add('table-info');
        }
        
        // Format symbol to show only first 8 characters with ellipsis if longer
        const symbol = position.symbol || '';
        const displaySymbol = symbol.length > 8 ? symbol.substring(0, 8) + '...' : symbol;
        const symbolTooltip = symbol.length > 8 ? `title="${symbol}"` : '';
        
        row.innerHTML = `
            <td><a href="https://dexscreener.com/ethereum/${position.token_address}" target="_blank">
                ${position.token_address.substring(0, 8)}...
            </a></td>
            <td ${symbolTooltip}>${displaySymbol}</td>
            <td>${position.token_age_blocks || 0}</td>
            <td>${position.token_age_hours ? formatTradingAge(parseFloat(position.token_age_hours)) : ''}</td>
            <td>${position.position_state || 'Init'}</td>
            <td>${formatNumber(position.current_value, 2)}</td>
            <td>${formatNumber(position.current_price_ratio, 2)}</td>
            <td class="${position.realized_profit > 0 ? 'text-success' : position.realized_profit < 0 ? 'text-danger' : ''}">${formatNumber(position.realized_profit, 2)}</td>
            <td class="${position.unrealized_profit > 0 ? 'text-success' : position.unrealized_profit < 0 ? 'text-danger' : ''}">${formatNumber(position.unrealized_profit, 2)}</td>
            <td>${position.num_greys || 0}</td>
            <td>${position.num_greens || 0}</td>
            <td>${formatNumber((position.scam_probability || 0) * 100, 2)}%</td>
            <td>${position.currency || ''}</td>
        `;
    });
    
    // Initialize DataTable safely
    safeInitDataTable(tableId, {
        paging: false,      // No pagination for these tables
        searching: true,    // Enable search
        info: false,        // Don't show info (Showing X of Y entries)
        ordering: true,     // Enable column sorting
        order: [[2, 'desc']], // Default sort by age (blocks) in descending order
        columnDefs: [
            { type: 'num', targets: [2, 3, 5, 6, 7, 8, 9, 10, 11] } // Specify numeric columns for proper sorting
        ]
    });
}

function updatePortfolioStats(metrics) {
    // Get container for currency metrics
    const currencyMetricsContainer = document.getElementById('currency-metrics-container');
    if (!currencyMetricsContainer) return;
    
    currencyMetricsContainer.innerHTML = ''; // Clear existing content
    
    // Create table structure
    const table = document.createElement('table');
    table.className = 'table table-bordered table-hover';
    table.id = 'portfolio-metrics-table';
    
    // Create table header
    const thead = document.createElement('thead');
    thead.innerHTML = `
        <tr class="bg-light">
            <th>Currency</th>
            <th>Realized P/L</th>
            <th>Unrealized P/L</th>
            <th>Total P/L</th>
            <th>Init</th>
            <th>Buy</th>
            <th>Sell</th>
            <th>Scam</th>
        </tr>
    `;
    table.appendChild(thead);
    
    // Create table body
    const tbody = document.createElement('tbody');
    
    // Define the order of currencies to display (WETH first)
    const currencyOrder = ['WETH'];
    
    // Add all other currencies
    Object.keys(metrics.currency_metrics || {}).forEach(currency => {
        if (currency !== 'WETH' && currency !== 'Unknown') {
            currencyOrder.push(currency);
        }
    });
    
    // Create totals object for all currencies
    const totals = {
        realized_profit: 0,
        unrealized_profit: 0,
        init_count: 0,
        buy_count: 0,
        sell_count: 0,
        scam_count: 0
    };
    
    // Add each currency row
    currencyOrder.forEach(currency => {
        if (!metrics.currency_metrics || !metrics.currency_metrics[currency]) return;
        
        const currencyMetrics = metrics.currency_metrics[currency];
        
        // Abbreviate currency if needed
        const displayCurrency = currency.length > 4 ? currency.substring(0, 4) : currency;
        
        // Calculate total P/L
        const totalPL = (currencyMetrics.realized_profit || 0) + (currencyMetrics.unrealized_profit || 0);
        
        // Create table row
        const row = document.createElement('tr');
        row.innerHTML = `
            <td><strong>${displayCurrency}</strong></td>
            <td class="${currencyMetrics.realized_profit > 0 ? 'text-success' : currencyMetrics.realized_profit < 0 ? 'text-danger' : ''}">
                ${(currencyMetrics.realized_profit || 0).toFixed(4)}
            </td>
            <td class="${currencyMetrics.unrealized_profit > 0 ? 'text-success' : currencyMetrics.unrealized_profit < 0 ? 'text-danger' : ''}">
                ${(currencyMetrics.unrealized_profit || 0).toFixed(4)}
            </td>
            <td class="${totalPL > 0 ? 'text-success' : totalPL < 0 ? 'text-danger' : ''}">
                ${totalPL.toFixed(4)}
            </td>
            <td>${currencyMetrics.init_count || 0}</td>
            <td>${currencyMetrics.buy_count || currencyMetrics.active_position_count || 0}</td>
            <td>${currencyMetrics.sell_count || currencyMetrics.closed_position_count || 0}</td>
            <td>${currencyMetrics.scam_count || currencyMetrics.scammed_position_count || 0}</td>
        `;
        tbody.appendChild(row);
        
        // Add to totals
        totals.realized_profit += currencyMetrics.realized_profit || 0;
        totals.unrealized_profit += currencyMetrics.unrealized_profit || 0;
        totals.init_count += currencyMetrics.init_count || 0;
        totals.buy_count += currencyMetrics.buy_count || currencyMetrics.active_position_count || 0;
        totals.sell_count += currencyMetrics.sell_count || currencyMetrics.closed_position_count || 0;
        totals.scam_count += currencyMetrics.scam_count || currencyMetrics.scammed_position_count || 0;
    });
    
    table.appendChild(tbody);
    currencyMetricsContainer.appendChild(table);
    
    // Update position counts from the metrics data
    document.getElementById('init-count').textContent = 
        metrics.init_position_count || metrics.init_state_count || 0;
    document.getElementById('buy-count').textContent = 
        metrics.buy_position_count || metrics.buy_state_count || 0;
    document.getElementById('sell-count').textContent = 
        metrics.sell_position_count || metrics.sell_state_count || 0;
    document.getElementById('scam-count').textContent = 
        metrics.scam_position_count || metrics.scammed_state_count || 0;
}

function updateBasicMetrics(positionsData) {
    // Simple totals for display
    let realizedProfit = 0;
    let unrealizedProfit = 0;
    let totalProfit = 0;
    
    // Primary currency will be determined from first position if available
    let primaryCurrency = 'WETH';
    
    // Process positions from all categories
    const allPositions = [
        ...positionsData.init_positions, 
        ...positionsData.buy_positions,
        ...positionsData.sell_positions, 
        ...positionsData.scam_positions
    ];
    
    // Group positions by currency
    const currencyMetrics = {};
    
    allPositions.forEach(position => {
        const currency = position.currency || primaryCurrency;
        
        if (!currencyMetrics[currency]) {
            currencyMetrics[currency] = {
                realized_profit: 0,
                unrealized_profit: 0,
                position_count: 0,
                active_position_count: 0,
                closed_position_count: 0,
                scammed_position_count: 0
            };
        }
        
        // Sum financial metrics
        const realizedProfit = parseFloat(position.realized_profit || 0);
        const unrealizedProfit = parseFloat(position.unrealized_profit || 0);
        
        currencyMetrics[currency].realized_profit += realizedProfit;
        currencyMetrics[currency].unrealized_profit += unrealizedProfit;
        currencyMetrics[currency].position_count += 1;
        
        // Count by position state
        const state = position.position_state;
        if (state === 'Buy Confirmed' || state === 'Buy Submitted') {
            currencyMetrics[currency].active_position_count += 1;
        } else if (state === 'Sell Confirmed' || state === 'Sell Submitted') {
            currencyMetrics[currency].closed_position_count += 1;
        } else if (state === 'Scammed') {
            currencyMetrics[currency].scammed_position_count += 1;
        }
    });
    
    // Update the UI with our computed metrics
    const metrics = {
        currency_metrics: currencyMetrics,
        init_position_count: positionsData.init_positions.length,
        buy_position_count: positionsData.buy_positions.length,
        sell_position_count: positionsData.sell_positions.length,
        scam_position_count: positionsData.scam_positions.length
    };
    
    updatePortfolioStats(metrics);
}

function clearStrategyInfo() {
    document.getElementById('strategy-start-time').textContent = '-';
    document.getElementById('strategy-end-time').textContent = '-';
    document.getElementById('strategy-total-trades').textContent = '-';
    document.getElementById('strategy-success-rate').textContent = '-';
}

function clearTables() {
    // For each table, properly destroy DataTables and clear contents
    ['init-positions-table', 'buy-positions-table', 'sell-positions-table', 'scam-positions-table'].forEach(tableId => {
        // Check if it's a DataTable and destroy it properly
        if ($.fn.DataTable.isDataTable(`#${tableId}`)) {
            $(`#${tableId}`).DataTable().destroy();
        }
        
        // Clear table body
        const tbody = document.getElementById(tableId).getElementsByTagName('tbody')[0];
        if (tbody) {
            tbody.innerHTML = '';
        }
    });
    
    // Reset counts
    document.getElementById('init-count').textContent = '0';
    document.getElementById('buy-count').textContent = '0';
    document.getElementById('sell-count').textContent = '0';
    document.getElementById('scam-count').textContent = '0';
    
    // Clear currency metrics container
    const currencyMetricsContainer = document.getElementById('currency-metrics-container');
    if (currencyMetricsContainer) {
        currencyMetricsContainer.innerHTML = '';
    }
}

function showError(message) {
    const alertDiv = document.createElement('div');
    alertDiv.className = 'alert alert-danger';
    alertDiv.textContent = message;
    document.querySelector('.container').prepend(alertDiv);
    setTimeout(() => alertDiv.remove(), 5000);
}

// Show loading indicator
function showLoading() {
    const loadingDiv = document.createElement('div');
    loadingDiv.id = 'loading-indicator';
    loadingDiv.className = 'text-center mt-3 mb-3';
    loadingDiv.innerHTML = `
        <div class="spinner-border text-primary" role="status">
            <span class="sr-only">Loading...</span>
        </div>
        <p class="mt-2">Loading data...</p>
    `;
    
    document.querySelector('.container').appendChild(loadingDiv);
}

// Hide loading indicator
function hideLoading() {
    const loadingIndicator = document.getElementById('loading-indicator');
    if (loadingIndicator) {
        loadingIndicator.remove();
    }
}

// Helper functions for formatting 
function formatNumber(value, decimals = 2) {
    if (value === null || value === undefined) return '-';
    const number = parseFloat(value);
    if (isNaN(number)) return '-';
    return number.toFixed(decimals);
}

function formatTradingAge(hours) {
    if (hours === null || hours === undefined) return '-';
    const hoursNum = parseFloat(hours);
    if (isNaN(hoursNum)) return '-';
    
    // Format based on time ranges
    if (hoursNum < 1) {
        const minutes = Math.round(hoursNum * 60);
        return `${minutes}m`;
    } else if (hoursNum < 24) {
        return `${Math.round(hoursNum)}h`;
    } else {
        const days = Math.floor(hoursNum / 24);
        const remainingHours = Math.round(hoursNum % 24);
        return remainingHours > 0 ? `${days}d ${remainingHours}h` : `${days}d`;
    }
}

// Add this helper function near the top of the file
function safeInitDataTable(tableId, options) {
    try {
        // Ensure any existing DataTable is destroyed first
        if ($.fn.DataTable.isDataTable(`#${tableId}`)) {
            $(`#${tableId}`).DataTable().destroy();
        }
        
        // Make sure destroy option is set
        const safeOptions = { 
            ...options,
            destroy: true
        };
        
        // Initialize the DataTable
        return $(`#${tableId}`).DataTable(safeOptions);
    } catch (error) {
        console.error(`Error initializing DataTable for ${tableId}:`, error);
        // Return a stub object with common DataTable methods to prevent further errors
        return {
            draw: () => {},
            clear: () => {},
            rows: () => ({ remove: () => {} }),
            destroy: () => {}
        };
    }
}

// Simple function to update table without DataTables
function updateTableWithoutDataTables(tableId, positions, tableType) {
    const table = document.getElementById(tableId);
    if (!table) return;
    
    const tbody = table.querySelector('tbody');
    if (!tbody) return;
    
    // Clear the table body
    tbody.innerHTML = '';
    
    // Add empty state message if no positions
    if (!positions || positions.length === 0) {
        const row = tbody.insertRow();
        const cell = row.insertCell();
        cell.colSpan = 13;
        cell.textContent = `No ${tableType} positions found`;
        cell.className = 'text-center';
        return;
    }
    
    // Add position data rows
    positions.forEach(position => {
        const row = tbody.insertRow();
        
        // Add styling classes
        if (position.position_state === 'Scammed') {
            row.classList.add('table-danger');
        } else if (position.position_state === 'Buy Confirmed' || position.position_state === 'Buy Submitted') {
            row.classList.add('table-success');
        } else if (position.position_state === 'Sell Confirmed' || position.position_state === 'Sell Submitted') {
            row.classList.add('table-warning');
        } else if (position.position_state === 'Init') {
            row.classList.add('table-info');
        }
        
        // Format symbol
        const symbol = position.symbol || '';
        const displaySymbol = symbol.length > 8 ? symbol.substring(0, 8) + '...' : symbol;
        const symbolTooltip = symbol.length > 8 ? `title="${symbol}"` : '';
        
        // Add cells with data
        row.innerHTML = `
            <td><a href="https://dexscreener.com/ethereum/${position.token_address}" target="_blank">
                ${position.token_address.substring(0, 8)}...
            </a></td>
            <td ${symbolTooltip}>${displaySymbol}</td>
            <td>${position.token_age_blocks || 0}</td>
            <td>${position.token_age_hours ? formatTradingAge(parseFloat(position.token_age_hours)) : ''}</td>
            <td>${position.position_state || 'Init'}</td>
            <td>${formatNumber(position.current_value, 2)}</td>
            <td>${formatNumber(position.current_price_ratio, 2)}</td>
            <td class="${position.realized_profit > 0 ? 'text-success' : position.realized_profit < 0 ? 'text-danger' : ''}">${formatNumber(position.realized_profit, 2)}</td>
            <td class="${position.unrealized_profit > 0 ? 'text-success' : position.unrealized_profit < 0 ? 'text-danger' : ''}">${formatNumber(position.unrealized_profit, 2)}</td>
            <td>${position.num_greys || 0}</td>
            <td>${position.num_greens || 0}</td>
            <td>${formatNumber((position.scam_probability || 0) * 100, 2)}%</td>
            <td>${position.currency || ''}</td>
        `;
    });
} 