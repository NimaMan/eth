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
                const createdAt = new Date(run.created_at).toLocaleString();
                option.textContent = `${run.name} (${createdAt}) - Blocks: ${run.start_block}-${run.end_block}`;
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
        const response = await fetch(`/api/backtest/positions/${strategyRunId}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        console.log("Positions data:", data); // Debug log
        
        if (!data || !data.positions) {
            throw new Error('Invalid positions data format');
        }

        // Calculate portfolio metrics
        let totalValue = 0;
        let totalRealizedProfit = 0;
        let totalUnrealizedProfit = 0;

        const positions = data.positions.map(position => {
            // Ensure numeric values
            position.current_value = parseFloat(position.current_value) || 0;
            position.realized_profit = parseFloat(position.realized_profit) || 0;
            position.unrealized_profit = parseFloat(position.unrealized_profit) || 0;
            
            // Update totals
            totalValue += position.current_value;
            totalRealizedProfit += position.realized_profit;
            totalUnrealizedProfit += position.unrealized_profit;
            
            return position;
        });

        // Update portfolio stats
        const metrics = {
            total_value: totalValue,
            realized_profit: totalRealizedProfit,
            unrealized_profit: totalUnrealizedProfit,
            total_profit_loss: totalRealizedProfit + totalUnrealizedProfit
        };
        
        updatePortfolioStats(metrics);
        
        // Group and update tables
        const groupedPositions = groupPositionsByState(positions);
        updatePositionTables(groupedPositions);

    } catch (error) {
        console.error('Error loading positions:', error);
        showError('Failed to load positions');
    }
}

function groupPositionsByState(positions) {
    // Initialize groups based on TokenPositionState enum values
    const grouped = {
        Init: [],           // TokenPositionState.INIT
        BuySubmitted: [],   // TokenPositionState.BUY_SUBMITTED
        BuyConfirmed: [],   // TokenPositionState.BUY_CONFIRMED
        SellSubmitted: [],  // TokenPositionState.SELL_SUBMITTED
        SellConfirmed: [],  // TokenPositionState.SELL_CONFIRMED
        Scammed: []         // TokenPositionState.SCAMMED
    };
    
    positions.forEach(position => {
        // Match the position states from TokenPositionState enum
        switch(position.position_state) {
            case 'Init':
                grouped.Init.push(position);
                break;
            case 'Buy Submitted':
                grouped.BuySubmitted.push(position);
                break;
            case 'Buy Confirmed':
                grouped.BuyConfirmed.push(position);
                break;
            case 'Sell Submitted':
                grouped.SellSubmitted.push(position);
                break;
            case 'Sell Confirmed':
                grouped.SellConfirmed.push(position);
                break;
            case 'Scammed':
                grouped.Scammed.push(position);
                break;
            default:
                console.warn(`Unknown position state: ${position.position_state}`);
                grouped.Init.push(position);
        }
    });
    
    return grouped;
}

function updatePositionTables(groupedPositions) {
    // Update Init positions
    updateTable('init-positions-table', groupedPositions.Init);
    
    // Update Buy state positions (combine submitted and confirmed)
    const buyPositions = [...groupedPositions.BuySubmitted, ...groupedPositions.BuyConfirmed];
    updateTable('buy-positions-table', buyPositions);
    
    // Update Sell state positions (combine submitted, confirmed, and scammed)
    const sellPositions = [
        ...groupedPositions.SellSubmitted,
        ...groupedPositions.SellConfirmed,
        ...groupedPositions.Scammed
    ];
    updateTable('sell-positions-table', sellPositions);
    
    // Update position counts
    document.getElementById('init-count').textContent = groupedPositions.Init.length;
    document.getElementById('buy-count').textContent = buyPositions.length;
    document.getElementById('sell-count').textContent = sellPositions.length;
}

function updateTable(tableId, positions) {
    const tbody = document.getElementById(tableId).getElementsByTagName('tbody')[0];
    tbody.innerHTML = '';
    
    positions.forEach(position => {
        // Parse numeric values
        const realizedProfit = parseFloat(position.realized_profit) || 0;
        const unrealizedProfit = parseFloat(position.unrealized_profit) || 0;
        
        const row = tbody.insertRow();
        row.innerHTML = `
            <td><a href="https://dexscreener.com/ethereum/${position.token_address}" target="_blank">${position.token_address.substring(0, 8)}...</a></td>
            <td>${position.symbol || 'UNKNOWN'}</td>
            <td>${position.token_age_blocks || 0}</td>
            <td>${(position.token_age_hours || 0).toFixed(2)}</td>
            <td>${position.position_state || 'Init'}</td>
            <td>${(position.current_value || 0).toFixed(2)}</td>
            <td>${(position.current_Xprice || 0).toFixed(2)}</td>
            <td class="${realizedProfit >= 0 ? 'text-success' : 'text-danger'}">${realizedProfit.toFixed(2)}</td>
            <td class="${unrealizedProfit >= 0 ? 'text-success' : 'text-danger'}">${unrealizedProfit.toFixed(2)}</td>
            <td>${position.num_greys || 0}</td>
            <td>${position.num_greens || 0}</td>
            <td>${((position.scam_probability || 0) * 100).toFixed(2)}%</td>
            <td>${position.scam_reason || 'NA'}</td>
        `;
    });
}

function updatePortfolioStats(metrics) {
    document.getElementById('total-value').textContent = metrics.total_value.toFixed(2);
    document.getElementById('realized-profit').textContent = metrics.realized_profit.toFixed(2);
    document.getElementById('unrealized-profit').textContent = metrics.unrealized_profit.toFixed(2);
    document.getElementById('total-pl').textContent = metrics.total_profit_loss.toFixed(2);
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