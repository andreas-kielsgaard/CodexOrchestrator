import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [
    react(),
    {
      name: 'native-review-entry',
      transformIndexHtml: {
        order: 'pre',
        handler(html) {
          return html.replace('/src/main.tsx', '/src/nativeReview.ts');
        },
      },
    },
  ],
  clearScreen: false,
  build: {
    outDir: 'dist-native-review',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        app: 'index.html',
      },
    },
  },
});
