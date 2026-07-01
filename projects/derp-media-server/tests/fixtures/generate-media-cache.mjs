import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { generateTestMedia } from './generate-media.mjs'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const ROOT = path.resolve(path.join(__dirname, '..', '..'))
const MEDIA_CACHE_DIR = path.join(ROOT, '.test-media-cache')

if (fs.existsSync(MEDIA_CACHE_DIR)) {
  fs.rmSync(MEDIA_CACHE_DIR, { recursive: true, force: true })
}
fs.mkdirSync(MEDIA_CACHE_DIR, { recursive: true })
console.log('Generating test media into .test-media-cache ...')
generateTestMedia(MEDIA_CACHE_DIR)
console.log('Done. Run test:batch to use the cache for faster setup.')
