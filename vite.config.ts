import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  build: {
    target: 'es2020',
  },
  server: {
    port: 3000,
    host: true,
  },
  plugins: [
    wasm(),
    topLevelAwait(),
  ],
  root: 'web',
});