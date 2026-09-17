import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Mirrors docker/nginx/gateway.dev.conf's routing exactly, so `pnpm dev` also
// works standalone against the four services running locally (no gateway
// container needed for day-to-day frontend work) — see docs/STATUS.md's
// microservices entry for why the split means the proxy must be path-based
// instead of one fixed target.
const IDENTITY = 'http://localhost:8281';
const COMMERCE = 'http://localhost:8282';
const NOTIFICATIONS = 'http://localhost:8283';
const MEDIA = 'http://localhost:8284';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5190,
    proxy: {
      '^/api/v1/(auth|onboarding|admin)/': { target: IDENTITY, changeOrigin: true, rewrite: (p) => p.replace(/^\/api/, '') },
      '^/api/v1/(warehouses|catalog|cart|orders)': { target: COMMERCE, changeOrigin: true, rewrite: (p) => p.replace(/^\/api/, '') },
      '^/api/v1/(notifications|devices)': { target: NOTIFICATIONS, changeOrigin: true, rewrite: (p) => p.replace(/^\/api/, '') },
      '^/api/v1/media/': { target: MEDIA, changeOrigin: true, rewrite: (p) => p.replace(/^\/api/, '') },
    },
  },
});
