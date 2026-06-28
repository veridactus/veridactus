import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// 环境变量驱动代理目标（vite 默认加载 .env / .env.local）
// 变量名: VITE_CP_HOST (默认 127.0.0.1:8081), VITE_DP_HOST (默认 127.0.0.1:8080), VITE_PW_HOST (默认 127.0.0.1:8001)
const cpTarget = `http://${process.env.VITE_CP_HOST || '127.0.0.1:8081'}`;
const dpTarget = `http://${process.env.VITE_DP_HOST || '127.0.0.1:8080'}`;
const pwTarget = `http://${process.env.VITE_PW_HOST || '127.0.0.1:8001'}`;

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    proxy: {
      '/api/v1': {
        target: cpTarget,
        changeOrigin: true,
      },
      '/v1': {
        target: dpTarget,
        changeOrigin: true,
      },
      '/health': {
        target: dpTarget,
        changeOrigin: true,
      },
      '/models': {
        target: dpTarget,
        changeOrigin: true,
        // SPA fallback：浏览器页面刷新时不代理，交给 React Router
        bypass(req) {
          const accept = req.headers.accept || '';
          if (accept.includes('text/html')) return '/index.html';
        },
      },
      '/pw': {
        target: pwTarget,
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/pw/, ''),
      },
    },
  },
  test: {
    globals: true,
    environment: 'happy-dom',
    setupFiles: path.resolve(__dirname, 'src/test/setup.ts'),
    css: true,
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
