// Initialize tables
let initTable, buyTable, sellTable;

document.addEventListener('DOMContentLoaded', function() {
    // Initialize all three tables with the same configuration
    const tableConfig = {
        paging: false,
        scrollY: '400px',
        scrollCollapse: true,
        order: [[2, 'desc']], // Sort by current value descending
        dom: 'Bfrtip',
        buttons: ['copy', 'csv', 'excel'],
        info: false,
        searching: true,
        columns: [
            { 
                data: 'symbol',
                defaultContent: 'UNKNOWN'
            },
            { 
                data: 'position_state',
                defaultContent: 'Unknown'
            },
            { 
                data: 'current_value',
                render: (data) => parseFloat(data || 0).toFixed(2)
            },
            { 
                data: 'current_Xprice',
                render: (data) => parseFloat(data || 0).toFixed(2)
            },
            { 
                data: 'token_age_blocks',
                defaultContent: '0'
            },
            { 
                data: 'token_age_hours',
                render: (data) => Math.floor(data || 0)
            },
            { 
                data: 'scam_reason',
                defaultContent: 'NA'
            },
            { 
                data: 'scam_probability',
                render: (data) => `${(parseFloat(data || 0) * 100).toFixed(2)}%`,
                defaultContent: '0.00%'
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
                render: function(data) {
                    return `<a href="/position/${data}" class="text-primary">${data.substring(0, 8)}...</a>`;
                },
                defaultContent: ''
            }
        ]
    };

    // Initialize tables
    initTable = $('#init-positions-table').DataTable(tableConfig);
    buyTable = $('#buy-positions-table').DataTable(tableConfig);
    sellTable = $('#sell-positions-table').DataTable(tableConfig);

    // Load initial data
    loadPositions();
});

async function loadPositions() {
    try {
        const response = await fetch('/api/backtest/positions');
        const data = await response.json();

        if (!data || !data.positions) {
            throw new Error('Invalid data format received');
        }

        // Clear all tables
        initTable.clear();
        buyTable.clear();
        sellTable.clear();

        // Sort positions into respective tables
        Object.values(data.positions).forEach(position => {
            const state = position.position_state?.toLowerCase() || '';
            if (state === 'init') {
                initTable.row.add(position);
            } else if (state.includes('buy')) {
                buyTable.row.add(position);
            } else if (state.includes('sell')) {
                sellTable.row.add(position);
            }
        });

        // Draw all tables
        initTable.draw();
        buyTable.draw();
        sellTable.draw();

        // Update counts
        document.getElementById('init-count').textContent = initTable.rows().count();
        document.getElementById('buy-count').textContent = buyTable.rows().count();
        document.getElementById('sell-count').textContent = sellTable.rows().count();

        // Update portfolio stats
        document.getElementById('total-value').textContent = 
            parseFloat(data.metrics?.total_value || 0).toFixed(2);
        document.getElementById('total-pl').textContent = 
            parseFloat(data.metrics?.total_profit_loss || 0).toFixed(2);

    } catch (error) {
        console.error('Error loading positions:', error);
        showError('Failed to load positions: ' + error.message);
    }
}

function showError(message) {
    console.error(message);
    const alertDiv = document.createElement('div');
    alertDiv.className = 'alert alert-danger';
    alertDiv.textContent = message;
    document.querySelector('.container').prepend(alertDiv);
    setTimeout(() => alertDiv.remove(), 5000);
}

function updatePositionsTable() {
    console.log('Fetching positions...');
    fetch('/api/positions')
        .then(response => response.json())
        .then(data => {
            console.log('Received data:', data);
            
            // Update metrics
            document.getElementById('total-value').textContent = data.metrics.total_value.toFixed(4);
            document.getElementById('total-pl').textContent = data.metrics.total_profit_loss.toFixed(4);
            document.getElementById('position-count').textContent = data.metrics.position_count;

            // Initialize DataTable if not already initialized
            if (!$.fn.DataTable.isDataTable('#positions-table')) {
                $('#positions-table').DataTable({
                    pageLength: -1,
                    lengthMenu: [[10, 25, 50, -1], [10, 25, 50, "All"]],
                    order: [[3, 'desc']],
                    dom: 'Bfrtip',
                    columns: [
                        { title: "#", data: null, render: (data, type, row, meta) => meta.row + 1 },
                        { title: "Symbol", data: "symbol" },
                        { title: "State", data: "state", orderable: false },
                        { title: "Current Value", data: "current_value" },
                        { title: "Unrealized P/L", data: "unrealized_profit" },
                        { title: "Entry Price", data: "entry_price" },
                        { title: "Current Price", data: "current_price" },
                        { title: "Quantity", data: "quantity" },
                        { title: "Age", data: "age" },
                        { title: "Last Updated", data: "last_updated" },
                        { title: "Address", data: "contract_address", orderable: false }
                    ]
                });
            }

            let table = $('#positions-table').DataTable();

            const positionsArray = Object.entries(data.positions)
                .map(([key, pos]) => {
                    // Clean up the contract address by removing "position:" prefix if it exists
                    const cleanAddress = pos.contract_address.replace(/^position:/, '');
                    // Get characters 2-8 if address starts with 0x
                    const shortAddress = cleanAddress.startsWith('0x') 
                        ? cleanAddress.slice(2, 8) 
                        : cleanAddress.slice(0, 6);
                    
                    return {
                        symbol: pos.symbol,
                        state: `<span class="badge bg-${getStateBadgeColor(pos.state)}">${pos.state}</span>`,
                        current_value: pos.current_value,
                        unrealized_profit: pos.unrealized_profit,
                        entry_price: `${pos.entry_price}x`,
                        current_price: `${pos.current_price}x`,
                        quantity: pos.quantity,
                        age: pos.age,
                        last_updated: new Date(pos.last_updated).toLocaleString(),
                        contract_address: `<a href="https://dexscreener.com/ethereum/${cleanAddress}" 
                                        target="_blank" rel="noopener noreferrer" 
                                        title="${cleanAddress}">
                                        ${shortAddress}
                                     </a>`
                    };
                });

            table.clear();
            table.rows.add(positionsArray);
            table.draw();
        })
        .catch(error => {
            console.error('Error fetching positions:', error);
        });
}

function getStateBadgeColor(state) {
    switch(state.toLowerCase()) {
        case 'active': return 'success';
        case 'init': return 'secondary';
        case 'buy submitted': return 'warning';
        case 'sell submitted': return 'warning';
        case 'sell confirmed': return 'danger';
        case 'scammed': return 'dark';
        default: return 'info';
    }
}

// Update every 12 seconds
document.addEventListener('DOMContentLoaded', function() {
    updatePositionsTable();
    setInterval(updatePositionsTable, 12000);
}); 