// Initialize charts
let valueChart = null;
let priceChart = null;
let tokenTable = null;
let currentStrategyId = null;

document.addEventListener('DOMContentLoaded', function() {
    // Initialize the token table with DataTables
    tokenTable = $('#token-list-table').DataTable({
        order: [[2, 'desc']], // Sort by age descending by default
        paging: true,
        pageLength: 10,
        scrollY: '300px',
        scrollCollapse: true,
        columns: [
            { data: 'symbol', title: 'Symbol' },
            { 
                data: 'token_address',
                title: 'Address',
                render: function(data) {
                    return `<a href="https://dexscreener.com/ethereum/${data}" target="_blank">${data.substring(0, 8)}...</a>`;
                }
            },
            { 
                data: 'token_age_hours',
                title: 'Age (hours)',
                render: function(data) {
                    return data ? Math.floor(data) : '0';
                }
            },
            { data: 'position_state', title: 'State' },
            { 
                data: 'current_value',
                title: 'Current Value',
                render: function(data) {
                    return parseFloat(data || 0).toFixed(4);
                }
            }
        ],
        select: true
    });

    // Add row click handler
    $('#token-list-table tbody').on('click', 'tr', function() {
        const data = tokenTable.row(this).data();
        if (data) {
            loadPositionHistory(currentStrategyId, data.token_address);
            
            // Update visual selection
            $(this).addClass('selected').siblings().removeClass('selected');
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
                const createdAt = new Date(run.created_at).toLocaleString();
                option.textContent = `${run.name} (${createdAt}) - Blocks: ${run.start_block}-${run.end_block}`;
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

async function loadTokens(strategyId) {
    try {
        currentStrategyId = strategyId;
        const response = await fetch(`/api/backtest/latest-strategy-positions/${strategyId}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        // Clear and reload the token table
        tokenTable.clear();
        tokenTable.rows.add(data.positions).draw();
        
        // Select and load the first token by default
        if (data.positions && data.positions.length > 0) {
            const firstToken = data.positions[0];
            loadPositionHistory(strategyId, firstToken.token_address);
            
            // Highlight the first row
            $(tokenTable.row(0).node()).addClass('selected');
        }
        
        // Show the position details section
        document.getElementById('position-details').style.display = 'block';
        
    } catch (error) {
        console.error('Error loading tokens:', error);
        showError('Failed to load tokens');
    }
}

async function loadPositionHistory(strategyId, tokenAddress) {
    try {
        console.log(`Loading history for strategy ${strategyId} and token ${tokenAddress}`);
        const response = await fetch(`/api/backtest/position-history/${strategyId}/${tokenAddress}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        
        if (!data.history || !Array.isArray(data.history)) {
            console.error('Invalid history data:', data);
            return;
        }
        
        // Show the position details section
        document.getElementById('position-details').style.display = 'block';
        
        // Update overview cards with static data
        updateOverviewCards(data.history);
        
        // Initialize and update the position history table
        if (!$.fn.DataTable.isDataTable('#position-table')) {
            $('#position-table').DataTable({
                data: data.history,
                order: [[0, 'asc']], // Sort by block number ascending
                pageLength: 25,
                scrollY: '400px',
                scrollCollapse: true,
                columns: [
                    { 
                        data: 'symbol',
                        title: 'Symbol'
                    },
                    { 
                        data: 'position_state',
                        title: 'State'
                    },
                    { 
                        data: 'current_value',
                        title: 'Current Value',
                        render: function(data) {
                            return parseFloat(data || 0).toFixed(4);
                        }
                    },
                    { 
                        data: 'current_Xprice',
                        title: 'Current XPrice',
                        render: function(data) {
                            return parseFloat(data || 0).toFixed(8);
                        }
                    },
                    { 
                        data: 'token_age_blocks',
                        title: 'Age (blocks)'
                    },
                    { 
                        data: 'token_age_hours',
                        title: 'Age (hours)',
                        render: function(data) {
                            return parseFloat(data || 0).toFixed(2);
                        }
                    },
                    { 
                        data: 'scam_reason',
                        title: 'Scam Reason',
                        defaultContent: 'NA'
                    },
                    { 
                        data: 'scam_probability',
                        title: 'Scam Prob',
                        render: function(data) {
                            return `${((data || 0) * 100).toFixed(2)}%`;
                        }
                    },
                    { 
                        data: 'num_greys',
                        title: 'Greys',
                        defaultContent: '0'
                    },
                    { 
                        data: 'num_greens',
                        title: 'Greens',
                        defaultContent: '0'
                    },
                    { 
                        data: 'token_address',
                        title: 'Address',
                        render: function(data) {
                            return `<a href="https://dexscreener.com/ethereum/${data}" target="_blank">${data.substring(0, 8)}...</a>`;
                        }
                    }
                ]
            });
        } else {
            // If table already exists, just update the data
            const table = $('#position-table').DataTable();
            table.clear();
            table.rows.add(data.history).draw();
        }
        
        // Initialize and update charts
        if (!priceChart) {
            initializeCharts();
        }
        updateCharts(data.history);
        
        console.log(`Updated table with ${data.history.length} records`);
        
    } catch (error) {
        console.error('Error loading position history:', error);
        showError('Failed to load position history');
    }
}

function updateOverviewCards(history) {
    if (!history || history.length === 0) {
        console.log('No history data for cards');
        return;
    }
    
    console.log('Updating cards with history:', history);
    
    // Get the latest entry for static information
    const latest = history[history.length - 1];
    
    // Entry Details
    document.getElementById('position-entry-block').textContent = latest.entry_block || '0';
    document.getElementById('position-current-block').textContent = latest.block_number || '0';
    document.getElementById('position-entry-price').textContent =
        latest.entry_price ? parseFloat(latest.entry_price).toFixed(8) : '0.00000000';
    document.getElementById('position-entry-value').textContent =
        latest.purchase_value ? parseFloat(latest.purchase_value).toFixed(4) : '0.0000';
    document.getElementById('position-currency').textContent =
        latest.currency ? latest.currency : '';
    
    // Current Position
    document.getElementById('position-quantity').textContent =
        latest.quantity ? parseFloat(latest.quantity).toFixed(4) : '0.0000';
    document.getElementById('position-state').textContent = latest.position_state || 'Unknown';
    document.getElementById('position-scam-prob').textContent =
        latest.scam_probability ? `${(latest.scam_probability * 100).toFixed(1)}%` : '0%';
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
            data: ['XPrice']
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
            name: 'XPrice',
            axisLabel: {
                formatter: '{value}'
            }
        },
        series: [
            {
                name: 'XPrice',
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
            name: 'XPrice',
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
    console.error(message);
    const alertDiv = document.createElement('div');
    alertDiv.className = 'alert alert-danger';
    alertDiv.textContent = message;
    document.querySelector('.container-fluid').prepend(alertDiv);
    
    // Remove the alert after 5 seconds
    setTimeout(() => alertDiv.remove(), 5000);
} 