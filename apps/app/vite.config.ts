import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Keep the development server local and give the immutable Tauri config a separate overlay.
const productionCsp =
  "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";
const securityHeaders = {
  'X-Content-Type-Options': 'nosniff',
  'X-Frame-Options': 'DENY',
  'Referrer-Policy': 'no-referrer',
  'Permissions-Policy': 'camera=(), microphone=(), geolocation=(), payment=()',
};

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  base: './',
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
    headers: {
      ...securityHeaders,
      'Content-Security-Policy': productionCsp
        .replace("script-src 'self'", "script-src 'self' 'unsafe-inline'")
        .replace(
          "connect-src 'self'",
          "connect-src 'self' ws://127.0.0.1:1420 ws://localhost:1420",
        ),
    },
  },
  preview: {
    host: '127.0.0.1',
    port: 1421,
    strictPort: true,
    headers: { ...securityHeaders, 'Content-Security-Policy': productionCsp },
  },
  build: { target: 'es2022', sourcemap: false },
});
