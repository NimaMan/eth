// Initialize charts
let valueChart = null;
let priceChart = null;
let tokenTable = null;
let currentStrategyId = null;
let positionHistoryTable = null;

document.addEventListener('DOMContentLoaded', function() {
    // Initialize the token table with DataTables
    tokenTable = $('#token-list-table').DataTable({
        order: [[0, 'desc']], // Sort by creation time descending by default
        paging: true,
        pageLength: 10,
        scrollY: '300px',
        scrollCollapse: true,
        select: {
            style: 'single'  // Allow only single row selection
        },
        columnDefs: [
            {
                targets: 2, // Address column
                width: '100px'
            }
        ],
        columns: [
            { 
                data: null,
                title: 'Creation Time',
                render: function(data) {
                    const timestamp = data.latest_snapshot.timestamp;
                    if (!timestamp) return '-';
                    const date = new Date(timestamp * 1000);
                    return date.toLocaleString('en-US', {
                        month: 'short',
                        day: '2-digit',
                        hour: '2-digit',
                        minute: '2-digit',
                        hour12: false
                    });
                }
            },
            { 
                data: null,
                title: 'Symbol',
                render: function(data) {
                    // Try to get symbol from both static_data and latest_snapshot
                    return data.static_data?.symbol || data.latest_snapshot?.symbol || '-';
                }
            },
            { 
                data: null,
                title: 'Address',
                render: function(data) {
                    const address = data.static_data.token_address;
                    if (address && typeof address === 'string') {
                        return `<a href="https://dexscreener.com/ethereum/${address}" target="_blank">${address.substring(0,8)}</a>`;
                    }
                    return '-';
                }
            },
            { 
                data: null,
                title: 'Age (blocks)',
                render: function(data) {
                    const hours = data.latest_snapshot.token_age_hours;
                    const blocks = data.latest_snapshot.token_age_blocks;
                    return `${hours ? formatTradingAge(parseFloat(hours)) : '0'} (${blocks || 0})`;
                }
            },
            { 
                data: null,
                title: 'ROI',
                render: function(data) {
                    const roi = data.latest_snapshot.roi || 0;
                    return `${formatNumber(roi * 100, 2)}%`;
                },
                className: 'text-right'
            },
            { 
                data: null,
                title: 'Total Profit',
                render: function(data) {
                    const realizedPL = data.latest_snapshot.realized_profit || 0;
                    const unrealizedPL = data.latest_snapshot.unrealized_profit || 0;
                    const totalPL = realizedPL + unrealizedPL;
                    
                    const colorClass = totalPL > 0 ? 'text-success' : totalPL < 0 ? 'text-danger' : '';
                    return `<span class="${colorClass}">${formatNumber(totalPL, 4)}</span>`;
                },
                className: 'text-right'
            },
            { 
                data: null,
                title: 'State',
                render: function(data) {
                    return data.latest_snapshot.position_state || 'Init';
                }
            },
            { 
                data: null,
                title: 'Current Price Ratio',
                render: function(data) {
                    return formatNumber(data.latest_snapshot.current_price_ratio || 0, 2);
                },
                className: 'text-right'
            }
        ]
    });

    // Add row click handler
    $('#token-list-table tbody').on('click', 'tr', function() {
        const rowData = tokenTable.row(this).data();
        if (rowData) {
            $(this).addClass('selected').siblings().removeClass('selected');
            // Use token address from static_data if present or fallback to latest_snapshot
            const tokenAddress = rowData.static_data.token_address || rowData.latest_snapshot.token_address;
            updateTokenStaticData(rowData.static_data);
            loadPositionHistory(currentStrategyId, tokenAddress);
        }
    });

    // Add strategy change handler
    document.getElementById('strategy-select').addEventListener('change', function(e) {
        const strategyId = e.target.value;
        if (strategyId) {
            loadTokens(strategyId);
        }
    });

    // Load strategies and auto-select first one
    loadStrategies();
});

async function loadStrategies() {
    try {
        const response = await fetch('/api/backtest/strategy-runs');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        const select = document.getElementById('strategy-select');
        select.innerHTML = '<option value="">Select Strategy...</option>';
        
        if (data.strategy_runs && data.strategy_runs.length > 0) {
            data.strategy_runs.forEach(run => {
                const option = document.createElement('option');
                option.value = run.id;
                
                // Format parameters for display
                const params = Object.entries(run.parameters)
                    .map(([key, value]) => {
                        const formattedKey = key.split('_')
                            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
                            .join(' ');
                        const formattedValue = typeof value === 'number' ? 
                            (value % 1 === 0 ? value : value.toFixed(4)) : 
                            value;
                        return `${formattedKey}: ${formattedValue}`;
                    })
                    .join(' | ');
                
                const createdAt = new Date(run.created_at).toLocaleString();
                option.textContent = `${run.name} | ${params} | ${createdAt} | Blocks: ${run.start_block}-${run.end_block}`;
                select.appendChild(option);
            });

            // Auto-select first strategy
            select.value = data.strategy_runs[0].id;
            loadTokens(data.strategy_runs[0].id);
        }
    } catch (error) {
        console.error('Error loading strategies:', error);
        showError('Failed to load strategies');
    }
}

// Helper function: Updates the Token Static Data card with static_data values.
function updateTokenStaticData(staticData) {
    // Token symbol
    document.getElementById('token-symbol').textContent = staticData.symbol || '-';
    // Currency (if available)
    document.getElementById('token-currency').textContent = staticData.currency || '-';
    // Token address
    document.getElementById('token-address').textContent = staticData.token_address || '-';
    // Creation block
    document.getElementById('creation-block').textContent = staticData.creation_block || '-';
    // Trading enabled block
    document.getElementById('trading-enabled-block').textContent = staticData.trading_enabled_block || '-';
    // Entry block
    document.getElementById('entry-block').textContent = staticData.entry_block || '-';
    // Exit block
    document.getElementById('exit-block').textContent = staticData.exit_block || '-';
    // Entry Price (format as needed)
    document.getElementById('entry-price').textContent = 
        staticData.entry_price_ratio !== undefined ? staticData.entry_price_ratio.toFixed(2) : '-';
    // Exit Price
    document.getElementById('exit-price').textContent = 
        staticData.exit_price_ratio !== undefined ? staticData.exit_price_ratio.toFixed(2) : '-';
    // Purchase value (using formatNumber utility)
    document.getElementById('purchase-value').textContent = 
        staticData.purchase_value ? formatNumber(staticData.purchase_value, 2) : '-';
    // Entry fee
    document.getElementById('entry-fee').textContent = 
        staticData.entry_txn_fee ? formatNumber(staticData.entry_txn_fee, 2) : '-';
    // Exit fee
    document.getElementById('exit-fee').textContent = 
        staticData.exit_txn_fee ? formatNumber(staticData.exit_txn_fee, 2) : '-';
}

async function loadTokens(strategyId) {
    try {
        currentStrategyId = strategyId;
        const response = await fetch(`/api/backtest/positions/${strategyId}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        
        const data = await response.json();
        console.log('Raw API response:', data);
        
        // Clear the existing table data
        tokenTable.clear();
        
        if (!data.positions) {
            console.error('No positions data:', data);
            showError('No positions data received from server');
            return;
        }
        
        // Transform the positions; use the token address from static_data if available, otherwise use the key
        const formattedPositions = Object.entries(data.positions).map(([key, position]) => {
            console.log('Processing position:', key, position);
            return {
                static_data: position.static_data || {},
                latest_snapshot: {
                    // Use the token_address from static_data if present, so we send the proper address;
                    // otherwise fall back to the key
                    token_address: (position.static_data && position.static_data.token_address) || key,
                    ...position.latest_snapshot
                }
            };
        });
        
        console.log('Formatted positions:', formattedPositions);
        
        if (formattedPositions.length === 0) {
            console.warn('No positions to display after formatting');
            showError('No positions to display');
            return;
        }
        
        // Add the formatted data to the table
        tokenTable.rows.add(formattedPositions).draw();
        
        // Show the position details section
        document.getElementById('position-details').style.display = 'block';
        
        // Select the first row if available and update the static data card and token history.
        const firstRow = tokenTable.row(0).node();
        if (firstRow) {
            $(firstRow).addClass('selected').siblings().removeClass('selected');
            const firstToken = formattedPositions[0];
            // Use token address from static_data if available; otherwise from latest_snapshot.
            const tokenAddress = firstToken.static_data.token_address || firstToken.latest_snapshot.token_address;
            updateTokenStaticData(firstToken.static_data);
            loadPositionHistory(strategyId, tokenAddress);
        }
        
    } catch (error) {
        console.error('Error loading tokens:', error);
        showError(`Failed to load tokens: ${error.message}`);
    }
}

// Add this helper function to check data structure
function validatePositionData(position) {
    console.log('Validating position:', position);
    return {
        latest_snapshot: {
            token_age_hours: position.token_age_hours || 0,
            token_age_blocks: position.token_age_blocks || 0,
            roi: position.roi || 0,
            realized_profit: position.realized_profit || 0,
            unrealized_profit: position.unrealized_profit || 0,
            position_state: position.position_state || 'Init',
            current_price_ratio: position.current_price_ratio || 0
        }
    };
}

async function loadPositionHistory(strategyId, tokenAddress) {
    try {
        console.log(`Loading history for strategy ${strategyId} and token ${tokenAddress}`);
        const response = await fetch(`/api/backtest/position-history/${strategyId}/${tokenAddress}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        if (data.history && (data.history.static_data || data.history.dynamic_history)) {
            // Store the token's static data globally for rendering Symbol and Address in every row.
            window.currentStaticData = data.history.static_data;
            
            // Show the position details section.
            document.getElementById('position-details').style.display = 'block';
            
            // Update overview cards with static and dynamic data.
            updateOverviewCards(data.history.static_data, data.history.dynamic_history);
            
            // Update the position history table with dynamic snapshot data.
            updatePositionTable(data.history.dynamic_history);
            
            // Update charts if applicable.
            if (!priceChart) {
                initializeCharts();
            }
            updateCharts(data.history.dynamic_history);
        } else {
            console.warn('No history data available:', data);
            showError('No position history available');
        }
    } catch (error) {
        console.error('Error loading position history:', error);
        showError('Failed to load position history');
    }
}

function updateOverviewCards(staticData, history) {
    if (!staticData || !history || history.length === 0) {
        console.log('No history data for cards');
        return;
    }
    
    const latest = history[history.length - 1];
    
    // Static Data Card
    document.getElementById('token-symbol').textContent = staticData.symbol || '';
    document.getElementById('token-currency').textContent = staticData.currency || '';
    document.getElementById('token-address').textContent = staticData.token_address ? 
        `${staticData.token_address.substring(0, 8)}...${staticData.token_address.slice(-6)}` : '';
    document.getElementById('creation-block').textContent = staticData.creation_block || '';
    document.getElementById('trading-enabled-block').textContent = staticData.trading_enabled_block || '';
    document.getElementById('entry-block').textContent = staticData.entry_block || '';
    document.getElementById('exit-block').textContent = staticData.exit_block || '';
    document.getElementById('entry-price').textContent = 
        staticData.entry_price_ratio ? parseFloat(staticData.entry_price_ratio).toFixed(2) : '';
    document.getElementById('exit-price').textContent = 
        staticData.exit_price_ratio ? parseFloat(staticData.exit_price_ratio).toFixed(2) : '';
    document.getElementById('purchase-value').textContent =
        staticData.purchase_value ? parseFloat(staticData.purchase_value).toFixed(2) : '';
    document.getElementById('entry-fee').textContent =
        staticData.entry_txn_fee ? parseFloat(staticData.entry_txn_fee).toFixed(2) : '';
    document.getElementById('exit-fee').textContent =
        staticData.exit_txn_fee ? parseFloat(staticData.exit_txn_fee).toFixed(2) : '';
    
    // Latest Snapshot Card
    document.getElementById('current-block').textContent = latest.block_number || '';
    document.getElementById('current-price').textContent = 
        latest.current_price_ratio ? parseFloat(latest.current_price_ratio).toFixed(2) : '';
    document.getElementById('current-value').textContent =
        latest.current_value ? parseFloat(latest.current_value).toFixed(2) : '';
    document.getElementById('position-state').textContent = latest.position_state || 'Unknown';
    document.getElementById('position-quantity').textContent =
        latest.quantity ? parseFloat(latest.quantity).toFixed(2) : '';
    document.getElementById('realized-profit').textContent =
        latest.realized_profit ? parseFloat(latest.realized_profit).toFixed(2) : '';
    document.getElementById('unrealized-profit').textContent =
        latest.unrealized_profit ? parseFloat(latest.unrealized_profit).toFixed(2) : '';
    document.getElementById('roi').textContent =
        latest.roi ? `${(latest.roi * 100).toFixed(1)}%` : '';
    document.getElementById('token-age-blocks').textContent = latest.token_age_blocks || '';
    document.getElementById('token-age-hours').textContent =
        latest.token_age_hours ? formatTradingAge(parseFloat(latest.token_age_hours)) : '';
    document.getElementById('scam-probability').textContent =
        latest.scam_probability ? `${(latest.scam_probability * 100).toFixed(1)}%` : '';
    document.getElementById('scam-reason').textContent = latest.scam_reason || '';
    document.getElementById('num-greys').textContent = latest.num_greys || '';
    document.getElementById('num-greens').textContent = latest.num_greens || '';
    document.getElementById('num-bribers').textContent = latest.num_bribers || '';
    document.getElementById('token-bribe-amount').textContent = 
        latest.token_bribe_amount ? parseFloat(latest.token_bribe_amount).toFixed(2) : '';
}

function initializeCharts() {
    if (priceChart) {
        priceChart.dispose();
    }
    
    priceChart = echarts.init(document.getElementById('priceChart'));
    priceChart.setOption({
        tooltip: {
            trigger: 'axis',
            axisPointer: {
                type: 'cross'
            }
        },
        legend: {
            data: ['Price Ratio']
        },
        grid: {
            left: '3%',
            right: '4%',
            bottom: '15%',
            containLabel: true
        },
        xAxis: [
            {
                type: 'value',
                name: 'Blocks',
                position: 'bottom',
                axisLine: { onZero: false }
            },
            {
                type: 'value',
                name: 'Hours',
                position: 'bottom',
                offset: 40,
                axisLine: { onZero: false },
                axisLabel: {
                    formatter: '{value} hrs'
                }
            }
        ],
        yAxis: {
            type: 'value',
            name: 'Price Ratio',
            axisLabel: {
                formatter: '{value}'
            }
        },
        series: [
            {
                name: 'Price Ratio',
                type: 'line',
                smooth: true,
                data: []
            }
        ]
    });
}

function updateCharts(history) {
    if (!history || history.length === 0) return;

    const blockNumbers = [];
    const hours = [];
    const prices = [];
    const markers = [];
    
    // Sort history by block number
    const sortedHistory = [...history].sort((a, b) => a.block_number - b.block_number);
    
    // Collect data points
    sortedHistory.forEach(record => {
        blockNumbers.push(record.block_number);
        hours.push(parseFloat(record.token_age_hours || 0));
        prices.push(parseFloat(record.current_Xprice || 0));
    });

    // Find entry and exit points
    const entryBlock = sortedHistory[0].entry_block;
    const entryPrice = sortedHistory.find(h => h.block_number === entryBlock)?.current_Xprice;
    
    const exitBlock = sortedHistory.find(h => h.position_state === 'SELL')?.block_number;
    const exitPrice = sortedHistory.find(h => h.block_number === exitBlock)?.current_Xprice;

    // Create the series data with markers
    const seriesData = prices.map((price, index) => {
        const point = {
            value: [blockNumbers[index], price],
            symbol: 'circle',
            symbolSize: 6
        };

        // Mark entry point
        if (blockNumbers[index] === entryBlock) {
            point.itemStyle = {
                color: '#91cc75',  // Green
                borderWidth: 2,
                borderColor: '#fff'
            };
            point.symbolSize = 12;
        }
        
        // Mark exit point
        if (blockNumbers[index] === exitBlock) {
            point.itemStyle = {
                color: '#ee6666',  // Red
                borderWidth: 2,
                borderColor: '#fff'
            };
            point.symbolSize = 12;
        }

        return point;
    });

    priceChart.setOption({
        xAxis: [
            {
                min: Math.min(...blockNumbers),
                max: Math.max(...blockNumbers)
            },
            {
                min: Math.min(...hours),
                max: Math.max(...hours)
            }
        ],
        series: [{
            name: 'Price Ratio',
            data: seriesData,
            markPoint: {
                data: [
                    {
                        name: 'Entry',
                        coord: [entryBlock, entryPrice],
                        value: 'Entry',
                        itemStyle: { color: '#91cc75' }
                    },
                    ...(exitBlock ? [{
                        name: 'Exit',
                        coord: [exitBlock, exitPrice],
                        value: 'Exit',
                        itemStyle: { color: '#ee6666' }
                    }] : [])
                ]
            }
        }]
    });
}

function onStrategyChange(event) {
    const strategyId = event.target.value;
    const tokenSelect = document.getElementById('token-select');
    
    if (strategyId) {
        loadTokens(strategyId);
    } else {
        tokenSelect.innerHTML = '<option value="">Select Token...</option>';
        tokenSelect.disabled = true;
        document.getElementById('position-details').style.display = 'none';
    }
}

function onTokenChange() {
    const strategyId = document.getElementById('strategy-select').value;
    const tokenAddress = document.getElementById('token-select').value;
    
    if (strategyId && tokenAddress) {
        console.log(`Token changed: Strategy=${strategyId}, Token=${tokenAddress}`);
        loadPositionHistory(strategyId, tokenAddress);
    } else {
        document.getElementById('position-details').style.display = 'none';
    }
}

function showError(message) {
    // Only show error if it's not already displayed
    const existingError = document.querySelector('.alert-danger');
    if (existingError) {
        existingError.remove();
    }
    
    console.warn(message); // Use warn instead of error for non-critical issues
    const alertDiv = document.createElement('div');
    alertDiv.className = 'alert alert-danger';
    alertDiv.textContent = message;
    document.querySelector('.container-fluid').prepend(alertDiv);
    
    // Remove the alert after 3 seconds
    setTimeout(() => {
        if (alertDiv.parentNode) {
            alertDiv.remove();
        }
    }, 3000);
}

function updatePositionTable(history) {
    if (!history || history.length === 0) {
        if (positionHistoryTable) {
            positionHistoryTable.clear().draw();
        }
        return;
    }
    
    if (positionHistoryTable) {
        positionHistoryTable.clear();
        positionHistoryTable.rows.add(history);
        positionHistoryTable.draw();
    } else {
        positionHistoryTable = $('#position-table').DataTable({
            data: history,
            columns: [
                { data: 'block_number', title: 'Block' },
                { data: 'position_state', title: 'State' },
                { 
                    data: 'current_value', 
                    title: 'Current Value', 
                    render: function(data) { return formatNumber(data, 2); } 
                },
                { 
                    data: 'current_price_ratio', 
                    title: 'Current Price Ratio', 
                    render: function(data) { return formatNumber(data, 2); } 
                },
                { data: 'token_age_blocks', title: 'Age (blocks)' },
                { data: 'token_age_hours', 
                    title: 'Age (hours)',
                    render: function(data) { return formatTradingAge(parseFloat(data)); }
                },
                { 
                    data: 'scam_probability', 
                    title: 'Scam Prob', 
                    render: function(data) { return data ? formatNumber(data * 100, 1) + '%' : '0%'; } 
                },
                { data: 'num_greys', title: 'Greys' },
                { data: 'num_greens', title: 'Greens' },
                { 
                    data: 'token_bribe_amount', 
                    title: 'Bribe Amount', 
                    render: function(data) { return data ? formatNumber(data, 2) : ''; } 
                },
            ],
            order: [[0, 'desc']],
            paging: true,
            responsive: true,           // Disable Responsive to allow horizontal scrolling
            scrollX: true,               // Enable horizontal scrolling
            scrollY: '300px',            // Set vertical scrolling height
            scrollCollapse: true,        // Collapse table height if fewer records
            autoWidth: false             // Disable autoWidth to force the column width settings
        });
    }
} 