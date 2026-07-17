// Main application entry point
import init from '../pkg/game_client.js';

async function run() {
    try {
        // Initialize the WASM module
        await init();
        
        // The main function is automatically called by the WASM module
        // when it's initialized, but we can also call it explicitly if needed
        console.log('Application initialized successfully');
    } catch (error) {
        console.error('Failed to initialize application:', error);
    }
}

// Start the application
run();