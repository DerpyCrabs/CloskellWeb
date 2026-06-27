import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RUNTIME_PACKAGE = "@closkell/runtime";
const RUNTIME_OPTIMIZED_PREFIX = "@closkell_runtime";
const PLUGIN_PATH = fileURLToPath(import.meta.url);

function stripQuery(id) {
  const queryIndex = id.indexOf("?");
  return queryIndex === -1 ? id : id.slice(0, queryIndex);
}

function toPosixPath(value) {
  return value.split(path.sep).join("/");
}

function urlPathname(url) {
  try {
    return new URL(url, "http://closkell.local").pathname;
  } catch {
    return url.split("?")[0] ?? url;
  }
}

function requestUrl(url) {
  return new URL(url ?? "", "http://closkell.local");
}

function samePath(first, second) {
  return path.resolve(first).toLowerCase() === path.resolve(second).toLowerCase();
}

function withRuntimeOptimizeDepsExclude(exclude = []) {
  return [...new Set([...exclude, RUNTIME_PACKAGE])];
}

function withRuntimeSsrNoExternal(noExternal) {
  if (noExternal === true) return true;
  const entries = Array.isArray(noExternal) ? noExternal : noExternal ? [noExternal] : [];
  return [...new Set([...entries, RUNTIME_PACKAGE])];
}

function withoutModulePreloadPolyfill(modulePreload) {
  if (modulePreload === false) return false;
  const config = modulePreload && typeof modulePreload === "object" ? modulePreload : {};
  return { ...config, polyfill: false };
}

function isRuntimeOptimizedFile(file) {
  return file.startsWith(RUNTIME_OPTIMIZED_PREFIX);
}

function isRuntimeOptimizedRequest(pathname) {
  return (
    pathname.startsWith("/node_modules/.vite/deps/") &&
    path.posix.basename(pathname).startsWith(RUNTIME_OPTIMIZED_PREFIX) &&
    pathname.endsWith(".js")
  );
}

function exportedBindingNames(code) {
  const names = new Set();
  for (const match of code.matchAll(/\bexport\s+(?:const|let|var|function)\s+([A-Za-z_$][\w$]*)/g)) {
    names.add(match[1]);
  }
  return [...names].sort();
}

function appendVitestRegistration(code) {
  const exports = exportedBindingNames(code);
  if (exports.length === 0) return code;
  const moduleObject = exports
    .map((name) => `${name}: typeof ${name} !== "undefined" ? ${name} : undefined`)
    .join(", ");
  const suffix = [
    "",
    "import { registerVitestTests as __closkellRegisterVitestTests } from \"@closkell/runtime\";",
    "import { describe as __closkellVitestDescribe, test as __closkellVitestTest } from \"vitest\";",
    `__closkellRegisterVitestTests({ ${moduleObject} }, { describe: __closkellVitestDescribe, test: __closkellVitestTest });`,
    ""
  ].join("\n");
  return code.endsWith("\n") ? `${code}${suffix}` : `${code}\n${suffix}`;
}

async function pruneOptimizedRuntimeMetadata(metadataPath) {
  let raw;
  try {
    raw = await fs.readFile(metadataPath, "utf8");
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }

  let metadata;
  try {
    metadata = JSON.parse(raw);
  } catch {
    return false;
  }

  let changed = false;
  for (const key of ["optimized", "discovered"]) {
    if (metadata[key]?.[RUNTIME_PACKAGE]) {
      delete metadata[key][RUNTIME_PACKAGE];
      changed = true;
    }
  }

  if (changed) {
    await fs.writeFile(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`);
  }
  return changed;
}

async function pruneOptimizedRuntimeCache(config) {
  const depsDir = path.join(config.cacheDir, "deps");
  let entries;
  try {
    entries = await fs.readdir(depsDir, { withFileTypes: true });
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }

  let pruned = false;
  for (const entry of entries) {
    if (entry.isFile() && isRuntimeOptimizedFile(entry.name)) {
      await fs.rm(path.join(depsDir, entry.name), { force: true });
      pruned = true;
    }
  }

  const metadataPruned = await pruneOptimizedRuntimeMetadata(path.join(depsDir, "_metadata.json"));
  return pruned || metadataPruned;
}

async function statMtimeMs(file) {
  try {
    return (await fs.stat(file)).mtimeMs;
  } catch (error) {
    if (error?.code === "ENOENT") return 0;
    throw error;
  }
}

async function collectClskFiles(root) {
  const entries = await fs.readdir(root, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const file = path.join(root, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "generated" || entry.name === "node_modules") continue;
      files.push(...(await collectClskFiles(file)));
    } else if (entry.isFile() && entry.name.endsWith(".clsk")) {
      files.push(file);
    }
  }
  return files;
}

async function newestClskMtimeMs(root) {
  const files = await collectClskFiles(root);
  const mtimes = await Promise.all(files.map(statMtimeMs));
  return Math.max(0, ...mtimes);
}

async function newestRustBuildInputMtimeMs(root) {
  const files = [
    path.join(root, "Cargo.lock"),
    path.join(root, "Cargo.toml")
  ];
  const crates = path.join(root, "crates");
  let entries = [];
  try {
    entries = await fs.readdir(crates, { withFileTypes: true });
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const crateRoot = path.join(crates, entry.name);
    files.push(path.join(crateRoot, "Cargo.toml"));
    files.push(path.join(crateRoot, "build.rs"));
    files.push(
      ...(await collectFiles(path.join(crateRoot, "src"), (file) => file.endsWith(".rs")))
    );
  }
  const mtimes = await Promise.all(files.map(statMtimeMs));
  return Math.max(0, ...mtimes);
}

async function collectFiles(root, include) {
  let entries;
  try {
    entries = await fs.readdir(root, { withFileTypes: true });
  } catch (error) {
    if (error?.code === "ENOENT") return [];
    throw error;
  }

  const files = [];
  for (const entry of entries) {
    const file = path.join(root, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "target" || entry.name === "node_modules") continue;
      files.push(...(await collectFiles(file, include)));
    } else if (entry.isFile() && include(file)) {
      files.push(file);
    }
  }
  return files;
}

function runCommand(command, args, cwd, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      env: { ...process.env, ...env },
      stdio: ["ignore", "pipe", "pipe"]
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve({ stdout, stderr });
        return;
      }
      reject(
        new Error(
          [`${command} ${args.join(" ")} exited with code ${code}`, stdout, stderr]
            .filter(Boolean)
            .join("\n")
        )
      );
    });
  });
}

export function closkell(options = {}) {
  const entryConfigured = Object.prototype.hasOwnProperty.call(options, "entry");
  const {
    entry = "src/app.clsk",
    out = null,
    outDir = null,
    sourceRoot: sourceRootOption = null,
    manifestPath = "../Cargo.toml",
    packageName = "cli",
    binary = null,
    rootId = "root",
    css = "src/styles.css",
    app = true,
    vitest = false,
    sourceMap = false,
    vendorRuntime = true,
    inspect = true,
    inspectPath = "/__closkell/inspect"
  } = options;

  let config;
  let sourceRoot;
  let manifestRoot;
  let compilerCommand = null;
  let compilerCommandPromise = null;
  let entryPath = null;
  let outPath;
  let generatedRoot;
  const inFlight = new Map();
  const runtimeEffectsBySource = new Map();

  function resolveFromRoot(value) {
    return path.resolve(config.root, value);
  }

  function resolveClskId(source, importer) {
    const bare = stripQuery(source);
    if (bare.startsWith("/")) {
      return path.join(config.root, bare.slice(1));
    }
    if (path.isAbsolute(bare)) {
      return bare;
    }
    const importerPath = importer ? stripQuery(importer) : config.root;
    const base = importer ? path.dirname(importerPath) : config.root;
    return path.resolve(base, bare);
  }

  function outputForSource(source) {
    const resolved = path.resolve(source);
    if (entryPath && samePath(resolved, entryPath)) {
      return outPath;
    }

    let relative = path.relative(sourceRoot, resolved);
    if (relative.startsWith("..") || path.isAbsolute(relative)) {
      relative = path.basename(resolved);
    }
    return path.join(generatedRoot, relative).replace(/\.clsk$/i, ".mjs");
  }

  function cssImportForOutput(cssPath, output) {
    if (!cssPath) return null;
    if (
      cssPath.startsWith(".") ||
      cssPath.startsWith("/") ||
      path.isAbsolute(cssPath) ||
      /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(cssPath)
    ) {
      return toPosixPath(cssPath);
    }

    let relative = toPosixPath(path.relative(path.dirname(output), resolveFromRoot(cssPath)));
    if (!relative.startsWith(".")) {
      relative = `./${relative}`;
    }
    return relative;
  }

  function outputMetadataPath(output) {
    return `${output}.closkell-meta.json`;
  }

  async function readOutputMetadata(output) {
    try {
      return JSON.parse(await fs.readFile(outputMetadataPath(output), "utf8"));
    } catch (error) {
      if (error?.code === "ENOENT") return null;
      return null;
    }
  }

  async function writeOutputMetadata(output, fingerprint) {
    await fs.writeFile(outputMetadataPath(output), `${JSON.stringify(fingerprint, null, 2)}\n`);
  }

  async function outputFingerprint(source, output) {
    const compiler = await resolveCompilerCommand();
    return {
      source: path.resolve(source),
      output: path.resolve(output),
      app: Boolean(app && entryPath && samePath(path.resolve(source), entryPath)),
      rootId,
      css: cssImportForOutput(css, output) || "",
      sourceMap: Boolean(sourceMap),
      vendorRuntime: Boolean(vendorRuntime),
      vitest: Boolean(vitest),
      compilerCommand: path.resolve(compiler.command),
      compilerArgs: compiler.args,
      compilerMtime: await statMtimeMs(compiler.command),
      rustInputMtime: await currentNewestRustBuildInputMtimeMs(),
      runtimeMtime: await statMtimeMs(workspaceRuntimeSourcePath()),
      pluginMtime: await statMtimeMs(PLUGIN_PATH)
    };
  }

  function sameFingerprint(first, second) {
    return JSON.stringify(first) === JSON.stringify(second);
  }

  async function outputIsFresh(source, output, fingerprint) {
    if (!existsSync(output)) return false;
    const outputMtime = await statMtimeMs(output);
    const sourceMtime = entryNeedsVendoredRuntime(source)
      ? await currentNewestClskMtimeMs()
      : await statMtimeMs(source);
    if (outputMtime < sourceMtime) return false;
    if (outputMtime < (await currentNewestRustBuildInputMtimeMs())) return false;
    if (sourceMap && !existsSync(sourceMapPath(output))) return false;
    if (!sourceMap) {
      const tail = await readFileTail(output, 256);
      if (tail.includes("sourceMappingURL=")) return false;
    }
    if (!sameFingerprint(await readOutputMetadata(output), fingerprint)) return false;
    return true;
  }

  function sourceMapPath(output) {
    return `${output}.map`;
  }

  function vendoredRuntimeSourcePath() {
    return path.join(config.root, "node_modules", "@closkell", "runtime", "src", "index.js");
  }

  function workspaceRuntimeSourcePath() {
    return path.join(manifestRoot, "runtime-js", "src", "index.js");
  }

  async function vendoredRuntimeIsFresh() {
    const vendored = vendoredRuntimeSourcePath();
    if (!existsSync(vendored)) return false;
    const workspaceRuntime = workspaceRuntimeSourcePath();
    if (!existsSync(workspaceRuntime)) return true;
    return (await statMtimeMs(vendored)) >= (await statMtimeMs(workspaceRuntime));
  }

  function entryNeedsVendoredRuntime(source) {
    return vendorRuntime && app && entryPath && samePath(source, entryPath);
  }

  function pathKey(value) {
    return path.resolve(value).toLowerCase();
  }

  function parseBuildReport(stdout) {
    const text = stdout.trim();
    if (!text) return null;
    const line = [...text.split(/\r?\n/)].reverse().find((line) => line.trim().startsWith("{"));
    return line ? JSON.parse(line) : null;
  }

  function runtimeEffectsSignature(effects = []) {
    return [...effects].sort().join("\0");
  }

  function rememberBuildReport(report, { markNewRuntimeEffectsAsChanged = false } = {}) {
    let changed = false;
    for (const artifact of report?.artifacts ?? []) {
      if (!artifact?.source) continue;
      const key = pathKey(artifact.source);
      const signature = runtimeEffectsSignature(artifact.runtimeEffects);
      const previous = runtimeEffectsBySource.get(key);
      if (
        (previous === undefined && markNewRuntimeEffectsAsChanged) ||
        (previous !== undefined && previous !== signature)
      ) {
        changed = true;
      }
      runtimeEffectsBySource.set(key, signature);
    }
    return changed;
  }

  function reportOutputs(report, fallbackOutput) {
    const outputs = new Set();
    if (fallbackOutput) outputs.add(path.resolve(fallbackOutput));
    for (const artifact of report?.artifacts ?? []) {
      if (artifact?.output) outputs.add(path.resolve(artifact.output));
    }
    return outputs;
  }

  function invalidateOutputs(server, outputs) {
    for (const output of outputs) {
      const modules = server.moduleGraph.getModulesByFile(output);
      if (!modules) continue;
      for (const mod of modules) server.moduleGraph.invalidateModule(mod);
    }
  }

  async function readFileTail(file, bytes) {
    let handle;
    try {
      handle = await fs.open(file, "r");
      const stat = await handle.stat();
      const length = Math.min(bytes, stat.size);
      const buffer = Buffer.alloc(length);
      await handle.read(buffer, 0, length, stat.size - length);
      return buffer.toString("utf8");
    } catch (error) {
      if (error?.code === "ENOENT") return "";
      throw error;
    } finally {
      await handle?.close();
    }
  }

  function currentNewestClskMtimeMs() {
    return newestClskMtimeMs(sourceRoot);
  }

  function currentNewestRustBuildInputMtimeMs() {
    return newestRustBuildInputMtimeMs(manifestRoot);
  }

  async function resolveCompilerCommand() {
    if (compilerCommand) return compilerCommand;
    if (compilerCommandPromise) return compilerCommandPromise;
    compilerCommandPromise = resolveCompilerCommandUncached().finally(() => {
      compilerCommandPromise = null;
    });
    compilerCommand = await compilerCommandPromise;
    return compilerCommand;
  }

  async function resolveCompilerCommandUncached() {
    if (binary) {
      return { command: binary, args: [] };
    }

    const envBinary = process.env.CLOSKELL_BIN;
    if (envBinary) {
      return { command: envBinary, args: [] };
    }

    const exe = process.platform === "win32" ? ".exe" : "";
    const workspaceRoot = manifestRoot;
    const rustMtime = await currentNewestRustBuildInputMtimeMs();
    for (const profile of ["release", "debug"]) {
      const candidate = path.join(workspaceRoot, "target", profile, `closkell${exe}`);
      if (existsSync(candidate) && (await statMtimeMs(candidate)) >= rustMtime) {
        return { command: candidate, args: [] };
      }
    }

    await runCommand(
      "cargo",
      ["build", "-q", "--manifest-path", resolveFromRoot(manifestPath), "-p", packageName],
      config.root
    );
    const candidate = path.join(workspaceRoot, "target", "debug", `closkell${exe}`);
    if (!existsSync(candidate)) {
      throw new Error(`Closkell compiler binary was not produced at ${candidate}`);
    }
    return { command: candidate, args: [] };
  }

  async function compile(
    source,
    { force = false, json = false, markNewRuntimeEffectsAsChanged = false } = {}
  ) {
    const resolved = path.resolve(source);
    const output = outputForSource(resolved);
    const key = `${resolved}\0${output}\0${json ? "json" : "plain"}`;
    if (inFlight.has(key)) return inFlight.get(key);

    const task = (async () => {
      const fingerprint = await outputFingerprint(resolved, output);
      if (
        !force &&
        (await outputIsFresh(resolved, output, fingerprint)) &&
        (!entryNeedsVendoredRuntime(resolved) || (await vendoredRuntimeIsFresh()))
      ) {
        return json ? { output, report: null, runtimeEffectsChanged: false } : output;
      }

      const compiler = await resolveCompilerCommand();
      const args = [
        ...compiler.args,
        "build",
        resolved,
        "--out",
        output
      ];

      if (sourceMap) args.push("--sourcemap");
      if (app && entryPath && samePath(resolved, entryPath)) {
        args.push("--app", "--root", rootId);
        const cssImport = cssImportForOutput(css, output);
        if (cssImport) args.push("--css", cssImport);
        if (vendorRuntime) args.push("--vendor-runtime");
      }
      if (json) args.push("--json");

      config.logger.info(`closkell: ${toPosixPath(path.relative(config.root, resolved))}`);
      const { stdout } = await runCommand(compiler.command, args, config.root);
      await writeOutputMetadata(output, fingerprint);
      if (!json) return output;

      const report = parseBuildReport(stdout);
      const runtimeEffectsChanged = rememberBuildReport(report, {
        markNewRuntimeEffectsAsChanged
      });
      return { output, report, runtimeEffectsChanged };
    })().finally(() => {
      inFlight.delete(key);
    });

    inFlight.set(key, task);
    return task;
  }

  async function inspectSource(source) {
    const resolved = path.resolve(source);
    const compiler = await resolveCompilerCommand();
    const args = [...compiler.args, "inspect", resolved];
    const { stdout } = await runCommand(compiler.command, args, config.root);
    return stdout;
  }

  async function runtimeSourcePath({ allowCompile = false } = {}) {
    const vendored = vendoredRuntimeSourcePath();
    if (existsSync(vendored) && (await vendoredRuntimeIsFresh())) return vendored;

    if (allowCompile && vendorRuntime && entryPath && existsSync(entryPath)) {
      await compile(entryPath);
      if (existsSync(vendored)) return vendored;
    }

    const workspaceRuntime = workspaceRuntimeSourcePath();
    if (existsSync(workspaceRuntime)) return workspaceRuntime;

    throw new Error("Closkell runtime source was not found");
  }

  function shouldCompileEntryOnStart() {
    if (!entryPath) return false;
    return entryConfigured || existsSync(entryPath);
  }

  async function compileAndReload(server, changedFile) {
    try {
      const hasEntry = shouldCompileEntryOnStart();
      const source = hasEntry && samePath(changedFile, entryPath) ? entryPath : changedFile;
      const result = await compile(source, {
        force: true,
        json: true,
        markNewRuntimeEffectsAsChanged: true
      });
      const outputs = reportOutputs(result.report, result.output);

      if (hasEntry && !samePath(source, entryPath) && result.runtimeEffectsChanged) {
        const entryResult = await compile(entryPath, { force: true, json: true });
        for (const output of reportOutputs(entryResult.report, entryResult.output)) {
          outputs.add(output);
        }
      }

      invalidateOutputs(server, outputs);
      server.ws.send({ type: "full-reload", path: "*" });
    } catch (error) {
      server.config.logger.error(error.message);
      server.ws.send({
        type: "error",
        err: {
          message: error.message,
          stack: error.stack ?? "",
          id: changedFile,
          plugin: "vite-plugin-closkell"
        }
      });
    }
  }

  function isGeneratedClskOutput(id) {
    if (!vitest || !generatedRoot) return false;
    const resolved = path.resolve(stripQuery(id));
    if (!resolved.endsWith(".mjs")) return false;
    const relative = path.relative(generatedRoot, resolved);
    return Boolean(relative) && !relative.startsWith("..") && !path.isAbsolute(relative);
  }

  async function loadGeneratedClskOutput(id) {
    if (!isGeneratedClskOutput(id)) return null;
    const file = stripQuery(id);
    const code = await fs.readFile(file, "utf8");
    return appendVitestRegistration(code);
  }

  return {
    name: "vite-plugin-closkell",
    enforce: "pre",

    config(userConfig = {}) {
      return {
        optimizeDeps: {
          exclude: withRuntimeOptimizeDepsExclude(userConfig.optimizeDeps?.exclude)
        },
        ssr: {
          noExternal: withRuntimeSsrNoExternal(userConfig.ssr?.noExternal)
        },
        build: {
          modulePreload: withoutModulePreloadPolyfill(userConfig.build?.modulePreload)
        }
      };
    },

    async configResolved(resolvedConfig) {
      config = resolvedConfig;
      manifestRoot = path.dirname(resolveFromRoot(manifestPath));
      entryPath = entry ? resolveFromRoot(entry) : null;
      generatedRoot = outDir ? resolveFromRoot(outDir) : path.join(config.root, ".closkell", "vite");
      outPath = out ? resolveFromRoot(out) : path.join(generatedRoot, "main.mjs");
      sourceRoot = sourceRootOption
        ? resolveFromRoot(sourceRootOption)
        : entryPath
          ? path.dirname(entryPath)
          : resolveFromRoot("src");

      try {
        const pruned = await pruneOptimizedRuntimeCache(config);
        if (pruned) {
          config.logger.info("closkell: pruned stale optimized @closkell/runtime cache");
        }
      } catch (error) {
        config.logger.warn(`closkell: failed to prune optimized runtime cache: ${error.message}`);
      }
    },

    async buildStart() {
      if (shouldCompileEntryOnStart()) {
        await compile(entryPath, { force: true, json: true });
      }
    },

    async resolveId(source, importer) {
      if (stripQuery(source) === RUNTIME_PACKAGE) {
        return runtimeSourcePath().catch(() => null);
      }
      if (!stripQuery(source).endsWith(".clsk")) return null;
      const sourcePath = resolveClskId(source, importer);
      const output = await compile(sourcePath);
      return output;
    },

    async load(id) {
      return loadGeneratedClskOutput(id);
    },

    configureServer(server) {
      server.watcher.add(toPosixPath(path.join(sourceRoot, "**/*.clsk")));
      server.middlewares.use(async (req, res, next) => {
        const request = requestUrl(req.url);
        const pathname = request.pathname;
        if (isRuntimeOptimizedRequest(pathname)) {
          try {
            const runtimeSource = await runtimeSourcePath({ allowCompile: true });
            const code = await fs.readFile(runtimeSource, "utf8");
            res.statusCode = 200;
            res.setHeader("Cache-Control", "no-store");
            res.setHeader("Content-Type", "application/javascript");
            res.end(code);
          } catch (error) {
            next(error);
          }
          return;
        }

        if (inspect && pathname === inspectPath) {
          try {
            const requestedSource = request.searchParams.get("source");
            const sourcePath = requestedSource
              ? resolveClskId(decodeURIComponent(requestedSource), undefined)
              : entryPath;
            if (!sourcePath) {
              res.statusCode = 400;
              res.setHeader("Content-Type", "application/json");
              res.end(JSON.stringify({ error: "inspect source is required when no Closkell entry is configured" }));
              return;
            }
            if (!sourcePath.endsWith(".clsk")) {
              res.statusCode = 400;
              res.setHeader("Content-Type", "application/json");
              res.end(JSON.stringify({ error: "inspect source must end with .clsk" }));
              return;
            }

            const report = await inspectSource(sourcePath);
            res.statusCode = 200;
            res.setHeader("Cache-Control", "no-store");
            res.setHeader("Content-Type", "application/json");
            res.end(report);
          } catch (error) {
            res.statusCode = 500;
            res.setHeader("Content-Type", "application/json");
            res.end(JSON.stringify({ error: error.message }));
          }
          return;
        }

        if (!pathname.endsWith(".clsk")) {
          next();
          return;
        }

        try {
          const sourcePath = resolveClskId(decodeURIComponent(pathname), undefined);
          const output = await compile(sourcePath);
          const outputUrl = `/${toPosixPath(path.relative(config.root, output))}`;
          const transformed = await server.transformRequest(outputUrl);
          const code = transformed?.code ?? (await fs.readFile(output, "utf8"));
          res.statusCode = 200;
          res.setHeader("Cache-Control", "no-store");
          res.setHeader("Content-Type", "application/javascript");
          res.end(code);
        } catch (error) {
          next(error);
        }
      });
    },

    async handleHotUpdate(ctx) {
      if (!ctx.file.endsWith(".clsk")) return;
      await compileAndReload(ctx.server, ctx.file);
      return [];
    }
  };
}

export default closkell;
