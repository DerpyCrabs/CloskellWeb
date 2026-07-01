import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const BATCHES = [
  {
    id: '1',
    tests: ['workspace-layout-snap-resize', 'workspace-layout-chrome'],
  },
  {
    id: '2',
    tests: [
      'workspace-controls',
      'navigation',
      'breadcrumbs-adaptive',
      'upload',
      'file-browser-misc',
    ],
  },
  {
    id: '3',
    tests: [
      'workspace-viewers',
      'workspace-media-layout',
      'workspace-cross-dnd',
      'workspace-taskbar-pins',
      'workspace-taskbar-chrome',
      'workspace-layout-sessions',
      'workspace-named-layouts',
      'workspace-file-open-target',
    ],
  },
  {
    id: '4',
    tests: [
      'editable-folders',
      'share-security',
      'url-state',
      'login',
      'share-workspace',
      'share-browser-parity',
      'multiple-media-dirs',
    ],
  },
  {
    id: '5',
    tests: ['audio-player', 'shares-manage', 'shares-use', 'share-audio-api', 'sse-live-updates'],
  },
  {
    id: '6',
    tests: [
      'video-player',
      'image-viewer',
      'pdf-viewer',
      'download',
      'text-editor',
      'knowledge-base',
      'drag-drop',
      'passcode-shares',
      'share-viewers',
    ],
  },
]

const ROOT = path.resolve(__dirname, '..')
const FIXTURES_DIR = path.join(__dirname, 'fixtures')
const PLAYWRIGHT_CLI = path.join(ROOT, 'node_modules', 'playwright', 'cli.js')

function generateBatchConfig(batchId, port) {
  const configPath = path.join(FIXTURES_DIR, `test-config-${batchId}.jsonc`)
  const config = {
    mediaDir: `test-media-${batchId}`,
    dataPath: `../../test-data-${batchId}`,
    editableFolders: ['Notes', 'SharedContent'],
    shareLinkDomain: `http://localhost:${port}`,
    auth: { enabled: true, password: 'test-password' },
  }
  fs.writeFileSync(configPath, JSON.stringify(config, null, 2))
  return configPath
}

function cleanupBatchConfig(batchId) {
  const configPath = path.join(FIXTURES_DIR, `test-config-${batchId}.jsonc`)
  try {
    fs.unlinkSync(configPath)
  } catch {}
}

function extractFileTimesFromJsonReport(jsonPath) {
  const out = {}
  let data
  try {
    data = JSON.parse(fs.readFileSync(jsonPath, 'utf-8'))
  } catch {
    return out
  }

  function visit(suites) {
    if (!suites) return
    for (const suite of suites) {
      if (suite.specs) {
        for (const spec of suite.specs) {
          const file = spec.file ?? suite.file
          if (!file) continue
          const base = path.basename(file)
          const name = base.replace(/\.(spec\.)?ts$/, '')
          let total = 0
          for (const test of spec.tests ?? []) {
            for (const result of test.results ?? []) {
              if (typeof result.duration === 'number') total += result.duration
            }
          }
          if (total > 0) out[name] = (out[name] ?? 0) + total
        }
      }
      visit(suite.suites)
    }
  }
  visit(data.suites)
  return out
}

function runBatch(batch) {
  const port = 9200 + Number.parseInt(batch.id, 10)
  generateBatchConfig(batch.id, port)

  const hasLoginTests = batch.tests.includes('login')
  const projects = hasLoginTests
    ? ['--project=auth-setup', '--project=login', '--project=chromium']
    : ['--project=auth-setup', '--project=chromium']

  const testFiles = batch.tests.map((testName) => `tests/e2e/${testName}.spec.ts`)
  const jsonOutputPath = path.join(FIXTURES_DIR, `batch-${batch.id}-results.json`)
  const args = [
    PLAYWRIGHT_CLI,
    'test',
    ...testFiles,
    ...projects,
    '--reporter=line',
    '--reporter=json',
  ]

  const prefix = `[batch:${batch.id}]`
  const startMs = Date.now()

  return new Promise((resolve) => {
    const child = spawn(process.execPath, args, {
      cwd: ROOT,
      env: {
        ...process.env,
        BATCH_ID: batch.id,
        PLAYWRIGHT_JSON_OUTPUT_NAME: jsonOutputPath,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    })

    child.stdout.on('data', (data) => {
      for (const line of data.toString().split('\n')) {
        if (line.trim()) process.stdout.write(`${prefix} ${line}\n`)
      }
    })

    child.stderr.on('data', (data) => {
      for (const line of data.toString().split('\n')) {
        if (line.trim()) process.stderr.write(`${prefix} ${line}\n`)
      }
    })

    child.on('close', (code) => {
      const elapsedMs = Date.now() - startMs
      const fileTimes = extractFileTimesFromJsonReport(jsonOutputPath)
      try {
        fs.unlinkSync(jsonOutputPath)
      } catch {}
      resolve({ code: code ?? 1, elapsedMs, fileTimes })
    })
  })
}

async function main() {
  console.log(`Starting ${BATCHES.length} test batches in parallel...\n`)
  const startTime = Date.now()

  try {
    const results = await Promise.all(BATCHES.map(runBatch))

    const elapsed = ((Date.now() - startTime) / 1000).toFixed(1)
    console.log(`\n${'-'.repeat(60)}`)
    console.log(`Batch results (${elapsed}s total):`)

    let allPassed = true
    for (let index = 0; index < BATCHES.length; index += 1) {
      const { code, elapsedMs, fileTimes } = results[index]
      const status = code === 0 ? 'PASS' : 'FAIL'
      if (code !== 0) allPassed = false
      const names =
        fileTimes['auth-setup'] != null
          ? ['auth-setup', ...BATCHES[index].tests]
          : BATCHES[index].tests
      const testListWithTimes = names
        .map((testName) => {
          const sec = fileTimes[testName] != null ? (fileTimes[testName] / 1000).toFixed(1) : '?'
          return `${testName} ${sec}s`
        })
        .join(', ')
      const elapsedSec = (elapsedMs / 1000).toFixed(1)
      console.log(`  Batch ${BATCHES[index].id}: ${status}  ${elapsedSec}s  (${testListWithTimes})`)
    }

    console.log(`${'-'.repeat(60)}`)
    console.log(allPassed ? '\nAll batches passed!' : '\nSome batches failed!')
    process.exit(allPassed ? 0 : 1)
  } finally {
    for (const batch of BATCHES) cleanupBatchConfig(batch.id)
  }
}

void main()
