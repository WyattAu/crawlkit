import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    // Default node environment suits the api_client/store tests; component
    // tests select jsdom via a per-file docblock (see Button.test.tsx).
    environment: 'node',
    globals: true,
  },
});
