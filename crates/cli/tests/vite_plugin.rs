use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn vite_plugin_serves_clsk_modules_as_transformed_js() {
    if !node_available() {
        eprintln!("skipping Vite plugin integration test because node is unavailable");
        return;
    }

    let hrweb_dir = workspace_root().join("projects").join("hrweb");
    if !hrweb_dir.join("node_modules").join("vite").is_dir()
        || !hrweb_dir
            .join("node_modules")
            .join("@closkell")
            .join("vite-plugin")
            .is_dir()
    {
        eprintln!("skipping Vite plugin integration test because hrweb npm deps are not installed");
        return;
    }

    let temp_dir = temp_dir("closkell-vite-plugin-smoke");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("src")).expect("temp Vite src dir should be created");
    fs::write(temp_dir.join("package.json"), "{\"type\":\"module\"}\n")
        .expect("package.json should be written");
    fs::write(
        temp_dir.join("index.html"),
        "<!doctype html><div id=\"root\"></div><script type=\"module\" src=\"/src/app.clsk\"></script>\n",
    )
    .expect("index.html should be written");
    fs::write(
        temp_dir.join("src").join("app.clsk"),
        "(def init {:label \"Plugin Ready\"})\n\
         (defn update [state msg] [state {:kind :none}])\n\
         (defn view [state] #html <main data-testid=\"vite-plugin-smoke\">{state.label}</main>)\n",
    )
    .expect("app.clsk should be written");

    let script = format!(
        r#"
import {{ createServer }} from "vite";
import {{ closkell }} from "@closkell/vite-plugin";
import {{ existsSync, mkdirSync, readFileSync, writeFileSync }} from "node:fs";
import path from "node:path";

const root = {root};
const manifestPath = {manifest_path};
const depsDir = path.join(root, "node_modules", ".vite", "deps");
const staleRuntime = path.join(depsDir, "@closkell_runtime.js");
const staleRuntimeMap = path.join(depsDir, "@closkell_runtime.js.map");
const staleMetadata = path.join(depsDir, "_metadata.json");
let server;

try {{
  mkdirSync(depsDir, {{ recursive: true }});
  writeFileSync(staleRuntime, "export const staleRuntime = true;\n");
  writeFileSync(staleRuntimeMap, "{{}}\n");
  writeFileSync(staleMetadata, JSON.stringify({{
    hash: "stale",
    configHash: "stale",
    lockfileHash: "stale",
    browserHash: "stale",
    optimized: {{
      "@closkell/runtime": {{
        src: "../../@closkell/runtime/src/index.js",
        file: "@closkell_runtime.js",
        fileHash: "stale",
        needsInterop: false
      }}
    }},
    chunks: {{}}
  }}, null, 2));

  server = await createServer({{
    root,
    configFile: false,
    logLevel: "error",
    appType: "spa",
    plugins: [
      closkell({{
        entry: "src/app.clsk",
        manifestPath,
        css: null,
        vendorRuntime: true,
        sourceMap: true
      }})
    ],
    optimizeDeps: {{
      exclude: ["already-excluded"]
    }},
    server: {{
      host: "127.0.0.1",
      port: 0
    }}
  }});
  await server.listen();
  const optimizeDepsExclude = server.config.optimizeDeps.exclude || [];
  if (!optimizeDepsExclude.includes("@closkell/runtime")) {{
    throw new Error(`Closkell runtime was not excluded from Vite dependency prebundling: ${{JSON.stringify(optimizeDepsExclude)}}`);
  }}
  if (!optimizeDepsExclude.includes("already-excluded")) {{
    throw new Error(`Closkell plugin dropped existing optimizeDeps.exclude entries: ${{JSON.stringify(optimizeDepsExclude)}}`);
  }}
  if (existsSync(staleRuntime) || existsSync(staleRuntimeMap)) {{
    throw new Error("Closkell plugin did not prune stale optimized runtime files");
  }}
  if (existsSync(staleMetadata) && readFileSync(staleMetadata, "utf8").includes("\"@closkell/runtime\"")) {{
    throw new Error("Closkell plugin did not prune stale optimized runtime metadata");
  }}
  const address = server.httpServer.address();
  const port = typeof address === "object" && address ? address.port : 0;
  if (!port) throw new Error("Vite did not expose a listening port");

  const response = await fetch(`http://127.0.0.1:${{port}}/src/app.clsk`);
  if (!response.ok) throw new Error(`direct .clsk request failed with ${{response.status}}`);
  if (response.headers.get("cache-control") !== "no-store") {{
    throw new Error(`direct .clsk request should not be cached in dev, got ${{response.headers.get("cache-control")}}`);
  }}
  const js = await response.text();
  if (js.includes("(def init")) throw new Error("Vite served raw Closkell instead of JavaScript");
  if (!js.includes("__closkellStartApp") || !js.includes("export const __closkellApp")) {{
    throw new Error(`transformed module did not include app bootstrap:\n${{js.slice(0, 500)}}`);
  }}
  if (js.includes("/node_modules/.vite/deps/@closkell_runtime")) {{
    throw new Error(`transformed module still points at Vite's optimized runtime cache:\n${{js.slice(0, 500)}}`);
  }}

  const staleRuntimeResponse = await fetch(`http://127.0.0.1:${{port}}/node_modules/.vite/deps/@closkell_runtime.js?v=stale`);
  if (!staleRuntimeResponse.ok) {{
    throw new Error(`stale optimized runtime URL failed with ${{staleRuntimeResponse.status}}`);
  }}
  const staleRuntimeJs = await staleRuntimeResponse.text();
  if (!staleRuntimeJs.includes("export function createDevtoolsOverlay")) {{
    throw new Error(`stale optimized runtime URL did not serve the current runtime source:\n${{staleRuntimeJs.slice(0, 500)}}`);
  }}

  const inspectResponse = await fetch(`http://127.0.0.1:${{port}}/__closkell/inspect`);
  if (!inspectResponse.ok) throw new Error(`inspect endpoint failed with ${{inspectResponse.status}}`);
  const report = await inspectResponse.json();
  if (!report.componentGraph?.some((entry) => entry.component === "view")) {{
    throw new Error(`inspect endpoint did not report the view component: ${{JSON.stringify(report)}}`);
  }}
  if (!report.statePathToSlots?.some((entry) => entry.path === "state.label")) {{
    throw new Error(`inspect endpoint did not report state.label slot reads: ${{JSON.stringify(report.statePathToSlots)}}`);
  }}
  if (!report.commandLogSchema?.some((entry) => entry.kind === "none")) {{
    throw new Error(`inspect endpoint did not report the command schema: ${{JSON.stringify(report.commandLogSchema)}}`);
  }}

  const generated = path.join(server.config.cacheDir, "closkell", "main.mjs");
  const runtime = path.join(root, "node_modules", "@closkell", "runtime", "src", "index.js");
  if (!existsSync(generated)) throw new Error("Vite plugin did not emit the generated app module");
  if (!existsSync(`${{generated}}.map`)) throw new Error("Vite plugin did not emit a source map");
  if (!existsSync(runtime)) throw new Error("Vite plugin did not vendor @closkell/runtime");
}} finally {{
  if (server) {{
    await Promise.race([
      server.close(),
      new Promise((resolve) => setTimeout(resolve, 1000))
    ]);
  }}
}}
"#,
        root = json_string_for_test(&temp_dir.display().to_string()),
        manifest_path =
            json_string_for_test(&workspace_root().join("Cargo.toml").display().to_string())
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .current_dir(&hrweb_dir)
        .output()
        .expect("node should run the Vite plugin smoke test");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "Vite plugin smoke test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn vite_plugin_resolves_clsk_imports_from_plain_js() {
    if !node_available() {
        eprintln!("skipping Vite plugin module import test because node is unavailable");
        return;
    }

    let hrweb_dir = workspace_root().join("projects").join("hrweb");
    if !hrweb_dir.join("node_modules").join("vite").is_dir()
        || !hrweb_dir
            .join("node_modules")
            .join("@closkell")
            .join("vite-plugin")
            .is_dir()
    {
        eprintln!(
            "skipping Vite plugin module import test because hrweb npm deps are not installed"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-vite-plugin-module");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("src")).expect("temp Vite src dir should be created");
    fs::write(temp_dir.join("package.json"), "{\"type\":\"module\"}\n")
        .expect("package.json should be written");
    fs::write(
        temp_dir.join("src").join("math.clsk"),
        "(def base 41)\n\
         (defn add-one [value] (+ value 1))\n\
         (def summary {:answer (add-one base) :label (str \"answer \" (add-one base))})\n",
    )
    .expect("math.clsk should be written");
    fs::write(
        temp_dir.join("src").join("main.js"),
        "import { summary, add_one } from './math.clsk';\n\
         export const answer = summary.answer;\n\
         export const label = summary.label;\n\
         export const next = add_one(answer);\n",
    )
    .expect("main.js should be written");

    let script = format!(
        r#"
import {{ createServer }} from "vite";
import {{ closkell }} from "@closkell/vite-plugin";
import {{ existsSync }} from "node:fs";
import path from "node:path";

const root = {root};
const manifestPath = {manifest_path};
let server;

try {{
  server = await createServer({{
    root,
    configFile: false,
    logLevel: "error",
    appType: "custom",
    plugins: [
      closkell({{
        manifestPath,
        sourceMap: true
      }})
    ]
  }});

  const mod = await server.ssrLoadModule("/src/main.js");
  if (mod.answer !== 42 || mod.next !== 43 || mod.label !== "answer 42") {{
    throw new Error(`unexpected Closkell module exports: ${{JSON.stringify({{ answer: mod.answer, next: mod.next, label: mod.label }})}}`);
  }}

  const generated = path.join(server.config.cacheDir, "closkell", "math.mjs");
  if (!existsSync(generated)) throw new Error("plain JS import did not emit the generated Closkell module");
  if (!existsSync(`${{generated}}.map`)) throw new Error("plain JS import did not emit a source map");
}} finally {{
  if (server) {{
    await Promise.race([
      server.close(),
      new Promise((resolve) => setTimeout(resolve, 1000))
    ]);
  }}
}}
"#,
        root = json_string_for_test(&temp_dir.display().to_string()),
        manifest_path =
            json_string_for_test(&workspace_root().join("Cargo.toml").display().to_string())
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .current_dir(&hrweb_dir)
        .output()
        .expect("node should run the Vite plugin module import test");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "Vite plugin module import test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn vite_plugin_registers_closkell_tests_for_vitest() {
    if !node_available() {
        eprintln!("skipping Vite plugin Vitest registration test because node is unavailable");
        return;
    }

    let hrweb_dir = workspace_root().join("projects").join("hrweb");
    if !hrweb_dir.join("node_modules").join("vite").is_dir()
        || !hrweb_dir
            .join("node_modules")
            .join("@closkell")
            .join("vite-plugin")
            .is_dir()
    {
        eprintln!(
            "skipping Vite plugin Vitest registration test because hrweb npm deps are not installed"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-vite-plugin-vitest");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("src")).expect("temp Vite src dir should be created");
    fs::write(temp_dir.join("package.json"), "{\"type\":\"module\"}\n")
        .expect("package.json should be written");
    fs::write(
        temp_dir.join("src").join("sample_test.clsk"),
        "(import \"closkell/test\" [describe test expect= expect-match expect-throws])\n\
         \n\
         (describe \"closkell vitest\"\n\
           (test \"registers generated tests\"\n\
             (expect= (+ 1 1) 2)\n\
             (expect-match {:kind :ready :value 42} {:kind :ready})\n\
             (expect-throws (fn [] (fail \"boom\")) \"boom\")))\n",
    )
    .expect("sample_test.clsk should be written");
    let fake_vitest = temp_dir.join("fake-vitest.mjs");
    fs::write(
        &fake_vitest,
        "export function describe(name, fn) {\n  globalThis.__closkellVitestEvents.push(['describe', name]);\n  fn();\n}\n\
         export function test(name, fn) {\n  try {\n    fn();\n    globalThis.__closkellVitestEvents.push(['test', name, 'ok']);\n  } catch (error) {\n    globalThis.__closkellVitestEvents.push(['test', name, 'failed', error?.message || String(error)]);\n  }\n}\n",
    )
    .expect("fake Vitest module should be written");

    let script = format!(
        r#"
import {{ createServer }} from "vite";
import {{ closkell }} from "@closkell/vite-plugin";

const root = {root};
const manifestPath = {manifest_path};
const fakeVitest = {fake_vitest};
let server;

try {{
  globalThis.__closkellVitestEvents = [];
  server = await createServer({{
    root,
    configFile: false,
    logLevel: "error",
    appType: "custom",
    plugins: [
      closkell({{
        entry: null,
        app: false,
        vitest: true,
        sourceRoot: "src",
        manifestPath,
        inspect: false,
        vendorRuntime: false
      }})
    ],
    resolve: {{
      alias: [
        {{ find: "vitest", replacement: fakeVitest }}
      ]
    }},
    server: {{
      host: "127.0.0.1",
      port: 0
    }}
  }});

  await server.listen();
  await server.ssrLoadModule("/src/sample_test.clsk");
  const events = globalThis.__closkellVitestEvents;
  if (!events.some((event) => event[0] === "describe" && event[1] === "closkell vitest")) {{
    throw new Error(`Closkell tests did not register a Vitest describe: ${{JSON.stringify(events)}}`);
  }}
  if (!events.some((event) => event[0] === "test" && event[1] === "registers generated tests" && event[2] === "ok")) {{
    throw new Error(`Closkell tests did not register/pass a Vitest test: ${{JSON.stringify(events)}}`);
  }}
}} finally {{
  if (server) {{
    await Promise.race([
      server.close(),
      new Promise((resolve) => setTimeout(resolve, 1000))
    ]);
  }}
}}
"#,
        root = json_string_for_test(&temp_dir.display().to_string()),
        manifest_path =
            json_string_for_test(&workspace_root().join("Cargo.toml").display().to_string()),
        fake_vitest = json_string_for_test(&fake_vitest.display().to_string())
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .current_dir(&hrweb_dir)
        .output()
        .expect("node should run the Vite plugin Vitest registration test");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "Vite plugin Vitest registration test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn vite_plugin_builds_direct_clsk_entry_with_tailwind() {
    if !node_available() {
        eprintln!("skipping Vite Tailwind integration test because node is unavailable");
        return;
    }

    let hrweb_dir = workspace_root().join("projects").join("hrweb");
    if !hrweb_dir.join("node_modules").join("vite").is_dir()
        || !hrweb_dir
            .join("node_modules")
            .join("@closkell")
            .join("vite-plugin")
            .is_dir()
        || !hrweb_dir
            .join("node_modules")
            .join("@tailwindcss")
            .join("vite")
            .is_dir()
    {
        eprintln!(
            "skipping Vite Tailwind integration test because hrweb npm deps are not installed"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-vite-plugin-tailwind");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(temp_dir.join("src")).expect("temp Vite src dir should be created");
    fs::write(temp_dir.join("package.json"), "{\"type\":\"module\"}\n")
        .expect("package.json should be written");
    fs::write(
        temp_dir.join("index.html"),
        "<!doctype html><div id=\"root\"></div><script type=\"module\" src=\"/src/app.clsk\"></script>\n",
    )
    .expect("index.html should be written");
    fs::write(
        temp_dir.join("src").join("styles.css"),
        format!(
            "@import \"{}\";\n\n@theme {{\n  --font-sans: Inter, ui-sans-serif, system-ui, sans-serif;\n}}\n",
            posix_path(
                &hrweb_dir
                    .join("node_modules")
                    .join("tailwindcss")
                    .join("index.css")
            )
        ),
    )
    .expect("styles.css should be written");
    fs::write(
        temp_dir.join("src").join("app.clsk"),
        "(def init {:label \"Tailwind Ready\"})\n\
         (defn update [state msg] [state {:kind :none}])\n\
         (defn view [state]\n\
           #html <main class=\"grid min-h-screen bg-[#123456] p-4 text-[17px]\" data-testid=\"tailwind-smoke\">{state.label}</main>)\n",
    )
    .expect("app.clsk should be written");

    let script = format!(
        r##"
import {{ build }} from "vite";
import tailwindcss from "@tailwindcss/vite";
import {{ closkell }} from "@closkell/vite-plugin";
import fs from "node:fs/promises";
import path from "node:path";

const root = {root};
const manifestPath = {manifest_path};

await build({{
  root,
  configFile: false,
  logLevel: "error",
  plugins: [
    tailwindcss(),
    closkell({{
      entry: "src/app.clsk",
      manifestPath,
      css: "src/styles.css",
      vendorRuntime: true,
      sourceMap: false
    }})
  ],
  build: {{
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false
  }}
}});

const assetsDir = path.join(root, "dist", "assets");
const assets = await fs.readdir(assetsDir);
const cssFiles = assets.filter((asset) => asset.endsWith(".css"));
const jsFiles = assets.filter((asset) => asset.endsWith(".js"));
if (cssFiles.length !== 1) throw new Error(`expected one CSS asset, found ${{cssFiles.join(", ")}}`);
if (jsFiles.length !== 1) throw new Error(`expected one JS asset, found ${{jsFiles.join(", ")}}`);

const css = await fs.readFile(path.join(assetsDir, cssFiles[0]), "utf8");
const js = await fs.readFile(path.join(assetsDir, jsFiles[0]), "utf8");
if (!css.includes(".grid") || !css.includes("display:grid")) {{
  throw new Error(`Tailwind did not generate the .grid utility from app.clsk:\n${{css}}`);
}}
if (!css.includes("min-height:100vh")) {{
  throw new Error(`Tailwind did not generate the min-h-screen utility from app.clsk:\n${{css}}`);
}}
if (!css.includes("#123456")) {{
  throw new Error(`Tailwind did not generate the arbitrary background utility from app.clsk:\n${{css}}`);
}}
if (!js.includes("Tailwind Ready") || js.includes("(def init")) {{
  throw new Error("Vite build did not bundle transformed Closkell JavaScript");
}}
"##,
        root = json_string_for_test(&temp_dir.display().to_string()),
        manifest_path =
            json_string_for_test(&workspace_root().join("Cargo.toml").display().to_string())
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .current_dir(&hrweb_dir)
        .output()
        .expect("node should run the Vite Tailwind integration test");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "Vite Tailwind integration test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn temp_dir(name: &str) -> PathBuf {
    env::temp_dir().join(format!("{}-{}", name, std::process::id()))
}

fn node_available() -> bool {
    Command::new("node").arg("--version").output().is_ok()
}

fn posix_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn json_string_for_test(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
