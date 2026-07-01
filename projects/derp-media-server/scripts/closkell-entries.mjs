import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'

const generatedRoot = '.closkell/build'

const entries = [
  {
    source: 'lib/types.clsk',
    out: 'lib/closkell/types.mjs',
  },
  {
    source: 'lib/constants.clsk',
    out: 'lib/closkell/constants.mjs',
  },
  {
    source: 'lib/client-store-utils.clsk',
    out: 'lib/closkell/client-store-utils.mjs',
  },
  {
    source: 'lib/api.clsk',
    out: 'lib/closkell/api.mjs',
  },
  {
    source: 'lib/prefetch-folder-hover.clsk',
    out: 'lib/closkell/prefetch-folder-hover.mjs',
  },
  {
    source: 'lib/collect-dropped-upload-files.clsk',
    out: 'lib/closkell/collect-dropped-upload-files.mjs',
  },
  {
    source: 'lib/extract-paste-data.clsk',
    out: 'lib/closkell/extract-paste-data.mjs',
  },
  {
    source: 'lib/class-names.clsk',
    out: 'lib/closkell/class-names.mjs',
  },
  {
    source: 'lib/floating-layer-mount.clsk',
    out: 'lib/closkell/floating-layer-mount.mjs',
  },
  {
    source: 'lib/floating-layer-registry.clsk',
    out: 'lib/closkell/floating-layer-registry.mjs',
  },
  {
    source: 'lib/theme-dom.clsk',
    out: 'lib/closkell/theme-dom.mjs',
  },
  {
    source: 'lib/dynamic-favicon-core.clsk',
    out: 'lib/closkell/dynamic-favicon-core.mjs',
  },
  {
    source: 'lib/long-press-context-menu.clsk',
    out: 'lib/closkell/long-press-context-menu.mjs',
  },
  {
    source: 'lib/thumbnail-load-queue.clsk',
    out: 'lib/closkell/thumbnail-load-queue.mjs',
  },
  {
    source: 'lib/share-text-viewer-settings.clsk',
    out: 'lib/closkell/share-text-viewer-settings.mjs',
  },
  {
    source: 'lib/floating-z-index.clsk',
    out: 'lib/closkell/floating-z-index.mjs',
  },
  {
    source: 'lib/mutex.clsk',
    out: 'lib/closkell/mutex.mjs',
  },
  {
    source: 'lib/enable-fine-pointer-drag.clsk',
    out: 'lib/closkell/enable-fine-pointer-drag.mjs',
  },
  {
    source: 'lib/should-offer-paste-as-new-file.clsk',
    out: 'lib/closkell/should-offer-paste-as-new-file.mjs',
  },
  {
    source: 'lib/query-keys.clsk',
    out: 'lib/closkell/query-keys.mjs',
  },
  {
    source: 'src/features/source-context.clsk',
    out: 'lib/closkell/source-context.mjs',
  },
  {
    source: 'src/features/sse-reconnect.clsk',
    out: 'lib/closkell/sse-reconnect.mjs',
  },
  {
    source: 'src/features/sse-shared-worker.clsk',
    out: 'lib/closkell/sse-shared-worker.mjs',
  },
  {
    source: 'src/features/sse-shared-worker-client.clsk',
    out: 'lib/closkell/sse-shared-worker-client.mjs',
  },
  {
    source: 'src/features/client-paths.clsk',
    out: 'lib/closkell/client-paths.mjs',
  },
  {
    source: 'src/features/clamp-fixed-menu.clsk',
    out: 'lib/closkell/clamp-fixed-menu.mjs',
  },
  {
    source: 'src/features/download.clsk',
    out: 'lib/closkell/download.mjs',
  },
  {
    source: 'src/features/file-drag-data.clsk',
    out: 'lib/closkell/file-drag-data.mjs',
  },
  {
    source: 'src/features/knowledge-base.clsk',
    out: 'lib/closkell/knowledge-base.mjs',
  },
  {
    source: 'src/features/kb-chat-fs-paths.clsk',
    out: 'lib/closkell/kb-chat-fs-paths.mjs',
  },
  {
    source: 'src/features/media-utils.clsk',
    out: 'lib/closkell/media-utils.mjs',
  },
  {
    source: 'src/features/media-roots.clsk',
    out: 'lib/closkell/media-roots.mjs',
  },
  {
    source: 'src/features/media-urls.clsk',
    out: 'lib/closkell/media-urls.mjs',
  },
  {
    source: 'src/features/mcp-ai-tool-key.clsk',
    out: 'lib/closkell/mcp-ai-tool-key.mjs',
  },
  {
    source: 'src/features/mcp-config.clsk',
    out: 'lib/closkell/mcp-config.mjs',
  },
  {
    source: 'src/features/pasted-kb-image.clsk',
    out: 'lib/closkell/pasted-kb-image.mjs',
  },
  {
    source: 'src/features/paste-data.clsk',
    out: 'lib/closkell/paste-data.mjs',
  },
  {
    source: 'src/features/resolve-markdown-image-url.clsk',
    out: 'lib/closkell/resolve-markdown-image-url.mjs',
  },
  {
    source: 'src/features/text-viewer-markdown.clsk',
    out: 'lib/closkell/text-viewer-markdown.mjs',
  },
  {
    source: 'src/features/share-path.clsk',
    out: 'lib/closkell/share-path.mjs',
  },
  {
    source: 'src/features/share-restrictions.clsk',
    out: 'lib/closkell/share-restrictions.mjs',
  },
  {
    source: 'src/features/virtual-folders.clsk',
    out: 'lib/closkell/virtual-folders.mjs',
  },
  {
    source: 'src/features/video-player-position.clsk',
    out: 'lib/closkell/video-player-position.mjs',
  },
  {
    source: 'src/features/theme.clsk',
    out: 'lib/closkell/theme.mjs',
  },
  {
    source: 'src/features/upload-toast.clsk',
    out: 'lib/closkell/upload-toast.mjs',
  },
  {
    source: 'src/features/modal-overlay-scope.clsk',
    out: 'lib/closkell/modal-overlay-scope.mjs',
  },
  {
    source: 'src/features/virtual-directory-scroll.clsk',
    out: 'lib/closkell/virtual-directory-scroll.mjs',
  },
  {
    source: 'src/features/workspace/tab-drop-hit.clsk',
    out: 'lib/closkell/workspace/tab-drop-hit.mjs',
  },
  {
    source: 'src/features/workspace/assist-grid.clsk',
    out: 'lib/closkell/workspace/assist-grid.mjs',
  },
  {
    source: 'src/features/workspace/browser-pane-paths.clsk',
    out: 'lib/closkell/workspace/browser-pane-paths.mjs',
  },
  {
    source: 'src/features/workspace/file-open-target-picker.clsk',
    out: 'lib/closkell/workspace/file-open-target-picker.mjs',
  },
  {
    source: 'src/features/workspace/geometry.clsk',
    out: 'lib/closkell/workspace/geometry.mjs',
  },
  {
    source: 'src/features/workspace/snap-resize-handles.clsk',
    out: 'lib/closkell/workspace/snap-resize-handles.mjs',
  },
  {
    source: 'src/features/workspace/snap-live.clsk',
    out: 'lib/closkell/workspace/snap-live.mjs',
  },
  {
    source: 'src/features/workspace/snap-pick.clsk',
    out: 'lib/closkell/workspace/snap-pick.mjs',
  },
  {
    source: 'src/features/workspace/snap-preview.clsk',
    out: 'lib/closkell/workspace/snap-preview.mjs',
  },
  {
    source: 'src/features/workspace/merge-target.clsk',
    out: 'lib/closkell/workspace/merge-target.mjs',
  },
  {
    source: 'src/features/workspace/video-intrinsics.clsk',
    out: 'lib/closkell/workspace/video-intrinsics.mjs',
  },
  {
    source: 'src/features/workspace/tab-icon-colors.clsk',
    out: 'lib/closkell/workspace/tab-icon-colors.mjs',
  },
  {
    source: 'src/features/workspace/taskbar-pins.clsk',
    out: 'lib/closkell/workspace/taskbar-pins.mjs',
  },
  {
    source: 'src/features/workspace/tab-groups-core.clsk',
    out: 'lib/closkell/workspace/tab-groups-core.mjs',
  },
  {
    source: 'src/features/workspace/tab-group-ops.clsk',
    out: 'lib/closkell/workspace/tab-group-ops.mjs',
  },
  {
    source: 'src/features/workspace/titles.clsk',
    out: 'lib/closkell/workspace/titles.mjs',
  },
  {
    source: 'src/features/workspace/page-persistence.clsk',
    out: 'lib/closkell/workspace/page-persistence.mjs',
  },
  {
    source: 'src/features/workspace/bootstrap.clsk',
    out: 'lib/closkell/workspace/bootstrap.mjs',
  },
  {
    source: 'src/features/workspace/session-resize.clsk',
    out: 'lib/closkell/workspace/session-resize.mjs',
  },
  {
    source: 'src/features/workspace/layout-presets.clsk',
    out: 'lib/closkell/workspace/layout-presets.mjs',
  },
  {
    source: 'src/features/workspace/layout-preview.clsk',
    out: 'lib/closkell/workspace/layout-preview.mjs',
  },
  {
    source: 'src/features/workspace/state-core.clsk',
    out: 'lib/closkell/workspace/state-core.mjs',
  },
  {
    source: 'server/routes/api/stats.clsk',
    out: 'server/routes/api/stats.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/api/settings.clsk',
    out: 'server/routes/api/settings.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/api/auth.clsk',
    out: 'server/routes/api/auth.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/api/files.clsk',
    out: 'server/routes/api/files.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/api/shares.clsk',
    out: 'server/routes/api/shares.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/api/kb.clsk',
    out: 'server/routes/api/kb.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/api/kb-chat.clsk',
    out: 'server/routes/api/kb-chat.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/api/shareAccess.clsk',
    out: 'server/routes/api/shareAccess.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/download.clsk',
    out: 'server/routes/download.mjs',
    target: 'server',
  },
  {
    source: 'server/lib/audio-helpers.clsk',
    out: 'server/lib/audio-helpers.mjs',
    target: 'server',
  },
  {
    source: 'server/lib/thumbnails.clsk',
    out: 'server/lib/thumbnails.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/media.clsk',
    out: 'server/routes/media.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/thumbnail.clsk',
    out: 'server/routes/thumbnail.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/upload.clsk',
    out: 'server/routes/upload.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/sse.clsk',
    out: 'server/routes/sse.mjs',
    target: 'server',
  },
  {
    source: 'server/routes/shareMedia.clsk',
    out: 'server/routes/shareMedia.mjs',
    target: 'server',
  },
  {
    source: 'server/html.clsk',
    out: 'server/html.mjs',
    target: 'server',
  },
  {
    source: 'server/app-helpers.clsk',
    out: 'server/app-helpers.mjs',
    target: 'server',
  },
  {
    source: 'server/kb-context.clsk',
    out: 'server/kb-context.mjs',
    target: 'server',
  },
  {
    source: 'server/kb-chat-fs-tools.clsk',
    out: 'server/kb-chat-fs-tools.mjs',
    target: 'server',
  },
  {
    source: 'server/mcp-kb-chat-tools.clsk',
    out: 'server/mcp-kb-chat-tools.mjs',
    target: 'server',
  },
  {
    source: 'server/auth-middleware.clsk',
    out: 'server/auth-middleware.mjs',
    target: 'server',
  },
  {
    source: 'server/main.clsk',
    out: 'server/main.mjs',
    target: 'server',
    app: true,
  },
]

const mode = process.argv[2]
if (mode !== 'check' && mode !== 'build') {
  console.error('Usage: node scripts/closkell-entries.mjs <check|build>')
  process.exit(2)
}

const runner = '../../packages/closkell-runner.cjs'

function generatedOutPath(out) {
  return `${generatedRoot}/${out}`
}

for (const entry of entries) {
  if (!existsSync(entry.source)) {
    continue
  }

  const target = entry.target ?? 'core'
  const args =
    mode === 'check'
      ? [runner, 'check', entry.source, '--target', target]
      : [runner, 'build', entry.source, '--target', target, '--out', generatedOutPath(entry.out)]

  if (mode === 'build' && entry.app) {
    args.push('--app')
  }

  const result = spawnSync(process.execPath, args, { stdio: 'inherit' })
  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }
}
