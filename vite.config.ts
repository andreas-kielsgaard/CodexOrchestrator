import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';

const runtimeRoot = process.env.WORKTREE_RUNTIME_ROOT;
const runtimePort = Number.parseInt(process.env.WORKTREE_VITE_PORT ?? '1420', 10);

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  ...(runtimeRoot ? { cacheDir: path.join(runtimeRoot, 'vite-cache') } : {}),
  build: {
    ...(runtimeRoot ? { outDir: path.join(runtimeRoot, 'dist') } : {}),
    rollupOptions: {
      input: {
        app: 'index.html',
        agentSessionHarness: 'agent-session-harness.html',
      },
    },
  },
  server: {
    host: '127.0.0.1',
    port: runtimePort,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: './src/test/setup.ts',
  },
});
