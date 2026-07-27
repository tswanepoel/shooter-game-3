/** Vite with HTTPS so LAN clients get a secure context (WebGPU). */
import { spawn } from 'node:child_process';
import process from 'node:process';

const env = { ...process.env, DEV_HTTPS: '1' };
const child = spawn('npx', ['vite'], { stdio: 'inherit', shell: true, env });
child.on('exit', (code) => process.exit(code ?? 1));
