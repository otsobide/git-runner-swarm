import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [vue()],
  server: {
    // `npm run dev` serves the panel and hands the API to a server running
    // beside it, `GRS_RUNTIME=memory cargo run` for one.
    proxy: { '/api': 'http://127.0.0.1:8080' },
  },
  test: {
    environment: 'jsdom',
  },
})
