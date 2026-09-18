import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Fast local dev loop: run the four backend services + gateway in Docker
// (docker/compose.dev.yml) as usual, then `npm run dev` here for instant
// HMR on changes — no `web` container rebuild needed. The dev server runs
// on its own port (5191) and proxies all /api traffic to the already-running
// gateway container (localhost:5190), which fans it out to whichever
// service backs each path (see docker/nginx/gateway.dev.conf). Because the
// proxy is server-side, the browser only ever talks to :5191 — no CORS
// config changes needed. identity/commerce/notifications/media don't
// publish fixed host ports themselves (see compose.dev.yml's comment on
// why: they're scaled during rolling updates), so the gateway is the only
// stable target to proxy to from outside Docker.
const GATEWAY = `http://localhost:${process.env.SL_DEV_GATEWAY_PORT ?? 5190}`;

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5191,
    proxy: {
      '^/api/': { target: GATEWAY, changeOrigin: true },
    },
  },
});
