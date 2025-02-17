// Initialize charts
let valueChart = null;
let priceChart = null;

document.addEventListener('DOMContentLoaded', function() {
    // Load strategies when page loads
    loadStrategies();
    
    // Add event listeners
    document.getElementById('strategy-select').addEventListener('change', onStrategyChange);
    document.getElementById('token-select').addEventListener('change', onTokenChange);
    
    // Initialize DataTable with specific columns
    $('#position-table').DataTable({
        order: [[0, 'desc']], // Sort by block number descending
        paging: false,        // Disable pagination
        scrollY: '600px',     // Enable vertical scrolling with fixed height
        scrollCollapse: true, // Enable scroll collapse
        columns: [
            { 
                data: 'block_number',
                title: 'Block',
                defaultContent: '0'
            },
            { 
                data: 'position_state', 
                title: 'State',
                defaultContent: 'Unknown'
            },
            { 
                data: 'current_value', 
                title: 'Current Value',
                render: function(data) {
                    return parseFloat(data || 0).toFixed(6);
                },
                defaultContent: '0.000000'
            },
            { 
                data: 'current_Xprice', 
                title: 'Current XPrice',
                render: function(data) {
                    return parseFloat(data || 0).toFixed(6);
                },
                defaultContent: '0.000000'
            },
            { 
                data: 'token_age_blocks',
                title: 'Age (blocks)',
                defaultContent: '0'
            },
            { 
                data: 'token_age_hours',
                title: 'Age (hours)',
                render: function(data) {
                    return data ? Math.floor(data) : '0';
                },
                defaultContent: '0'
            },
            { 
                data: 'scam_reason',
                title: 'Scam Reason',
                defaultContent: 'NA'
            },
            { 
                data: 'num_greys',
                title: 'Grey Count',
                defaultContent: '0'
            },
            { 
                data: 'num_greens',
                title: 'Green Count',
                defaultContent: '0'
            }
        ],
        dom: 'Bfrtip',        // Add buttons for export
        buttons: [
            'copy', 'csv', 'excel' // Add export buttons
        ],
        info: false,          // Hide info about showing X of Y entries
        searching: true       // Keep search functionality
    });
});

async function loadStrategies() {
    try {
        // Updated to match existing API endpoint
        const response = await fetch('/api/backtest/strategy-runs');
        const data = await response.json();
        
        const select = document.getElementById('strategy-select');
        select.innerHTML = '<option value="">Select Strategy...</option>';
        
        data.strategy_runs.forEach(run => {
            const option = document.createElement('option');
            option.value = run.id;
            const startDate = new Date(run.start_time).toLocaleString();
            const profitLoss = (run.total_realized_profit + run.total_unrealized_profit).toFixed(2);
            option.textContent = `${run.strategy_name || 'Unnamed'} - ${startDate} (P/L: ${profitLoss} ETH)`;
            select.appendChild(option);
        });
    } catch (error) {
        console.error('Error loading strategies:', error);
        showError('Failed to load strategies');
    }
}

async function loadTokens(strategyId) {
    try {
        const response = await fetch(`/api/backtest/positions/${strategyId}`);
        const data = await response.json();
        
        const select = document.getElementById('token-select');
        select.innerHTML = '<option value="">Select Token...</option>';
        select.disabled = false;
        
        Object.entries(data.positions).forEach(([address, position]) => {
            const option = document.createElement('option');
            option.value = address;
            const symbol = position.symbol || 'UNKNOWN';
            // Updated format to show symbol-address
            option.textContent = `${symbol}-${address}`;
            select.appendChild(option);
        });

        select.disabled = Object.keys(data.positions).length === 0;
        console.log(`Loaded ${Object.keys(data.positions).length} tokens for strategy ${strategyId}`);
    } catch (error) {
        console.error('Error loading tokens:', error);
        showError('Failed to load tokens');
    }
}

async function loadPositionHistory(strategyId, tokenAddress) {
    try {
        console.log(`Loading history for strategy ${strategyId} and token ${tokenAddress}`);
        const response = await fetch(`/api/backtest/position-history/${strategyId}/${tokenAddress}`);
        const data = await response.json();
        
        console.log('Received history data:', data); // Debug log
        
        if (!data.history || !Array.isArray(data.history)) {
            console.error('Invalid history data:', data);
            return;
        }

        // Show the position details section
        document.getElementById('position-details').style.display = 'block';

        // Update cards with the data
        updateOverviewCards(data.history);
        
        // Get DataTable instance and update
        const table = $('#position-table').DataTable();
        
        // Clear existing data
        table.clear();
        
        // Add new data and redraw
        table.rows.add(data.history).draw();
        
        // Update charts if they exist
        if (document.getElementById('valueChart')) {
            updateCharts(data.history);
        }
        
        console.log(`Updated table with ${data.history.length} records`);
        
    } catch (error) {
        console.error('Error loading position history:', error);
        showError('Failed to load position history');
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

function updateOverviewCards(history) {
    if (!history || history.length === 0) {
        console.log('No history data for cards');
        return;
    }
    
    console.log('Updating cards with history:', history); // Debug log
    
    const latest = history[history.length - 1];
    
    // Entry Details
    document.getElementById('position-entry-block').textContent = latest.entry_block || '0';
    document.getElementById('position-current-block').textContent = latest.block_number || '0';
    document.getElementById('position-entry-price').textContent = 
        latest.entry_Xprice ? parseFloat(latest.entry_Xprice).toFixed(6) : '0.000000';
    document.getElementById('position-entry-value').textContent = 
        latest.purchase_value ? parseFloat(latest.purchase_value).toFixed(4) : '0.0000';

    // Current Position
    document.getElementById('position-quantity').textContent = 
        latest.quantity ? parseFloat(latest.quantity).toFixed(4) : '0.0000';
    document.getElementById('position-age').textContent = 
        latest.token_age_hours ? `${Math.floor(latest.token_age_hours)}h` : '0h';
    document.getElementById('position-scam-prob').textContent = 
        latest.scam_probability ? `${(latest.scam_probability * 100).toFixed(1)}%` : '0%';

    // Performance
    const totalPL = (parseFloat(latest.realized_profit || 0) + parseFloat(latest.unrealized_profit || 0));
    document.getElementById('position-total-pl').textContent = totalPL.toFixed(4);
    document.getElementById('position-realized').textContent = 
        latest.realized_profit ? parseFloat(latest.realized_profit).toFixed(4) : '0.0000';
    document.getElementById('position-unrealized').textContent = 
        latest.unrealized_profit ? parseFloat(latest.unrealized_profit).toFixed(4) : '0.0000';
}

function updateCharts(history) {
    // Update value chart
    const valueCtx = document.getElementById('valueChart').getContext('2d');
    if (valueChart) valueChart.destroy();
    
    valueChart = new Chart(valueCtx, {
        type: 'line',
        data: {
            labels: history.map(h => h.block_number),
            datasets: [{
                label: 'Position Value (ETH)',
                data: history.map(h => h.current_value),
                borderColor: 'rgb(75, 192, 192)',
                tension: 0.1
            }]
        },
        options: {
            responsive: true,
            scales: {
                y: {
                    beginAtZero: true
                }
            }
        }
    });
    
    // Update price chart
    const priceCtx = document.getElementById('priceChart').getContext('2d');
    if (priceChart) priceChart.destroy();
    
    priceChart = new Chart(priceCtx, {
        type: 'line',
        data: {
            labels: history.map(h => h.block_number),
            datasets: [{
                label: 'Token Price (ETH)',
                data: history.map(h => h.current_price),
                borderColor: 'rgb(153, 102, 255)',
                tension: 0.1
            }]
        },
        options: {
            responsive: true,
            scales: {
                y: {
                    beginAtZero: true
                }
            }
        }
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