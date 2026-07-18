import * as fs from 'node:fs';
import * as path from 'node:path';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { defineConfig, type Plugin } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
import { fileURLToPath } from 'node:url';

const repoRoot = path.dirname(fileURLToPath(import.meta.url));
const shotsDir = path.join(repoRoot, 'debug', 'shots');

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    req.on('data', (c) => chunks.push(Buffer.isBuffer(c) ? c : Buffer.from(c)));
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')));
    req.on('error', reject);
  });
}

/** Dev-only: POST /__debug/shot { dataUrl } → debug/shots/latest.png + timestamped copy. */
function debugShotsPlugin(): Plugin {
  return {
    name: 'debug-shots',
    configureServer(server) {
      server.middlewares.use(async (req: IncomingMessage, res: ServerResponse, next) => {
        const url = req.url?.split('?')[0];
        if (url !== '/__debug/shot' || req.method !== 'POST') {
          next();
          return;
        }

        try {
          const raw = await readBody(req);
          const body = JSON.parse(raw) as { dataUrl?: string };
          const dataUrl = body.dataUrl;
          if (typeof dataUrl !== 'string' || !dataUrl.startsWith('data:image/png;base64,')) {
            res.statusCode = 400;
            res.setHeader('Content-Type', 'application/json');
            res.end(JSON.stringify({ error: 'expected data:image/png;base64,...' }));
            return;
          }

          const b64 = dataUrl.slice('data:image/png;base64,'.length);
          const buf = Buffer.from(b64, 'base64');
          fs.mkdirSync(shotsDir, { recursive: true });

          const latest = path.join(shotsDir, 'latest.png');
          fs.writeFileSync(latest, buf);

          const stamp = new Date().toISOString().replace(/[:.]/g, '-');
          const archived = path.join(shotsDir, `shot-${stamp}.png`);
          fs.writeFileSync(archived, buf);

          res.statusCode = 200;
          res.setHeader('Content-Type', 'application/json');
          res.end(
            JSON.stringify({
              latest: path.relative(repoRoot, latest).replace(/\\/g, '/'),
              archived: path.relative(repoRoot, archived).replace(/\\/g, '/'),
              bytes: buf.length,
            }),
          );
        } catch (err) {
          res.statusCode = 500;
          res.setHeader('Content-Type', 'application/json');
          res.end(JSON.stringify({ error: String(err) }));
        }
      });
    },
  };
}

export default defineConfig({
  build: {
    target: 'es2020',
  },
  // Serve cook outputs only (packs + manifest). Source kits are not public.
  // Sole ship tree is web/dist (Vite chunks under /assets/* + this publicDir).
  publicDir: path.resolve(repoRoot, 'assets', 'cooked'),
  server: {
    port: 3000,
    host: true,
    fs: {
      allow: [repoRoot],
    },
  },
  plugins: [wasm(), topLevelAwait(), debugShotsPlugin()],
  root: 'web',
});
