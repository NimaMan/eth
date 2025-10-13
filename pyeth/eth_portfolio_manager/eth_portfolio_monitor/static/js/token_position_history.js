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
            // Log selected token data for debugging
            console.log('Selected token row data:', rowData);
            
            $(this).addClass('selected').siblings().removeClass('selected');
            
            // Use token address from static_data if present or fallback to latest_snapshot
            const tokenAddress = rowData.static_data.token_address || rowData.latest_snapshot.token_address;
            console.log('Selected token address:', tokenAddress);
            
            updateTokenStaticData(rowData.static_data);
            loadPositionHistory(currentStrategyId, tokenAddress);
        }
    });
    
    // Add click handler for token address copy functionality
    $(document).on('click', '#token-address-link', function(e) {
        const address = $(this).text();
        if (address && address !== '-') {
            e.preventDefault();
            
            // Create temporary textarea
            const tempInput = document.createElement('textarea');
            tempInput.value = address;
            document.body.appendChild(tempInput);
            tempInput.select();
            
            try {
                // Execute copy command
                document.execCommand('copy');
                
                // Provide visual feedback
                const originalTitle = $(this).attr('data-original-title');
                $(this).attr('data-original-title', 'Address copied!').tooltip('show');
                
                // Reset tooltip after a delay
                setTimeout(() => {
                    $(this).attr('data-original-title', originalTitle || 'Click to copy address');
                }, 1500);
            } catch (err) {
                console.error('Failed to copy address:', err);
            }
            
            document.body.removeChild(tempInput);
            return false;
        }
    });

    // Add strategy change handler
    document.getElementById('strategy-select').addEventListener('change', function(e) {
        const strategyId = e.target.value;
        if (strategyId) {
            loadTokens(strategyId);
        }
    });

    // Add window resize handler to ensure chart resizes properly
    window.addEventListener('resize', function() {
        if (priceChart) {
            console.log('Resizing chart due to window resize');
            priceChart.resize();
        }
    });

    // Add custom event for when position details are shown
    document.addEventListener('positionDetailsVisible', function() {
        if (priceChart) {
            console.log('Triggering chart resize when position details are visible');
            setTimeout(function() {
                priceChart.resize();
            }, 100);
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
    // Token Information Card - Symbol first, then address on the same line
    const addressLink = document.getElementById('token-address-link');
    const symbolElement = document.getElementById('token-symbol');
    
    if (staticData.token_address) {
        // Get token data
        const fullAddress = staticData.token_address;
        const symbol = staticData.symbol || '-';
        
        // Format: "SYMBOL: ADDRESS"
        symbolElement.textContent = symbol;
        addressLink.textContent = fullAddress;
        
        // Create DexScreener link
        const dexScreenerUrl = `https://dexscreener.com/ethereum/${fullAddress}`;
        addressLink.href = dexScreenerUrl;
        addressLink.title = 'View on DexScreener (click to copy address)';
        
        // Add click-to-copy functionality via data attribute and Bootstrap tooltip
        addressLink.setAttribute('data-toggle', 'tooltip');
        addressLink.setAttribute('data-placement', 'bottom');
        addressLink.setAttribute('data-original-title', 'Click to copy address');
        
        // Add a light styling class to make the address stand out
        addressLink.classList.add('text-primary');
        
        // Initialize the tooltip if jQuery and Bootstrap are available
        if (typeof $ !== 'undefined' && typeof $.fn.tooltip !== 'undefined') {
            $(addressLink).tooltip();
        }
    } else {
        symbolElement.textContent = staticData.symbol || '-';
        addressLink.textContent = '-';
        addressLink.href = '#';
        addressLink.classList.remove('text-primary');
        addressLink.removeAttribute('data-toggle');
        addressLink.removeAttribute('data-placement');
        addressLink.removeAttribute('data-original-title');
    }
    
    // Display currency
    document.getElementById('token-currency').textContent = staticData.currency || '-';
    
    // Creation block
    document.getElementById('creation-block').textContent = staticData.creation_block || '-';
    // Trading enabled block
    document.getElementById('trading-enabled-block').textContent = staticData.trading_enabled_block || '-';
    // Entry block
    document.getElementById('entry-block').textContent = staticData.entry_block || '-';
    // Exit block
    document.getElementById('exit-block').textContent = staticData.exit_block || '-';
    
    // FIX: Check for null or undefined values before calling toFixed()
    // Entry Price (format as needed)
    document.getElementById('entry-price').textContent = 
        (staticData.entry_price_ratio !== undefined && staticData.entry_price_ratio !== null) 
            ? parseFloat(staticData.entry_price_ratio).toFixed(2) 
            : '-';
    // Exit Price
    document.getElementById('exit-price').textContent = 
        (staticData.exit_price_ratio !== undefined && staticData.exit_price_ratio !== null) 
            ? parseFloat(staticData.exit_price_ratio).toFixed(2) 
            : '-';
    // Purchase value (using formatNumber utility)
    document.getElementById('purchase-value').textContent = 
        staticData.purchase_value ? formatNumber(staticData.purchase_value, 2) : '-';
    // Entry fee
    document.getElementById('entry-fee').textContent = 
        staticData.entry_tx_fee ? formatNumber(staticData.entry_tx_fee, 2) : '-';
    // Exit fee
    document.getElementById('exit-fee').textContent = 
        staticData.exit_tx_fee ? formatNumber(staticData.exit_tx_fee, 2) : '-';
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
        
        console.log('Position history data:', data); // Add debug log to check data structure
        
        if (data.history && (data.history.static_data || data.history.dynamic_history)) {
            // Store the token's static data globally for rendering Symbol and Address in every row.
            window.currentStaticData = data.history.static_data;
            
            // Show the position details section.
            document.getElementById('position-details').style.display = 'block';
            
            // Dispatch custom event that position details are visible
            document.dispatchEvent(new CustomEvent('positionDetailsVisible'));
            
            // Update overview cards with static and dynamic data.
            updateOverviewCards(data.history.static_data, data.history.dynamic_history);
            
            // Update the position history table with dynamic snapshot data.
            updatePositionTable(data.history.dynamic_history);
            
            // Initialize or update the chart
            if (!priceChart) {
                initializeCharts();
            } else {
                // Ensure proper chart refresh when selecting a new token
                console.log('Reinitializing chart for new token data');
                priceChart.dispose();
                priceChart = null;
                initializeCharts();
            }
            
            // Wait a bit to ensure chart container is fully visible and sized
            setTimeout(() => {
                console.log('Dynamic history for chart:', data.history.dynamic_history); // Add debug log
                updateCharts(data.history.dynamic_history);
                
                // Ensure chart is properly sized
                if (priceChart) {
                    console.log('Forcing chart resize after data update');
                    priceChart.resize();
                }
            }, 100);
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
    // Clear all fields first to ensure stale data doesn't persist
    const fieldsToClear = [
        // Token Information Card
        'token-symbol', 'token-currency', 'token-address-link', 'creation-block',
        'trading-enabled-block', 'entry-block', 'exit-block', 'entry-price',
        'exit-price', 'purchase-value', 'entry-fee', 'exit-fee',
        // Latest Snapshot Card
        'current-block', 'current-price', 'current-value', 'position-state',
        'position-quantity', 'realized-profit', 'unrealized-profit', 'roi',
        'token-age-blocks', 'token-age-hours', 'scam-info-greens',
        'scam-info-bribers', 'scam-info-bribe-amount',
        // Scam Info Card
        'scam-info-probability', 'scam-info-reason', 'scam-info-greys'
    ];
    
    fieldsToClear.forEach(id => {
        const element = document.getElementById(id);
        if (element) {
            if (id === 'token-address-link') {
                element.textContent = '-';
                element.href = '#';
            } else {
                element.textContent = '-';
            }
        }
    });
    
    // Check for valid data
    if (!staticData) {
        console.log('No static data available for overview cards');
        return;
    }
    
    if (!history || history.length === 0) {
        console.log('No history data for cards');
        return;
    }
    
    // Get the latest snapshot
    const latest = history[history.length - 1];
    console.log('Updating overview cards with latest snapshot:', latest);
    
    // Token Information Card - Symbol and address on same line
    const addressLink = document.getElementById('token-address-link');
    const symbolElement = document.getElementById('token-symbol');
    
    if (staticData.token_address) {
        // Get token data
        const fullAddress = staticData.token_address;
        const symbol = staticData.symbol || '-';
        
        // Format: "SYMBOL: ADDRESS"
        symbolElement.textContent = symbol;
        addressLink.textContent = fullAddress;
        
        // Create DexScreener link
        const dexScreenerUrl = `https://dexscreener.com/ethereum/${fullAddress}`;
        addressLink.href = dexScreenerUrl;
        addressLink.title = 'View on DexScreener (click to copy address)';
        
        // Add click-to-copy functionality via data attribute and Bootstrap tooltip
        addressLink.setAttribute('data-toggle', 'tooltip');
        addressLink.setAttribute('data-placement', 'bottom');
        addressLink.setAttribute('data-original-title', 'Click to copy address');
        
        // Add a light styling class to make the address stand out
        addressLink.classList.add('text-primary');
        
        // Initialize the tooltip if jQuery and Bootstrap are available
        if (typeof $ !== 'undefined' && typeof $.fn.tooltip !== 'undefined') {
            $(addressLink).tooltip();
        }
    } else {
        symbolElement.textContent = staticData.symbol || '-';
        addressLink.textContent = '-';
        addressLink.href = '#';
        addressLink.classList.remove('text-primary');
        addressLink.removeAttribute('data-toggle');
        addressLink.removeAttribute('data-placement');
        addressLink.removeAttribute('data-original-title');
    }
    
    // Display currency after symbol
    document.getElementById('token-currency').textContent = staticData.currency || '-';
    
    document.getElementById('creation-block').textContent = staticData.creation_block || '-';
    document.getElementById('trading-enabled-block').textContent = staticData.trading_enabled_block || '-';
    document.getElementById('entry-block').textContent = staticData.entry_block || '-';
    document.getElementById('exit-block').textContent = staticData.exit_block || '-';
    
    // FIX: Safe checks for price ratios and other numeric values
    document.getElementById('entry-price').textContent = 
        (staticData.entry_price_ratio !== undefined && staticData.entry_price_ratio !== null) 
            ? parseFloat(staticData.entry_price_ratio).toFixed(2) 
            : '-';
    document.getElementById('exit-price').textContent = 
        (staticData.exit_price_ratio !== undefined && staticData.exit_price_ratio !== null) 
            ? parseFloat(staticData.exit_price_ratio).toFixed(2) 
            : '-';
    document.getElementById('purchase-value').textContent =
        staticData.purchase_value ? formatNumber(staticData.purchase_value, 2) : '-';
    document.getElementById('entry-fee').textContent =
        staticData.entry_tx_fee ? formatNumber(staticData.entry_tx_fee, 2) : '-';
    document.getElementById('exit-fee').textContent =
        staticData.exit_tx_fee ? formatNumber(staticData.exit_tx_fee, 2) : '-';
    
    // Latest Snapshot Card
    if (latest) {
        document.getElementById('current-block').textContent = latest.block_number || '-';
        document.getElementById('current-price').textContent = 
            latest.current_price_ratio ? parseFloat(latest.current_price_ratio).toFixed(2) : '-';
        document.getElementById('current-value').textContent =
            latest.current_value ? parseFloat(latest.current_value).toFixed(2) : '-';
        document.getElementById('position-state').textContent = latest.position_state || 'Unknown';
        document.getElementById('position-quantity').textContent =
            latest.quantity ? parseFloat(latest.quantity).toFixed(2) : '-';
        document.getElementById('realized-profit').textContent =
            latest.realized_profit ? parseFloat(latest.realized_profit).toFixed(2) : '-';
        document.getElementById('unrealized-profit').textContent =
            latest.unrealized_profit ? parseFloat(latest.unrealized_profit).toFixed(2) : '-';
        document.getElementById('roi').textContent =
            latest.roi ? `${(latest.roi * 100).toFixed(1)}%` : '-';
        document.getElementById('token-age-blocks').textContent = latest.token_age_blocks || '-';
        document.getElementById('token-age-hours').textContent =
            latest.token_age_hours ? formatTradingAge(parseFloat(latest.token_age_hours)) : '-';
            
        // Update green holders, bribers and bribe amount in Current Position Status
        document.getElementById('scam-info-greens').textContent = latest.num_greens || '-';
        document.getElementById('scam-info-bribers').textContent = latest.num_bribers || '-';
        
        // Format bribe amount
        const bribeAmount = latest.token_bribe_amount;
        if (bribeAmount) {
            document.getElementById('scam-info-bribe-amount').textContent = formatNumber(bribeAmount, 2);
        } else {
            document.getElementById('scam-info-bribe-amount').textContent = '-';
        }

        // Update Scam Information Card
        updateScamInfoCard(latest);
    }
}

// New function to update the scam information card
function updateScamInfoCard(latestSnapshot) {
    if (!latestSnapshot) {
        console.log('No snapshot data for scam information card');
        return;
    }

    // Format scam probability as percentage with color coding
    const scamProbability = latestSnapshot.scam_probability;
    const scamProbElem = document.getElementById('scam-info-probability');
    
    if (scamProbability !== undefined && scamProbability !== null) {
        const probabilityPercentage = (scamProbability * 100).toFixed(1) + '%';
        scamProbElem.textContent = probabilityPercentage;
        
        // Color coding based on probability
        if (scamProbability >= 0.7) {
            scamProbElem.className = 'text-danger font-weight-bold'; // High risk
        } else if (scamProbability >= 0.4) {
            scamProbElem.className = 'text-warning font-weight-bold'; // Medium risk
        } else if (scamProbability > 0) {
            scamProbElem.className = 'text-info'; // Low risk
        } else {
            scamProbElem.className = 'text-success'; // No risk
        }
    } else {
        scamProbElem.textContent = '-';
        scamProbElem.className = '';
    }
    
    // Update scam reason with special handling for long text
    const scamReason = latestSnapshot.scam_reason;
    const scamReasonElem = document.getElementById('scam-info-reason');
    
    if (scamReason) {
        scamReasonElem.textContent = scamReason;
    } else {
        scamReasonElem.textContent = 'No scam reason provided';
    }
    
    // Update other scam-related metrics
    document.getElementById('scam-info-greys').textContent = latestSnapshot.num_greys || '-';
    document.getElementById('scam-info-greens').textContent = latestSnapshot.num_greens || '-';
    document.getElementById('scam-info-bribers').textContent = latestSnapshot.num_bribers || '-';
    
    // Format bribe amount
    const bribeAmount = latestSnapshot.token_bribe_amount;
    if (bribeAmount) {
        document.getElementById('scam-info-bribe-amount').textContent = formatNumber(bribeAmount, 2);
    } else {
        document.getElementById('scam-info-bribe-amount').textContent = '-';
    }
}

function clearPositionData() {
    // Clear the position history table
    if (positionHistoryTable) {
        positionHistoryTable.clear().draw();
    }
    
    // Clear the chart completely
    clearChart();
    
    // Clear all fields in the cards
    const fieldsToClear = [
        // Token Information Card - now directly handled below
        'token-currency', 'creation-block',
        'trading-enabled-block', 'entry-block', 'exit-block', 'entry-price',
        'exit-price', 'purchase-value', 'entry-fee', 'exit-fee',
        // Latest Snapshot Card
        'current-block', 'current-price', 'current-value', 'position-state',
        'position-quantity', 'realized-profit', 'unrealized-profit', 'roi',
        'token-age-blocks', 'token-age-hours', 'scam-info-greens',
        'scam-info-bribers', 'scam-info-bribe-amount',
        // Scam Info Card
        'scam-info-probability', 'scam-info-reason', 'scam-info-greys',
        // Old scam fields (for backward compatibility)
        'scam-probability', 'scam-reason', 'num-greys', 'num-greens',
        'num-bribers', 'token-bribe-amount'
    ];
    
    fieldsToClear.forEach(id => {
        const element = document.getElementById(id);
        if (element) {
            element.textContent = '-';
            // Reset any special styling that might have been applied
            element.className = element.className.replace(/(text-\w+|font-weight-\w+)/g, '').trim();
        }
    });
    
    // Special handling for the token elements - symbol and address on same line
    const symbolElement = document.getElementById('token-symbol');
    const addressLink = document.getElementById('token-address-link');
    const currencyElement = document.getElementById('token-currency');
    
    if (symbolElement) {
        symbolElement.textContent = '-';
        symbolElement.className = 'mb-0 mr-2 font-weight-bold';
    }
    
    if (addressLink) {
        addressLink.textContent = '-';
        addressLink.href = '#';
        // Keep the text-break class but reset other classes
        addressLink.className = 'text-break';
        addressLink.removeAttribute('data-toggle');
        addressLink.removeAttribute('data-placement');
        addressLink.removeAttribute('data-original-title');
    }
    
    if (currencyElement) {
        currencyElement.textContent = '-';
        currencyElement.className = 'badge badge-secondary';
    }
}

function initializeCharts() {
    if (priceChart) {
        priceChart.dispose();
    }
    
    console.log('Initializing price chart');
    
    // Wait for the DOM to be ready
    setTimeout(function() {
        const chartContainer = document.getElementById('priceChart');
        if (!chartContainer) {
            console.error('Chart container not found');
            return;
        }
        
        priceChart = echarts.init(chartContainer);
        console.log('Chart initialized with container size:', chartContainer.offsetWidth, chartContainer.offsetHeight);
        
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
                bottom: '3%',
                containLabel: true
            },
            xAxis: {
                type: 'value',
                name: 'Hours',
                axisLabel: {
                    formatter: '{value} hrs'
                }
            },
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
    }, 50);
}

function updateCharts(history) {
    if (!history || history.length === 0) {
        console.log('No history data for chart');
        return;
    }

    console.log('Updating chart with history:', history); // Add debug log

    // Sort history by block number to ensure chronological order
    const sortedHistory = [...history].sort((a, b) => a.block_number - b.block_number);
    
    // Prepare data arrays for the chart
    const blockNumbers = [];
    const hourData = [];
    const priceRatios = [];
    
    // Collect data points from history
    sortedHistory.forEach(record => {
        if (record.block_number && record.current_price_ratio !== undefined) {
            blockNumbers.push(record.block_number);
            hourData.push(parseFloat(record.token_age_hours || 0));
            priceRatios.push(parseFloat(record.current_price_ratio || 0));
        }
    });

    console.log('Processed chart data points count:', hourData.length);
    
    // Early return if we have no data points
    if (hourData.length === 0) {
        console.warn('No valid data points found for the chart');
        return;
    }

    console.log('Processed chart data:', { 
        hours: hourData, 
        prices: priceRatios,
        blocks: blockNumbers 
    }); // Add debug log
    
    // Find entry and exit blocks from static data and history
    const entryBlock = sortedHistory.find(h => h.position_state === 'BUY_CONFIRMED')?.block_number;
    const entryPrice = sortedHistory.find(h => h.block_number === entryBlock)?.current_price_ratio;
    
    const exitBlock = sortedHistory.find(h => h.position_state === 'SELL_CONFIRMED')?.block_number;
    const exitPrice = sortedHistory.find(h => h.block_number === exitBlock)?.current_price_ratio;

    console.log('Entry/Exit markers:', { 
        entryBlock,
        entryPrice,
        exitBlock,
        exitPrice 
    }); // Add debug log

    // Create series data with hours and price ratios
    const seriesData = hourData.map((hour, index) => {
        return [hour, priceRatios[index]];
    });

    // Create marker points for entry and exit if available
    const markPoints = [];
    if (entryBlock) {
        const entryIndex = blockNumbers.indexOf(entryBlock);
        if (entryIndex >= 0) {
            markPoints.push({
                name: 'Entry',
                coord: [hourData[entryIndex], priceRatios[entryIndex]],
                value: 'Entry',
                itemStyle: { color: '#91cc75' }
            });
        }
    }

    if (exitBlock) {
        const exitIndex = blockNumbers.indexOf(exitBlock);
        if (exitIndex >= 0) {
            markPoints.push({
                name: 'Exit',
                coord: [hourData[exitIndex], priceRatios[exitIndex]],
                value: 'Exit',
                itemStyle: { color: '#ee6666' }
            });
        }
    }

    // Update chart with new data
    priceChart.setOption({
        tooltip: {
            trigger: 'axis',
            axisPointer: {
                type: 'cross'
            },
            formatter: function(params) {
                const data = params[0].data;
                return `Hours: ${data[0].toFixed(1)}<br>Price Ratio: ${data[1].toFixed(2)}`;
            }
        },
        xAxis: {
            type: 'value',
            name: 'Hours',
            nameLocation: 'middle',
            nameGap: 30,
            axisLabel: {
                formatter: '{value} hrs'
            }
        },
        yAxis: {
            type: 'value',
            name: 'Price Ratio',
            nameLocation: 'middle',
            nameGap: 50,
            axisLabel: {
                formatter: '{value}'
            }
        },
        series: [{
            name: 'Price Ratio',
            type: 'line',
            data: seriesData,
            smooth: true,
            markPoint: {
                data: markPoints
            }
        }]
    }, true); // Add the true parameter to force chart update
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