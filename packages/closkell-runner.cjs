"use strict";

const cp = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

function resolveCloskellCommand(options = {}) {
  const workspaceRoot = path.resolve(options.workspaceRoot || path.join(__dirname, ".."));
  const cwd = options.cwd || process.cwd();
  const explicit = options.binary || process.env.CLOSKELL_BIN;
  if (explicit) {
    return { command: explicit, args: [], cwd, source: "configured" };
  }

  const executable = process.platform === "win32" ? "closkell.exe" : "closkell";
  const newestInput = newestCompilerInputMtimeMs(workspaceRoot);
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(workspaceRoot, "target", profile, executable);
    if (fs.existsSync(candidate) && statMtimeMs(candidate) >= newestInput) {
      return { command: candidate, args: [], cwd, source: profile };
    }
  }

  return {
    command: options.cargoCommand || process.env.CARGO || "cargo",
    args: [
      "run",
      "-q",
      "--manifest-path",
      options.manifestPath || path.join(workspaceRoot, "Cargo.toml"),
      "-p",
      options.packageName || "cli",
      "--",
    ],
    cwd,
    source: "cargo",
  };
}

function newestCompilerInputMtimeMs(workspaceRoot) {
  const inputs = compilerInputFiles(workspaceRoot);
  return Math.max(0, ...inputs.map(statMtimeMs));
}

function compilerInputFiles(workspaceRoot) {
  const inputs = [
    path.join(workspaceRoot, "Cargo.toml"),
    path.join(workspaceRoot, "Cargo.lock"),
  ];

  const crates = path.join(workspaceRoot, "crates");
  let crateEntries = [];
  try {
    crateEntries = fs.readdirSync(crates, { withFileTypes: true });
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }

  for (const entry of crateEntries) {
    if (!entry.isDirectory()) continue;
    const crateRoot = path.join(crates, entry.name);
    inputs.push(path.join(crateRoot, "Cargo.toml"));
    inputs.push(path.join(crateRoot, "build.rs"));
    collectFiles(path.join(crateRoot, "src"), (file) => file.endsWith(".rs"), inputs);
  }

  return inputs;
}

function collectFiles(root, include, files) {
  let entries = [];
  try {
    entries = fs.readdirSync(root, { withFileTypes: true });
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
    return files;
  }

  for (const entry of entries) {
    const file = path.join(root, entry.name);
    if (entry.isDirectory()) {
      collectFiles(file, include, files);
    } else if (entry.isFile() && include(file)) {
      files.push(file);
    }
  }
  return files;
}

function statMtimeMs(file) {
  try {
    return fs.statSync(file).mtimeMs;
  } catch (error) {
    if (error.code === "ENOENT") return 0;
    throw error;
  }
}

function runCloskell(args, options = {}) {
  const resolved = resolveCloskellCommand(options);
  const fullArgs = resolved.args.concat(args);
  return new Promise((resolve, reject) => {
    const child = cp.spawn(resolved.command, fullArgs, {
      cwd: resolved.cwd,
      stdio: "inherit",
      windowsHide: true,
    });
    child.on("error", reject);
    child.on("close", (code) => resolve(code));
  });
}

async function main() {
  try {
    const code = await runCloskell(process.argv.slice(2));
    process.exitCode = code == null ? 1 : code;
  } catch (error) {
    console.error(error && error.message ? error.message : String(error));
    process.exitCode = 1;
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  compilerInputFiles,
  newestCompilerInputMtimeMs,
  resolveCloskellCommand,
  runCloskell,
};
