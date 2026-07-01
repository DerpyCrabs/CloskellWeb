import type { FullConfig } from '@playwright/test'
import fs from 'fs'
import path from 'path'
import { scryptSync, createCipheriv, randomBytes } from 'crypto'
import { generateTestMedia, patchTestMediaAfterCacheCopy } from './generate-media.mjs'

const batchId = process.env.BATCH_ID
const mediaDirName = batchId ? `test-media-${batchId}` : 'test-media'
const dataDirName = batchId ? `test-data-${batchId}` : null

const TEST_MEDIA_DIR = path.resolve(mediaDirName)
const MEDIA_CACHE_DIR = path.resolve(__dirname, '..', '..', '.test-media-cache')
const MEDIA_DIR_KEY = mediaDirName
const DATA_DIR = dataDirName ? path.resolve(dataDirName) : null

function encryptPasscode(passcode: string): string {
  const key = scryptSync('test-password', 'derp-media-server-passcode-v1', 32)
  const iv = randomBytes(12)
  const cipher = createCipheriv('aes-256-gcm', key, iv)
  const encrypted = Buffer.concat([cipher.update(passcode, 'utf8'), cipher.final()])
  const authTag = cipher.getAuthTag()
  return Buffer.concat([iv, authTag, encrypted]).toString('base64url')
}

function mergeJsonFile(filePath: string, data: Record<string, unknown>) {
  let existing: Record<string, unknown> = {}
  try {
    existing = JSON.parse(fs.readFileSync(filePath, 'utf-8'))
  } catch {
    // file doesn't exist yet
  }
  Object.assign(existing, data)
  fs.writeFileSync(filePath, JSON.stringify(existing, null, 2))
}

function dataFilePath(filename: string): string {
  return DATA_DIR ? path.join(DATA_DIR, filename) : path.resolve(filename)
}

export default async function setup(_config: FullConfig) {
  console.log(`[e2e${batchId ? `:${batchId}` : ''}] Setting up test fixtures...`)

  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      if (fs.existsSync(TEST_MEDIA_DIR)) {
        fs.rmSync(TEST_MEDIA_DIR, { recursive: true, force: true })
      }
      break
    } catch {
      // eslint-disable-next-line no-await-in-loop -- retry delay between rm attempts
      if (attempt < 2) await new Promise((r) => setTimeout(r, 1000))
      else throw new Error(`Failed to clean ${mediaDirName} directory after 3 attempts`)
    }
  }

  if (fs.existsSync(MEDIA_CACHE_DIR)) {
    fs.cpSync(MEDIA_CACHE_DIR, TEST_MEDIA_DIR, { recursive: true })
  } else {
    generateTestMedia(TEST_MEDIA_DIR)
  }
  patchTestMediaAfterCacheCopy(TEST_MEDIA_DIR)

  if (DATA_DIR) {
    fs.mkdirSync(DATA_DIR, { recursive: true })
  }

  mergeJsonFile(dataFilePath('settings.json'), {
    [MEDIA_DIR_KEY]: {
      viewModes: {},
      favorites: [],
      knowledgeBases: ['Notes'],
      customIcons: {},
      autoSave: {},
    },
  })

  mergeJsonFile(dataFilePath('shares.json'), {
    [MEDIA_DIR_KEY]: {
      shares: [
        {
          token: 'test-passcode-share-token1',
          path: 'SharedContent',
          isDirectory: true,
          editable: false,
          passcode: encryptPasscode('secret123'),
          createdAt: Date.now(),
        },
      ],
    },
  })

  mergeJsonFile(dataFilePath('stats.json'), {
    [MEDIA_DIR_KEY]: { views: {}, shareViews: {} },
  })

  console.log(`[e2e${batchId ? `:${batchId}` : ''}] Test fixtures ready.`)
}
