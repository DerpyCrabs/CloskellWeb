import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import { closkell } from '@closkell/vite-plugin'

const projectRoot = fileURLToPath(new URL('.', import.meta.url))

export default defineConfig({
  plugins: [
    tailwindcss(),
    closkell({
      entry: 'src/app.clsk',
      rootId: 'root',
      css: 'src/globals.css',
      manifestPath: '../../Cargo.toml',
      vendorRuntime: true,
      sourceMap: false,
    }),
  ],
  server: {
    watch: {
      ignored: ['**/kb-chats.json', '**/shares.json', '**/settings.json', '**/stats.json'],
    },
  },
  resolve: {
    alias: [
      {
        find: /^@\/lib\/closkell\/(.*)$/,
        replacement: path.resolve(projectRoot, '.closkell/build/lib/closkell/$1'),
      },
      {
        find: '@',
        replacement: projectRoot,
      },
    ],
  },
  build: {
    outDir: 'dist/client',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: {
        app: path.resolve(projectRoot, 'index.html'),
        'sse-shared-worker': path.resolve(
          projectRoot,
          '.closkell/build/lib/closkell/sse-shared-worker.mjs',
        ),
      },
      output: {
        entryFileNames: (chunk) =>
          chunk.name === 'sse-shared-worker'
            ? 'assets/sse-shared-worker.js'
            : 'assets/[name]-[hash].js',
      },
    },
  },
})
