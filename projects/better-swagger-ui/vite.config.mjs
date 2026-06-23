import path from 'node:path'
import { fileURLToPath } from 'node:url'
import sirv from 'sirv'
import { defineConfig } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import { closkell } from '@closkell/vite-plugin'

const fixturesRoot = path.join(fileURLToPath(new URL('.', import.meta.url)), 'tests/fixtures')

function serveTestFixtures() {
  const serve = sirv(fixturesRoot, {
    dev: true,
    setHeaders(res) {
      res.setHeader('Access-Control-Allow-Origin', '*')
    },
  })

  return {
    name: 'serve-test-fixtures',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url?.startsWith('/v3/api-docs')) {
          req.url = `/fixtures${req.url}`
        }
        if (!req.url?.startsWith('/fixtures')) return next()
        req.url = req.url.slice('/fixtures'.length) || '/'
        serve(req, res, next)
      })
    },
  }
}

function targetFromProxyPath(pathname) {
  try {
    const target = new URL(pathname, 'http://localhost').searchParams.get('url')
    if (!target) return null
    const url = new URL(target)
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return null
    return url
  } catch {
    return null
  }
}

export default defineConfig(({ mode }) => ({
  plugins: [
    tailwindcss(),
    closkell({
      entry: 'src/app.clsk',
      rootId: 'root',
      css: 'src/styles.css',
      sourceRoot: 'src',
      manifestPath: '../../Cargo.toml',
      vendorRuntime: true,
    }),
    serveTestFixtures(),
  ],
  resolve: {
    extensions: ['.mjs', '.js', '.mts', '.ts', '.jsx', '.tsx', '.json', '.clsk'],
  },
  server:
    mode === 'proxy'
      ? {
          proxy: {
            '/__proxy': {
              target: 'http://localhost',
              changeOrigin: true,
              secure: false,
              configure(proxy, options) {
                options.rewrite = (pathname) => {
                  const url = targetFromProxyPath(pathname)
                  if (!url) return pathname
                  options.target = url.origin
                  return url.pathname + url.search
                }
                proxy.on('proxyReq', (proxyReq) => {
                  proxyReq.removeHeader('cookie')
                })
              },
            },
          },
        }
      : undefined,
  build: {
    target: 'es2022',
  },
}))
