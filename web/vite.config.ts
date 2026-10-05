import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Server-only configuration. No Kernel credential is read or bundled by Vite.
const target = process.env.LAIN_DEV_KERNEL_URL ?? 'http://127.0.0.1:8787';
const kernel = new URL(target);
if (!['http:', 'https:'].includes(kernel.protocol) || kernel.username || kernel.password) {
  throw new Error('LAIN_DEV_KERNEL_URL must be an HTTP(S) URL without credentials');
}
export default defineConfig({
  plugins: [react()],
  server: {
    host: '127.0.0.1',
    proxy: Object.fromEntries(['/v1', '/health', '/ready'].map(path => [path, { target, changeOrigin: false }])),
  },
  build: { sourcemap: false },
});
