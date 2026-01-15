function createCharts(data) {
    createPieChart(data);
    createLineChart(data);
}

function createPieChart(data) {
    const dates = Object.keys(data);
    const launches = dates.map(date => data[date].total_launches);
    const scams = dates.map(date => data[date].total_scams);
    
    const totalLaunches = launches.reduce((a, b) => a + b, 0);
    const totalScams = scams.reduce((a, b) => a + b, 0);
    
    const pieChart = echarts.init(document.getElementById('pieChart'));
    const pieOption = {
        tooltip: { trigger: 'item' },
        legend: {
            orient: 'vertical',
            left: 'left'
        },
        series: [{
            name: 'Token Distribution',
            type: 'pie',
            radius: '70%',
            data: [
                { 
                    value: totalLaunches - totalScams, 
                    name: 'Legit Tokens', 
                    itemStyle: { color: '#75b798' }
                },
                { 
                    value: totalScams, 
                    name: 'Scam Tokens', 
                    itemStyle: { color: '#ea868f' }
                }
            ],
            emphasis: {
                itemStyle: {
                    shadowBlur: 10,
                    shadowOffsetX: 0,
                    shadowColor: 'rgba(0, 0, 0, 0.5)'
                }
            }
        }]
    };
    pieChart.setOption(pieOption);
}

function createLineChart(data) {
    const dates = Object.keys(data).sort();
    const series = [{
        name: 'Total Launches',
        type: 'line',
        smooth: true,
        data: dates.map(date => data[date].total_launches)
    }, {
        name: 'Total Scams',
        type: 'line',
        smooth: true,
        data: dates.map(date => data[date].total_scams)
    }];
    
    // Add series for each scam type
    const scamTypes = new Set();
    dates.forEach(date => {
        Object.keys(data[date].scam_types || {}).forEach(type => scamTypes.add(type));
    });
    
    scamTypes.forEach(type => {
        series.push({
            name: type,
            type: 'line',
            smooth: true,
            data: dates.map(date => data[date].scam_types?.[type] || 0)
        });
    });
    
    const lineChart = echarts.init(document.getElementById('lineChart'));
    const lineOption = {
        tooltip: { trigger: 'axis' },
        legend: {
            data: series.map(s => s.name),
            top: 'bottom'
        },
        grid: {
            left: '3%',
            right: '4%',
            bottom: '15%',
            containLabel: true
        },
        xAxis: {
            type: 'category',
            data: dates
        },
        yAxis: {
            type: 'value'
        },
        series: series
    };
    lineChart.setOption(lineOption);
}

function updateSummaryCards(data) {
    const dates = Object.keys(data);
    const launches = dates.map(date => data[date].total_launches);
    const scams = dates.map(date => data[date].total_scams);
    
    const totalLaunches = launches.reduce((a, b) => a + b, 0);
    const totalScams = scams.reduce((a, b) => a + b, 0);
    const avgScamRatio = totalScams / totalLaunches || 0;
    
    // Get most common scam type across all dates
    const scamTypes = {};
    dates.forEach(date => {
        Object.entries(data[date].scam_types || {}).forEach(([type, count]) => {
            scamTypes[type] = (scamTypes[type] || 0) + count;
        });
    });
    
    const mostCommonScam = Object.entries(scamTypes)
        .sort(([,a], [,b]) => b - a)[0]?.[0] || 'None';
    
    // Update summary cards with proper formatting
    document.getElementById('total-launches').textContent = totalLaunches.toLocaleString();
    document.getElementById('total-scams').textContent = totalScams.toLocaleString();
    document.getElementById('avg-scam-ratio').textContent = 
        `${(avgScamRatio * 100).toFixed(1)}%`;
    document.getElementById('common-scam').textContent = mostCommonScam;
}

window.addEventListener('resize', () => {
    const pieChart = echarts.getInstanceByDom(document.getElementById('pieChart'));
    const lineChart = echarts.getInstanceByDom(document.getElementById('lineChart'));
    if (pieChart) pieChart.resize();
    if (lineChart) lineChart.resize();
});