import fs from 'node:fs'
import path from 'node:path'
import { execSync } from 'node:child_process'

const MINIMAL_JPEG = Buffer.from(
  '/9j/2wBDAAYEBQYFBAYGBQYHBwYIChAKCgkJChQODwwQFxQYGBcUFhYaHSUfGhsjHBYWICwg' +
    'IyYnKSopGR8tMC0oMCUoKSj/2wBDAQcHBwoIChMKChMoGhYaKCgoKCgoKCgoKCgoKCgoKCgo' +
    'KCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCj/wAARCAABAAEDASIAAhEBAxEB/8QA' +
    'FQABAQAAAAAAAAAAAAAAAAAAAAf/xAAUEAEAAAAAAAAAAAAAAAAAAAAA/8QAFQEBAQAAAAAAAA' +
    'AAAAAAAAAABgj/xAAUEQEAAAAAAAAAAAAAAAAAAAAA/9oADAMBAAIRAxEAPwCdABykX//Z',
  'base64',
)

const MINIMAL_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/Pch' +
    'I7wAAAABJRU5ErkJggg==',
  'base64',
)

const MINIMAL_PDF = Buffer.from(
  '%PDF-1.0\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n' +
    '2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n' +
    '3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj\n' +
    'xref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n' +
    '0000000058 00000 n \n0000000115 00000 n \n' +
    'trailer<</Size 4/Root 1 0 R>>\nstartxref\n190\n%%EOF',
)

const AUTOSAVE_PARITY_TXT_CONTENT = 'Autosave parity initial content for e2e only.\n'

function hasFfmpeg() {
  try {
    execSync('ffmpeg -version', { stdio: 'ignore' })
    return true
  } catch {
    return false
  }
}

function run(command, cwd) {
  execSync(command, { stdio: 'ignore', cwd, timeout: 30_000 })
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true })
}

export function patchTestMediaAfterCacheCopy(baseDir) {
  const notesDir = path.join(baseDir, 'Notes')
  ensureDir(notesDir)
  fs.writeFileSync(path.join(notesDir, 'autosave-parity.txt'), AUTOSAVE_PARITY_TXT_CONTENT)

  const deepDir = path.join(notesDir, 'subfolder', 'breadcrumb-deep')
  ensureDir(deepDir)
  fs.writeFileSync(
    path.join(deepDir, 'deep-readme.txt'),
    'Fixture for breadcrumb depth / ellipsis e2e tests.\n',
  )

  const deepChain = ['seg-a', 'seg-b', 'seg-c', 'breadcrumb-deep']
  let chainPath = notesDir
  for (const part of deepChain) {
    chainPath = path.join(chainPath, part)
    ensureDir(chainPath)
  }
  fs.writeFileSync(
    path.join(chainPath, 'chain-readme.txt'),
    'Fixture for deep breadcrumb ellipsis e2e tests.\n',
  )
}

export function generateTestMedia(baseDir) {
  const ff = hasFfmpeg()
  if (!ff) {
    console.warn('WARNING: ffmpeg not found; video/audio files will not be generated.')
    console.warn('Install ffmpeg for full test coverage.')
  }

  const videosDir = path.join(baseDir, 'Videos')
  ensureDir(videosDir)
  if (ff) {
    run(
      'ffmpeg -y -f lavfi -i color=black:s=320x240:d=2 -f lavfi -i anullsrc=r=44100:cl=mono -shortest -c:v libx264 -pix_fmt yuv420p -c:a aac sample.mp4',
      videosDir,
    )
    try {
      run(
        'ffmpeg -y -f lavfi -i color=black:s=320x240:d=2 -f lavfi -i anullsrc=r=44100:cl=mono -shortest -c:v libvpx -c:a libvorbis sample.webm',
        videosDir,
      )
    } catch {
      console.warn('  Could not generate WebM (libvpx unavailable), skipping')
    }
  }

  const musicDir = path.join(baseDir, 'Music')
  ensureDir(musicDir)
  if (ff) {
    run('ffmpeg -y -f lavfi -i anullsrc=r=44100:cl=mono -t 2 -c:a libmp3lame track.mp3', musicDir)
    run('ffmpeg -y -f lavfi -i anullsrc=r=44100:cl=mono -t 2 track.flac', musicDir)
  }
  fs.writeFileSync(path.join(musicDir, 'cover.jpg'), MINIMAL_JPEG)

  const imagesDir = path.join(baseDir, 'Images')
  ensureDir(imagesDir)
  fs.writeFileSync(path.join(imagesDir, 'photo.jpg'), MINIMAL_JPEG)
  fs.writeFileSync(path.join(imagesDir, 'photo.png'), MINIMAL_PNG)

  const docsDir = path.join(baseDir, 'Documents')
  ensureDir(docsDir)
  fs.writeFileSync(
    path.join(docsDir, 'readme.txt'),
    'This is a test readme file.\nIt has multiple lines.\nLine three.\n',
  )
  fs.writeFileSync(
    path.join(docsDir, 'notes.md'),
    '# Test Notes\n\nThis is a **markdown** file with [a link](https://example.com).\n\n## Section Two\n\nMore content here.\n',
  )
  fs.writeFileSync(path.join(docsDir, 'image-note.md'), '# Image Note\n\n![photo](Images/photo.jpg)\n')
  fs.writeFileSync(path.join(docsDir, 'data.json'), JSON.stringify({ name: 'test', items: [1, 2, 3] }, null, 2))
  fs.writeFileSync(path.join(docsDir, 'sample.pdf'), MINIMAL_PDF)
  fs.writeFileSync(path.join(docsDir, 'unsupported.xyz'), Buffer.from('test'))

  const notesDir = path.join(baseDir, 'Notes')
  ensureDir(path.join(notesDir, 'images'))
  ensureDir(path.join(notesDir, 'subfolder'))
  fs.writeFileSync(
    path.join(notesDir, 'welcome.md'),
    '# Welcome\n\nThis is the welcome note.\n\n![[diagram.png]]\n',
  )
  fs.writeFileSync(
    path.join(notesDir, 'todo.md'),
    '# Todo List\n\n- [ ] First task\n- [ ] Second task\n- [x] Done task\n',
  )
  fs.writeFileSync(path.join(notesDir, 'autosave-parity.txt'), AUTOSAVE_PARITY_TXT_CONTENT)
  fs.writeFileSync(
    path.join(notesDir, 'subfolder', 'nested-note.md'),
    '# Nested Note\n\nThis is a nested note inside a subfolder.\n',
  )
  ensureDir(path.join(notesDir, 'subfolder', 'breadcrumb-deep'))
  fs.writeFileSync(
    path.join(notesDir, 'subfolder', 'breadcrumb-deep', 'deep-readme.txt'),
    'Fixture for breadcrumb depth / ellipsis e2e tests.\n',
  )
  const deepChain = ['seg-a', 'seg-b', 'seg-c', 'breadcrumb-deep']
  let chainPath = notesDir
  for (const part of deepChain) {
    chainPath = path.join(chainPath, part)
    ensureDir(chainPath)
  }
  fs.writeFileSync(
    path.join(chainPath, 'chain-readme.txt'),
    'Fixture for deep breadcrumb ellipsis e2e tests.\n',
  )
  fs.writeFileSync(path.join(notesDir, 'images', 'diagram.png'), MINIMAL_PNG)

  const sharedDir = path.join(baseDir, 'SharedContent')
  ensureDir(path.join(sharedDir, 'subfolder'))
  fs.writeFileSync(path.join(sharedDir, 'public-doc.txt'), 'This is a public document for share testing.\n')
  fs.writeFileSync(path.join(sharedDir, 'subfolder', 'nested.txt'), 'Nested file in shared content.\n')
  fs.writeFileSync(path.join(sharedDir, 'photo.jpg'), MINIMAL_JPEG)
  fs.writeFileSync(path.join(sharedDir, 'photo.png'), MINIMAL_PNG)
  fs.writeFileSync(path.join(sharedDir, 'sample.pdf'), MINIMAL_PDF)
  fs.writeFileSync(path.join(sharedDir, 'cover.jpg'), MINIMAL_JPEG)
  if (ff) {
    run(
      'ffmpeg -y -f lavfi -i color=black:s=320x240:d=2 -f lavfi -i anullsrc=r=44100:cl=mono -shortest -c:v libx264 -pix_fmt yuv420p -c:a aac public-video.mp4',
      sharedDir,
    )
    run('ffmpeg -y -f lavfi -i anullsrc=r=44100:cl=mono -t 2 -c:a libmp3lame track.mp3', sharedDir)
  }

  ensureDir(path.join(baseDir, 'EmptyFolder'))
}
