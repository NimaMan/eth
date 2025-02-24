/**
 * Utility Functions Module
 *
 * Objective:
 * ----------
 * Centralize common helper functions for formatting numbers, formatting trading age,
 * and displaying error messages across multiple modules.
 *
 * Functions included:
 * - formatTradingAge(totalHours): Formats a number of hours into a string "Xh Ym".
 * - formatNumber(value, decimals): Formats a number with a fixed number of decimals.
 * - showError(message, options): Displays an error message in the specified container.
 */

/**
 * Formats a number of hours into a string "Xh Ym".
 *
 * @param {number} totalHours - The total trading age in hours.
 * @return {string} - Formatted string in the form "Xh Ym".
 */
function formatTradingAge(totalHours) {
    const hours = Math.floor(totalHours);
    const minutes = Math.round((totalHours - hours) * 60);
    return `${hours}h ${minutes}m`;
}

/**
 * Formats a number with a specified number of decimals.
 *
 * @param {number|string|null} value - The value to format.
 * @param {number} decimals - The number of decimal places (defaults to 2).
 * @return {string} - The formatted number as a string.
 */
function formatNumber(value, decimals = 2) {
    return parseFloat(value || 0).toFixed(decimals);
}

/**
 * Displays an error message on the page.
 *
 * @param {string} message - The error message to display.
 * @param {object} [options] - Optional settings: { container: string, duration: number }.
 *   - container: A CSS selector for the container in which to display the error (default: '.container').
 *   - duration: How long (in milliseconds) before the error is removed (default: 5000).
 */
function showError(message, options = {}) {
    const containerSelector = options.container || '.container';
    const duration = options.duration || 5000;
    
    // Remove any existing error messages in the specified container.
    const existingError = document.querySelector(`${containerSelector} .alert.alert-danger`);
    if (existingError) {
        existingError.remove();
    }
    
    console.warn(message);
    
    const alertDiv = document.createElement('div');
    alertDiv.className = 'alert alert-danger';
    alertDiv.textContent = message;
    
    const container = document.querySelector(containerSelector) || document.body;
    container.prepend(alertDiv);
    
    setTimeout(() => {
        if (alertDiv.parentNode) {
            alertDiv.remove();
        }
    }, duration);
}

// Export functions if using CommonJS modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        formatTradingAge,
        formatNumber,
        showError
    };
} 