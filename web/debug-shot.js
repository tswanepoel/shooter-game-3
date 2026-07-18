// Dev screenshot sink: disk write via Vite middleware (debug/shots/).

/**
 * Install window.__debugSaveShot for the WASM client (debug-tools builds).
 * POST /__debug/shot → debug/shots/latest.png + timestamped copy.
 */
export function installDebugShotSink() {
  window.__debugSaveShot = async (dataUrl) => {
    if (typeof dataUrl !== 'string' || !dataUrl.startsWith('data:image/png')) {
      console.error('debug shot: expected PNG data URL');
      return;
    }

    try {
      const res = await fetch('/__debug/shot', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ dataUrl }),
      });
      if (!res.ok) {
        const text = await res.text();
        console.warn('debug shot: disk sink failed', res.status, text);
        return;
      }
      const info = await res.json();
      console.log('debug shot saved', info);
    } catch (err) {
      console.warn('debug shot: disk sink unavailable (use npm run dev)', err);
    }
  };
}
