// Main application entry point: load WASM, bind canvas, start the client loop.
import init, { GameClient } from '../pkg/game_client.js';

async function run() {
  const canvas = document.getElementById('game-canvas');
  if (!(canvas instanceof HTMLCanvasElement)) {
    throw new Error('Missing #game-canvas element');
  }

  await init();

  const client = await GameClient.create(canvas);

  // Dev host bridge: same command/cvar registry as the in-engine console.
  if (typeof client.debugHost === 'function') {
    window.__DEBUG__ = client.debugHost();
  }

  client.startRenderLoop();
  console.log('Game client render loop started');
}

run().catch((error) => {
  console.error('Failed to start application:', error);
});
