"use strict";

const cp = require("child_process");
const fs = require("fs");
const path = require("path");
const vscode = require("vscode");

const LANGUAGE_ID = "closkell";

function activate(context) {
  const diagnostics = vscode.languages.createDiagnosticCollection("closkell");
  const controller = new CloskellController(diagnostics);

  context.subscriptions.push(controller, diagnostics);
  context.subscriptions.push(
    vscode.languages.registerDocumentFormattingEditProvider(
      { language: LANGUAGE_ID },
      {
        provideDocumentFormattingEdits(document) {
          return controller.formatDocument(document);
        },
      }
    )
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("closkell.checkDocument", () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && isCloskellDocument(editor.document)) {
        return controller.lintDocument(editor.document, true);
      }
      return undefined;
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((document) => {
      controller.scheduleLint(document, 100);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((event) => {
      if (getConfig(event.document).get("lint.onChange", true)) {
        controller.scheduleLint(event.document, 500);
      }
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((document) => {
      if (getConfig(document).get("lint.onSave", true)) {
        controller.scheduleLint(document, 0);
      }
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((document) => {
      controller.forgetDocument(document);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("closkell")) {
        controller.resetToolError();
        for (const document of vscode.workspace.textDocuments) {
          controller.scheduleLint(document, 100);
        }
      }
    })
  );

  for (const document of vscode.workspace.textDocuments) {
    controller.scheduleLint(document, 100);
  }
}

function deactivate() {}

class CloskellController {
  constructor(diagnostics) {
    this.diagnostics = diagnostics;
    this.timers = new Map();
    this.diagnosticUrisBySource = new Map();
    this.lastToolError = "";
  }

  dispose() {
    for (const timer of this.timers.values()) {
      clearTimeout(timer);
    }
    this.timers.clear();
  }

  resetToolError() {
    this.lastToolError = "";
  }

  forgetDocument(document) {
    const key = document.uri.toString();
    const timer = this.timers.get(key);
    if (timer) {
      clearTimeout(timer);
      this.timers.delete(key);
    }
    this.clearDiagnosticsForSource(document.uri);
  }

  scheduleLint(document, delayMs) {
    if (!isCloskellDocument(document)) {
      return;
    }

    const key = document.uri.toString();
    const existing = this.timers.get(key);
    if (existing) {
      clearTimeout(existing);
    }

    const timer = setTimeout(() => {
      this.timers.delete(key);
      this.lintDocument(document, false);
    }, delayMs);
    this.timers.set(key, timer);
  }

  async formatDocument(document) {
    if (!isCloskellDocument(document) || !getConfig(document).get("format.enable", true)) {
      return [];
    }

    const result = await runCloskell(document, ["fmt", "--stdin"], document.getText());
    if (result.error || result.code !== 0) {
      const detail = commandFailureMessage(result);
      vscode.window.showWarningMessage(`Closkell format failed: ${detail}`);
      return [];
    }

    const formatted = result.stdout;
    if (!formatted || formatted === document.getText()) {
      return [];
    }

    return [vscode.TextEdit.replace(fullDocumentRange(document), formatted)];
  }

  async lintDocument(document, manual) {
    if (!isCloskellDocument(document)) {
      return;
    }
    if (!getConfig(document).get("lint.enable", true)) {
      this.clearDiagnosticsForSource(document.uri);
      return;
    }

    if (document.uri.scheme !== "file") {
      this.clearDiagnosticsForSource(document.uri);
      return;
    }

    const result = await runCloskell(
      document,
      ["check", "--json", "--stdin", document.uri.fsPath],
      document.getText()
    );
    if (result.error) {
      this.notifyToolError(commandFailureMessage(result), manual);
      return;
    }

    const parsed = parseDiagnosticJson(result.stdout);
    if (!parsed) {
      this.notifyToolError(result.stderr || "check --json did not return valid JSON", manual);
      return;
    }

    this.lastToolError = "";
    this.applyDiagnostics(document.uri, document.uri.fsPath, parsed.diagnostics || []);
  }

  applyDiagnostics(sourceUri, sourcePath, compilerDiagnostics) {
    this.clearDiagnosticsForSource(sourceUri);

    const grouped = new Map();
    for (const item of compilerDiagnostics) {
      if (!item || !item.file || !item.range) {
        continue;
      }

      const uri = samePath(item.file, sourcePath) ? sourceUri : vscode.Uri.file(item.file);
      const diagnostic = new vscode.Diagnostic(
        toRange(item.range),
        String(item.message || "Closkell diagnostic"),
        toSeverity(item.severity)
      );
      diagnostic.source = "closkell";

      const key = uri.toString();
      const group = grouped.get(key) || { uri, diagnostics: [] };
      group.diagnostics.push(diagnostic);
      grouped.set(key, group);
    }

    const managed = [];
    for (const [key, group] of grouped) {
      this.diagnostics.set(group.uri, group.diagnostics);
      managed.push(key);
    }
    this.diagnosticUrisBySource.set(sourceUri.toString(), managed);
  }

  clearDiagnosticsForSource(sourceUri) {
    const sourceKey = sourceUri.toString();
    const managed = this.diagnosticUrisBySource.get(sourceKey) || [];
    for (const uriString of managed) {
      this.diagnostics.delete(vscode.Uri.parse(uriString));
    }
    this.diagnosticUrisBySource.delete(sourceKey);
  }

  notifyToolError(message, manual) {
    const trimmed = String(message || "unknown error").trim();
    if (!trimmed || (!manual && trimmed === this.lastToolError)) {
      return;
    }
    this.lastToolError = trimmed;
    const text = manual
      ? `Closkell check failed: ${trimmed}`
      : `Closkell diagnostics are unavailable: ${trimmed}`;
    vscode.window.showWarningMessage(text);
  }
}

async function runCloskell(document, args, stdin) {
  const resolved = resolveCloskellCommand(document);
  const timeoutMs = getConfig(document).get("commandTimeoutMs", 10000);
  return runProcess(resolved, args, timeoutMs, stdin);
}

function runProcess(resolved, args, timeoutMs, stdin) {
  return new Promise((resolve) => {
    let stdout = "";
    let stderr = "";
    let settled = false;
    const fullArgs = resolved.prefixArgs.concat(args);
    const child = cp.spawn(resolved.command, fullArgs, {
      cwd: resolved.cwd,
      windowsHide: true,
    });

    const timer = setTimeout(() => {
      if (!settled) {
        child.kill();
        settled = true;
        resolve({
          code: null,
          stdout,
          stderr,
          error: new Error(`timed out after ${timeoutMs} ms: ${displayCommand(resolved, args)}`),
        });
      }
    }, timeoutMs);

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", (error) => {
      if (!settled) {
        clearTimeout(timer);
        settled = true;
        resolve({ code: null, stdout, stderr, error });
      }
    });
    child.on("close", (code) => {
      if (!settled) {
        clearTimeout(timer);
        settled = true;
        resolve({ code, stdout, stderr, error: null });
      }
    });
    if (typeof stdin === "string") {
      child.stdin.end(stdin);
    } else {
      child.stdin.end();
    }
  });
}

function resolveCloskellCommand(document) {
  const config = getConfig(document);
  const configured = String(config.get("executablePath", "")).trim();
  const cwd = document.uri.scheme === "file" ? path.dirname(document.uri.fsPath) : undefined;
  if (configured) {
    return { command: configured, prefixArgs: [], cwd };
  }

  const root = findCompilerWorkspace(document);
  if (root) {
    const binary = findBuiltCompiler(root);
    if (binary) {
      return { command: binary, prefixArgs: [], cwd: root };
    }
    return {
      command: String(config.get("cargoCommand", "cargo")),
      prefixArgs: ["run", "-q", "-p", "cli", "--"],
      cwd: root,
    };
  }

  return { command: "closkell", prefixArgs: [], cwd };
}

function findCompilerWorkspace(document) {
  const starts = [];
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  if (folder) {
    starts.push(folder.uri.fsPath);
  }
  if (document.uri.scheme === "file") {
    starts.push(path.dirname(document.uri.fsPath));
  }

  for (const start of starts) {
    let current = start;
    while (current && current !== path.dirname(current)) {
      if (
        fs.existsSync(path.join(current, "Cargo.toml")) &&
        fs.existsSync(path.join(current, "crates", "cli", "Cargo.toml"))
      ) {
        return current;
      }
      current = path.dirname(current);
    }
  }
  return null;
}

function findBuiltCompiler(root) {
  const executable = process.platform === "win32" ? "closkell.exe" : "closkell";
  const newestInput = newestCompilerInputMtimeMs(root);
  const candidates = [
    path.join(root, "target", "release", executable),
    path.join(root, "target", "debug", executable),
  ];
  return (
    candidates.find(
      (candidate) => fs.existsSync(candidate) && statMtimeMs(candidate) >= newestInput
    ) || null
  );
}

function newestCompilerInputMtimeMs(root) {
  return Math.max(0, ...compilerInputFiles(root).map(statMtimeMs));
}

function compilerInputFiles(root) {
  const files = [path.join(root, "Cargo.toml"), path.join(root, "Cargo.lock")];
  const crates = path.join(root, "crates");
  let entries = [];
  try {
    entries = fs.readdirSync(crates, { withFileTypes: true });
  } catch (error) {
    if (error.code !== "ENOENT") {
      throw error;
    }
  }

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    const crateRoot = path.join(crates, entry.name);
    files.push(path.join(crateRoot, "Cargo.toml"));
    files.push(path.join(crateRoot, "build.rs"));
    collectFiles(path.join(crateRoot, "src"), (file) => file.endsWith(".rs"), files);
  }
  return files;
}

function collectFiles(root, include, files) {
  let entries = [];
  try {
    entries = fs.readdirSync(root, { withFileTypes: true });
  } catch (error) {
    if (error.code !== "ENOENT") {
      throw error;
    }
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
    if (error.code === "ENOENT") {
      return 0;
    }
    throw error;
  }
}

function parseDiagnosticJson(stdout) {
  const trimmed = String(stdout || "").trim();
  if (!trimmed) {
    return null;
  }
  try {
    return JSON.parse(trimmed);
  } catch (_error) {
    const start = trimmed.indexOf("{\"diagnostics\"");
    const end = trimmed.lastIndexOf("}");
    if (start >= 0 && end > start) {
      try {
        return JSON.parse(trimmed.slice(start, end + 1));
      } catch (_nestedError) {
        return null;
      }
    }
    return null;
  }
}

function toRange(range) {
  const startLine = Math.max(0, Number(range.start && range.start.line ? range.start.line : 1) - 1);
  const startColumn = Math.max(
    0,
    Number(range.start && range.start.column ? range.start.column : 1) - 1
  );
  let endLine = Math.max(0, Number(range.end && range.end.line ? range.end.line : 1) - 1);
  let endColumn = Math.max(0, Number(range.end && range.end.column ? range.end.column : 1) - 1);

  if (endLine < startLine || (endLine === startLine && endColumn <= startColumn)) {
    endLine = startLine;
    endColumn = startColumn + 1;
  }

  return new vscode.Range(startLine, startColumn, endLine, endColumn);
}

function toSeverity(severity) {
  switch (severity) {
    case "warning":
      return vscode.DiagnosticSeverity.Warning;
    case "error":
    default:
      return vscode.DiagnosticSeverity.Error;
  }
}

function commandFailureMessage(result) {
  if (result.error) {
    return result.error.message;
  }
  const output = `${result.stderr || ""}${result.stdout || ""}`.trim();
  return output || `exit code ${result.code}`;
}

function displayCommand(resolved, args) {
  return [resolved.command].concat(resolved.prefixArgs, args).join(" ");
}

function fullDocumentRange(document) {
  const lastLine = document.lineAt(document.lineCount - 1);
  return new vscode.Range(0, 0, document.lineCount - 1, lastLine.text.length);
}

function samePath(left, right) {
  const a = path.resolve(left);
  const b = path.resolve(right);
  return process.platform === "win32" ? a.toLowerCase() === b.toLowerCase() : a === b;
}

function isCloskellDocument(document) {
  return (
    document &&
    (document.languageId === LANGUAGE_ID ||
      (document.uri.scheme === "file" && path.extname(document.uri.fsPath) === ".clsk"))
  );
}

function getConfig(resource) {
  const uri = resource && resource.uri ? resource.uri : resource;
  return vscode.workspace.getConfiguration("closkell", uri);
}

module.exports = {
  activate,
  deactivate,
};
