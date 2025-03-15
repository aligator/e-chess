// Function to fetch and update the board
function updateBoard() {
    fetch('/board-update')
        .then(response => response.text())
        .then(html => {
            document.getElementById('board-container').innerHTML = html;
            
            // Check if we have a game loaded
            const hasGame = !html.includes('No game loaded');
            updateUIVisibility(hasGame);
            
            // Hide loading indicator if it's visible
            hideLoadingIndicator();
        })
        .catch(error => {
            console.error('Error updating board:', error);
            hideLoadingIndicator();
        });
}

// Function to fetch and update the game info
function updateGameInfo() {
    fetch('/game-info')
        .then(response => response.json())
        .then(data => {
            document.getElementById('game-status').textContent = data.status;
            document.getElementById('active-player').textContent = 'Active player: ' + data.activePlayer;
            
            // Check if we have a game loaded
            const hasGame = data.status !== 'No game';
            updateUIVisibility(hasGame);
        })
        .catch(error => {
            console.error('Error updating game info:', error);
        });
}

// Function to show loading indicator
function showLoadingIndicator() {
    const loadingIndicator = document.getElementById('loading-indicator');
    const noGameMessage = document.getElementById('no-game-message');
    
    if (loadingIndicator) {
        loadingIndicator.classList.remove('hidden');
    }
    
    if (noGameMessage) {
        noGameMessage.classList.add('hidden');
    }
}

// Function to hide loading indicator
function hideLoadingIndicator() {
    const loadingIndicator = document.getElementById('loading-indicator');
    if (loadingIndicator) {
        loadingIndicator.classList.add('hidden');
    }
}

// Function to update UI visibility based on game state
function updateUIVisibility(hasGame) {
    // Show/hide game info
    const gameInfo = document.getElementById('game-info');
    if (gameInfo) {
        gameInfo.classList.toggle('hidden', !hasGame);
    }
    
    // Show/hide no game message
    const noGameMessage = document.getElementById('no-game-message');
    if (noGameMessage) {
        noGameMessage.classList.toggle('hidden', hasGame);
    }
    
    // Show/hide refresh control
    const refreshControl = document.getElementById('refresh-control');
    if (refreshControl) {
        refreshControl.classList.toggle('hidden', !hasGame);
    }
    
    // Always hide loading indicator when updating UI
    hideLoadingIndicator();
}

// Function to schedule updates
function scheduleUpdates() {
    if (document.getElementById('autoRefresh') && document.getElementById('autoRefresh').checked) {
        setTimeout(function() {
            updateBoard();
            updateGameInfo();
            scheduleUpdates();
        }, 1000);
    }
}

// Set up event listeners when DOM is loaded
document.addEventListener('DOMContentLoaded', function() {
    // Auto refresh checkbox
    const autoRefreshCheckbox = document.getElementById('autoRefresh');
    if (autoRefreshCheckbox) {
        autoRefreshCheckbox.addEventListener('change', function() {
            if (this.checked) {
                scheduleUpdates();
            }
        });
    }

    // Load game button
    const loadGameButton = document.getElementById('loadGame');
    if (loadGameButton) {
        loadGameButton.addEventListener('click', function() {
            const gameId = document.getElementById('gameId').value.trim();
            if (gameId) {
                // Show loading indicator
                showLoadingIndicator();
                
                fetch('/load-game?id=' + encodeURIComponent(gameId), {
                    method: 'GET'
                }).then(function(response) {
                    if (response.ok) {
                        updateBoard();
                        updateGameInfo();
                    } else {
                        alert('Failed to load game. Please check the game ID.');
                        // Show no game message again
                        hideLoadingIndicator();
                        const noGameMessage = document.getElementById('no-game-message');
                        if (noGameMessage) {
                            noGameMessage.textContent = "No game loaded. Please enter a game ID below to load a game.";
                            noGameMessage.classList.remove('hidden');
                        }
                    }
                }).catch(function(error) {
                    alert('Error: ' + error);
                    // Show no game message again
                    hideLoadingIndicator();
                    const noGameMessage = document.getElementById('no-game-message');
                    if (noGameMessage) {
                        noGameMessage.textContent = "No game loaded. Please enter a game ID below to load a game.";
                        noGameMessage.classList.remove('hidden');
                    }
                });
            } else {
                alert('Please enter a valid game ID');
            }
        });
    }

    // Start the update cycle if auto-refresh is checked
    if (autoRefreshCheckbox && autoRefreshCheckbox.checked) {
        // Delay the first update slightly to ensure the page is fully loaded
        setTimeout(function() {
            updateBoard();
            updateGameInfo();
            scheduleUpdates();
        }, 200);
    }
}); 