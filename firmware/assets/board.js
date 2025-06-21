// Function to fetch and update game data (board and game info combined)
function updateGameData() {
    fetch('/game-data')
        .then(response => response.json())
        .then(data => {
            // Check game state
            const isLoaded = data.isLoaded;

            // Update UI visibility based on game state
            updateUIVisibility(isLoaded);

            // Update game info
            document.getElementById('game-status').textContent = data.status;
            document.getElementById('active-player').textContent = `Active player: ${data.activePlayer || 'None'}`;

            // Update board HTML
            document.getElementById('board-container').innerHTML = data.boardHtml || "";

            // Check if this is the game we're waiting for
            if (window.requestedGameKey && isLoaded && data.gameKey === window.requestedGameKey) {
                console.log('Game loaded successfully:', data.gameKey);

                // Update the text field with the loaded game key
                const gameKeyInput = document.getElementById('gameKey');
                if (gameKeyInput) {
                    gameKeyInput.value = data.gameKey;
                }

                // Enable the button
                loadingFinished();
            }
        })
        .catch(error => {
            console.error('Error updating game data:', error);
            updateUIVisibility(false);
            loadingFinished();
        });
}

// Function to disable load button and show loading indicator inside it
function disableLoadButton() {
    const loadGameButton = document.getElementById('loadGame');
    if (loadGameButton) {
        loadGameButton.disabled = true;
        loadGameButton.classList.add('loading');
        loadGameButton.innerHTML = '<span class="button-loading-indicator"></span> Loading...';
    }
}

// Function to enable load button and restore its text
function loadingFinished() {
    // Reset the requested game Key
    window.requestedGameKey = null;

    const loadGameButton = document.getElementById('loadGame');
    if (loadGameButton) {
        loadGameButton.disabled = false;
        loadGameButton.classList.remove('loading');
        loadGameButton.textContent = 'Load Game';
    }
}

// Function to update UI visibility
function updateUIVisibility(isLoaded) {
    // Show/hide game info
    const gameInfo = document.getElementById('game-info');
    if (gameInfo) {
        gameInfo.classList.toggle('hidden', !isLoaded);
    }

    // Show/hide board container
    const boardContainer = document.getElementById('board-container');
    if (boardContainer) {
        boardContainer.classList.toggle('hidden', !isLoaded);
    }
}

// Function to load a game with the given game key
function loadGame(gameKey) {
    if (!gameKey) {
        alert('Please enter a valid game Key');
        return;
    }

    // Disable the button and show loading indicator inside it
    disableLoadButton();

    updateUIVisibility(false);

    // Clear the board immediately to provide visual feedback
    const boardContainer = document.getElementById('board-container');
    if (boardContainer) {
        boardContainer.innerHTML = '';
    }

    fetch('/load-game?key=' + encodeURIComponent(gameKey), {
        method: 'GET'
    }).then(function (response) {
        if (response.ok) {
            console.log('Load game request sent, waiting for game to load...');

            // Store the requested game Key
            window.requestedGameKey = gameKey;
        } else {
            alert('Failed to load game. Please check the game Key.');
            loadingFinished();

            // Update UI to show no game
            updateUIVisibility(false);
        }
    }).catch(function (error) {
        alert('Error: ' + error);
        loadingFinished();

        // Update UI to show no game
        updateUIVisibility(false);
    });
}

// Set up event listeners when DOM is loaded
document.addEventListener('DOMContentLoaded', function () {
    // Initial update to get the current game state
    updateGameData();

    // Set up a single interval for updates
    setInterval(function () {
        updateGameData();
    }, 1000);

    // Load game button
    const loadGameButton = document.getElementById('loadGame');
    if (loadGameButton) {
        loadGameButton.addEventListener('click', function () {
            const gameKey = document.getElementById('gameKey').value.trim();
            loadGame(gameKey);
        });
    }

    // New game button
    const newGameButton = document.getElementById('newGame');
    if (newGameButton) {
        newGameButton.addEventListener('click', function () {
            // Standard chess starting position FEN
            const standardFen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
            loadGame(standardFen);
        });
    }
}); 