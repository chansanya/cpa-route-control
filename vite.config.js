import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      '/cpa-management': {
        target: process.env.CPA_DEV_TARGET || 'http://127.0.0.1:8317',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/cpa-management/, '/v0/management')
      },
      '/management.html': {
        target: process.env.CPA_DEV_TARGET || 'http://127.0.0.1:8317',
        changeOrigin: true
      },
      '/v0/management': {
        target: process.env.CPA_DEV_TARGET || 'http://127.0.0.1:8317',
        changeOrigin: true
      }
    }
  }
});
