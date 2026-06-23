import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";

const RUNTIME_PACKAGE = "@closkell/runtime";
const RUNTIME_OPTIMIZED_PREFIX = "@closkell_runtime";

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

function runCommand(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
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
    sourceMap = false,
    vendorRuntime = true,
    inspect = true,
    inspectPath = "/__closkell/inspect"
  } = options;

  let config;
  let sourceRoot;
  let manifestRoot;
  let compilerCommand = null;
  let entryPath = null;
  let outPath;
  let generatedRoot;
  const inFlight = new Map();

  function resolveFromRoot(value) {
    return path.resolve(config.root, value);
  }

  function resolveCacheDir() {
    return path.isAbsolute(config.cacheDir)
      ? config.cacheDir
      : path.resolve(config.root, config.cacheDir);
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

  async function outputIsFresh(output) {
    if (!existsSync(output)) return false;
    const outputMtime = await statMtimeMs(output);
    if (outputMtime < (await newestClskMtimeMs(sourceRoot))) return false;
    if (outputMtime < (await newestRustBuildInputMtimeMs(manifestRoot))) return false;
    if (sourceMap && !existsSync(sourceMapPath(output))) return false;
    if (!sourceMap) {
      const tail = await readFileTail(output, 256);
      if (tail.includes("sourceMappingURL=")) return false;
    }
    return true;
  }

  function sourceMapPath(output) {
    return `${output}.map`;
  }

  function vendoredRuntimeSourcePath() {
    return path.join(config.root, "node_modules", "@closkell", "runtime", "src", "index.js");
  }

  function entryNeedsVendoredRuntime(source) {
    return vendorRuntime && app && entryPath && samePath(source, entryPath);
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

  async function resolveCompilerCommand() {
    if (compilerCommand) return compilerCommand;
    if (binary) {
      compilerCommand = { command: binary, args: [] };
      return compilerCommand;
    }

    const envBinary = process.env.CLOSKELL_BIN;
    if (envBinary) {
      compilerCommand = { command: envBinary, args: [] };
      return compilerCommand;
    }

    const exe = process.platform === "win32" ? ".exe" : "";
    const workspaceRoot = manifestRoot;
    const rustMtime = await newestRustBuildInputMtimeMs(workspaceRoot);
    for (const profile of ["release", "debug"]) {
      const candidate = path.join(workspaceRoot, "target", profile, `closkell${exe}`);
      if (existsSync(candidate) && (await statMtimeMs(candidate)) >= rustMtime) {
        compilerCommand = { command: candidate, args: [] };
        return compilerCommand;
      }
    }

    compilerCommand = {
      command: "cargo",
      args: ["run", "-q", "--manifest-path", resolveFromRoot(manifestPath), "-p", packageName, "--"]
    };
    return compilerCommand;
  }

  async function compile(source, { force = false } = {}) {
    const resolved = path.resolve(source);
    const output = outputForSource(resolved);
    const key = `${resolved}\0${output}`;
    if (inFlight.has(key)) return inFlight.get(key);

    const task = (async () => {
      if (
        !force &&
        (await outputIsFresh(output)) &&
        (!entryNeedsVendoredRuntime(resolved) || existsSync(vendoredRuntimeSourcePath()))
      ) {
        return output;
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

      config.logger.info(`closkell: ${toPosixPath(path.relative(config.root, resolved))}`);
      await runCommand(compiler.command, args, config.root);
      return output;
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
    if (existsSync(vendored)) return vendored;

    if (allowCompile && vendorRuntime && entryPath && existsSync(entryPath)) {
      await compile(entryPath);
      if (existsSync(vendored)) return vendored;
    }

    const workspaceRuntime = path.join(manifestRoot, "runtime-js", "src", "index.js");
    if (existsSync(workspaceRuntime)) return workspaceRuntime;

    throw new Error("Closkell runtime source was not found");
  }

  function shouldCompileEntryOnStart() {
    if (!entryPath) return false;
    return entryConfigured || existsSync(entryPath);
  }

  async function compileAndReload(server, changedFile) {
    try {
      const source = shouldCompileEntryOnStart() ? entryPath : changedFile;
      const output = await compile(source, { force: true });
      const modules = server.moduleGraph.getModulesByFile(output);
      if (modules) {
        for (const mod of modules) server.moduleGraph.invalidateModule(mod);
      }
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

  return {
    name: "vite-plugin-closkell",
    enforce: "pre",

    config(userConfig = {}) {
      return {
        optimizeDeps: {
          exclude: withRuntimeOptimizeDepsExclude(userConfig.optimizeDeps?.exclude)
        }
      };
    },

    async configResolved(resolvedConfig) {
      config = resolvedConfig;
      manifestRoot = path.dirname(resolveFromRoot(manifestPath));
      entryPath = entry ? resolveFromRoot(entry) : null;
      generatedRoot = outDir ? resolveFromRoot(outDir) : path.join(resolveCacheDir(), "closkell");
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
        await compile(entryPath);
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
