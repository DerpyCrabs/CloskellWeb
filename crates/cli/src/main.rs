use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    env, fs,
    io::{self, Read},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
    thread,
    time::{Duration, SystemTime},
};

use syntax::{
    Diagnostic, Expr, ExprKind, Severity, SourceFile, line_column, parse_source, render_diagnostics,
};
use template_ir::{NamedTemplate, NodeKind, SlotKind};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error);
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };

    match command {
        "check" => {
            let path = require_check_path(&args)?;
            let json = has_flag(&args, "--json");
            let cache_debug = has_flag(&args, "--cache-debug");
            let print_forms = !json && (has_flag(&args, "--types") || has_flag(&args, "--verbose"));
            let source_override = if has_flag(&args, "--stdin") {
                Some(SourceOverride::read(&path)?)
            } else {
                None
            };
            let cache_probe = if json && source_override.is_none() && !print_forms {
                match check_cache_probe(&path) {
                    Ok(probe) => Some(probe),
                    Err(reason) => {
                        if cache_debug {
                            eprintln!("closkell cache disabled: {}", reason);
                        }
                        None
                    }
                }
            } else {
                None
            };
            if let Some(probe) = &cache_probe {
                if let Some(cached) = read_check_cache(probe) {
                    if cache_debug {
                        eprintln!(
                            "closkell cache hit: check {} ({} modules)",
                            probe.key, probe.module_count
                        );
                    }
                    println!("{}", cached.diagnostics_json);
                    return if cached.ok {
                        Ok(())
                    } else {
                        Err(format!("check failed: {}", path.display()))
                    };
                }
                if cache_debug {
                    eprintln!(
                        "closkell cache miss: check {} ({} modules)",
                        probe.key, probe.module_count
                    );
                }
            }
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            let mut reporter = CheckReporter::new(!json);
            let result = check_file_with_reporter(
                &path,
                &mut modules,
                &mut checking,
                print_forms,
                &mut reporter,
                source_override.as_ref(),
            );
            if json {
                let output = diagnostics_json(&reporter.diagnostics);
                if let Some(probe) = &cache_probe {
                    match write_check_cache(probe, result.is_ok(), &output) {
                        Ok(()) if cache_debug => {
                            eprintln!("closkell cache write: check {}", probe.key)
                        }
                        Err(error) if cache_debug => {
                            eprintln!("closkell cache write failed: {}", error)
                        }
                        _ => {}
                    }
                }
                println!("{}", output);
            }
            result.map(|_| ())
        }
        "expand" => {
            let path = require_path(&args)?;
            let (input, source) = parse_file(&path)?;
            let imports = parse_imports(&input, &source)?;
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            for import in &imports {
                if !is_closkell_import_path(&import.path) {
                    continue;
                }
                let import_source = resolve_import_source(&path, &import.path)?;
                check_file(&import_source, &mut modules, &mut checking, false)?;
            }
            let imported_macros = imported_macros_from_imports(&path, &imports, &modules)?;
            let expanded =
                macro_expand::expand_source_with_imported_macros(&source, &imported_macros);
            if !expanded.diagnostics.is_empty() {
                println!("{}", render_diagnostics(&input, &expanded.diagnostics));
            }
            println!("{}", expanded.source.pretty());
            if !expanded.diagnostics.is_empty() {
                return Err("expand failed".to_string());
            }
            Ok(())
        }
        "build" => {
            let path = require_path(&args)?;
            let output = output_path(&args);
            let json = has_flag(&args, "--json");
            let source_maps = has_flag(&args, "--sourcemap") || has_flag(&args, "--source-map");
            let app = parse_app_options(&args)?;
            if json && output.is_none() {
                let error = "build --json expects --out".to_string();
                println!(
                    "{}",
                    build_report_json(&path, output.as_ref(), false, &[], &[], Some(&error))
                );
                return Err(error);
            }
            if source_maps && output.is_none() {
                return Err("build --sourcemap expects --out".to_string());
            }
            if app.is_some() && output.is_none() {
                return Err("build --app expects --out".to_string());
            }
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            let mut reporter = CheckReporter::new(!json);
            let checked = check_file_with_reporter(
                &path,
                &mut modules,
                &mut checking,
                false,
                &mut reporter,
                None,
            );
            let module = match checked {
                Ok(module) => module,
                Err(error) => {
                    if json {
                        println!(
                            "{}",
                            build_report_json(
                                &path,
                                output.as_ref(),
                                false,
                                &[],
                                &reporter.diagnostics,
                                Some(&error)
                            )
                        );
                    }
                    return Err(error);
                }
            };
            if app.is_some() {
                if let Err(error) = require_app_exports(&path, &module.exports) {
                    if json {
                        println!(
                            "{}",
                            build_report_json(
                                &path,
                                output.as_ref(),
                                false,
                                &[],
                                &reporter.diagnostics,
                                Some(&error)
                            )
                        );
                    }
                    return Err(error);
                }
            }

            if let Some(output) = output {
                let mut visited = HashSet::new();
                let options = BuildOptions {
                    source_maps,
                    app,
                    emit_options: emit_options_from_env(),
                };
                let mut artifacts = Vec::new();
                if let Err(error) = build_file(
                    &path,
                    &output,
                    &mut visited,
                    &options,
                    &modules,
                    &mut artifacts,
                    "entry",
                ) {
                    if json {
                        println!(
                            "{}",
                            build_report_json(
                                &path,
                                Some(&output),
                                false,
                                &artifacts,
                                &reporter.diagnostics,
                                Some(&error)
                            )
                        );
                    }
                    return Err(error);
                }
                if json {
                    println!(
                        "{}",
                        build_report_json(
                            &path,
                            Some(&output),
                            true,
                            &artifacts,
                            &reporter.diagnostics,
                            None
                        )
                    );
                }
            } else {
                let emitted = build_single_module(&path, &modules, &emit_options_from_env())?;
                print!("{}", emitted);
            }
            Ok(())
        }
        "fmt" => {
            let (input, source) = if has_flag(&args, "--stdin") {
                let input = read_stdin()?;
                let source = parse_source(&input);
                (input, source)
            } else {
                let path = require_path(&args)?;
                parse_file(&path)?
            };
            if source.has_errors() {
                return Err(format!(
                    "fmt failed during parsing:\n{}",
                    render_diagnostics(&input, &source.diagnostics)
                ));
            }
            println!("{}", source.pretty());
            Ok(())
        }
        "inspect" => {
            let path = require_inspect_path(&args)?;
            let cache_debug = has_flag(&args, "--cache-debug");
            let cache_probe = match inspect_cache_probe(&path) {
                Ok(probe) => Some(probe),
                Err(reason) => {
                    if cache_debug {
                        eprintln!("closkell cache disabled: {}", reason);
                    }
                    None
                }
            };
            if let Some(probe) = &cache_probe {
                if let Some(cached) = read_inspect_cache(probe) {
                    if cache_debug {
                        eprintln!(
                            "closkell cache hit: inspect {} ({} modules)",
                            probe.key, probe.module_count
                        );
                    }
                    println!("{}", cached.report_json);
                    return Ok(());
                }
                if cache_debug {
                    eprintln!(
                        "closkell cache miss: inspect {} ({} modules)",
                        probe.key, probe.module_count
                    );
                }
            }
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            check_file(&path, &mut modules, &mut checking, false)?;
            let report = inspect_file(&path, &modules)?;
            if let Some(probe) = &cache_probe {
                match write_inspect_cache(probe, &report) {
                    Ok(()) if cache_debug => {
                        eprintln!("closkell cache write: inspect {}", probe.key)
                    }
                    Err(error) if cache_debug => {
                        eprintln!("closkell cache write failed: {}", error)
                    }
                    _ => {}
                }
            }
            println!("{}", report);
            Ok(())
        }
        "test" => {
            let path = require_path(&args)?;
            let json = has_flag(&args, "--json");
            run_module_tests(&path, json)
        }
        "dev" if args.iter().any(|arg| arg == "--watch") => run_dev_watch(&args),
        "dev" => Err("dev expects --watch".to_string()),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command `{}`", other)),
    }
}

#[derive(Clone, Debug)]
struct ModuleInfo {
    input: String,
    exports: HashSet<String>,
    bindings: Vec<typecheck::ExportedBinding>,
    type_declarations: Vec<typecheck::TypeDeclaration>,
    macros: HashMap<String, macro_expand::MacroDef>,
    command_shapes_by_binding: HashMap<String, Vec<CommandShape>>,
    message_kinds_by_binding: HashMap<String, BTreeSet<String>>,
    source: SourceFile,
    expr_types: BTreeMap<usize, String>,
}

#[derive(Clone, Debug)]
struct CollectedDiagnostic {
    file: String,
    code: String,
    severity: Severity,
    message: String,
    expected: Option<String>,
    actual: Option<String>,
    fixes: Vec<DiagnosticFix>,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Clone, Debug)]
struct DiagnosticFix {
    title: String,
    replacement: Option<String>,
}

#[derive(Clone, Debug)]
struct DiagnosticDetails {
    code: String,
    expected: Option<String>,
    actual: Option<String>,
    fixes: Vec<DiagnosticFix>,
}

#[derive(Clone, Debug)]
struct CheckReporter {
    print_diagnostics: bool,
    diagnostics: Vec<CollectedDiagnostic>,
}

#[derive(Clone, Debug)]
struct SourceOverride {
    canonical: PathBuf,
    input: String,
}

impl CheckReporter {
    fn new(print_diagnostics: bool) -> Self {
        Self {
            print_diagnostics,
            diagnostics: Vec::new(),
        }
    }

    fn report(&mut self, path: &Path, input: &str, diagnostics: &[Diagnostic]) {
        if diagnostics.is_empty() {
            return;
        }

        if self.print_diagnostics {
            println!("{}", render_diagnostics(input, diagnostics));
        }

        let file =
            output_path_string(&fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
        for diagnostic in diagnostics {
            let (line, column) = line_column(input, diagnostic.span.start);
            let (end_line, end_column) = line_column(input, diagnostic.span.end);
            let details = diagnostic_details(&diagnostic.message);
            self.diagnostics.push(CollectedDiagnostic {
                file: file.clone(),
                code: details.code,
                severity: diagnostic.severity.clone(),
                message: diagnostic.message.clone(),
                expected: details.expected,
                actual: details.actual,
                fixes: details.fixes,
                start: diagnostic.span.start,
                end: diagnostic.span.end,
                line,
                column,
                end_line,
                end_column,
            });
        }
    }
}

impl SourceOverride {
    fn read(path: &Path) -> Result<Self, String> {
        let canonical = fs::canonicalize(path)
            .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
        Ok(Self {
            canonical,
            input: read_stdin()?,
        })
    }
}

#[derive(Clone, Debug)]
struct WatchOptions {
    source: PathBuf,
    output: PathBuf,
    once: bool,
    poll_interval: Duration,
    source_maps: bool,
    app: Option<AppOptions>,
    emit_options: js_backend::EmitOptions,
}

#[derive(Clone, Debug)]
struct BuildOptions {
    source_maps: bool,
    app: Option<AppOptions>,
    emit_options: js_backend::EmitOptions,
}

#[derive(Clone, Debug)]
struct BuildArtifact {
    kind: String,
    source: PathBuf,
    output: PathBuf,
    source_map: Option<PathBuf>,
    bytes: u64,
    runtime_effects: BTreeSet<String>,
    runtime_exports: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AppOptions {
    root_id: String,
    css: Option<String>,
    vendor_runtime: bool,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            root_id: "root".to_string(),
            css: None,
            vendor_runtime: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WatchStamp {
    modified: Option<SystemTime>,
    len: Option<u64>,
}

#[derive(Clone, Debug)]
struct CacheProbe {
    key: String,
    module_count: usize,
    cache_file: PathBuf,
}

#[derive(Clone, Debug)]
struct CachedCheck {
    ok: bool,
    diagnostics_json: String,
}

#[derive(Clone, Debug)]
struct CachedInspect {
    report_json: String,
}

#[derive(Clone, Debug)]
struct ModuleFingerprint {
    canonical: PathBuf,
    source_hash: String,
    imports: Vec<String>,
}

fn require_path(args: &[String]) -> Result<PathBuf, String> {
    args.get(1)
        .map(PathBuf::from)
        .ok_or_else(|| "expected a source file path".to_string())
}

fn require_check_path(args: &[String]) -> Result<PathBuf, String> {
    require_flaggable_path(args, "check")
}

fn require_inspect_path(args: &[String]) -> Result<PathBuf, String> {
    require_flaggable_path(args, "inspect")
}

fn require_flaggable_path(args: &[String], command: &str) -> Result<PathBuf, String> {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| format!("{} expects a source file path", command))
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

fn parse_app_options(args: &[String]) -> Result<Option<AppOptions>, String> {
    let mut app = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--app" => {
                app = Some(app.unwrap_or_default());
                index += 1;
            }
            "--root" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--root expects an element id".to_string());
                };
                let options = app.get_or_insert_with(AppOptions::default);
                options.root_id = value.clone();
                index += 2;
            }
            "--css" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--css expects an import path".to_string());
                };
                let options = app.get_or_insert_with(AppOptions::default);
                options.css = Some(value.clone());
                index += 2;
            }
            "--vendor-runtime" => {
                let options = app.get_or_insert_with(AppOptions::default);
                options.vendor_runtime = true;
                index += 1;
            }
            _ => index += 1,
        }
    }
    Ok(app)
}

fn emit_options_from_env() -> js_backend::EmitOptions {
    js_backend::EmitOptions {
        reachable_message_kinds: None,
        message_field_reads: BTreeMap::new(),
        static_reads: BTreeMap::new(),
        direct_call_replacements: BTreeMap::new(),
        prelude_code: String::new(),
        browser_app_runtime: false,
    }
}

fn run_module_tests(path: &Path, json: bool) -> Result<(), String> {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let temp_dir = env::temp_dir().join(format!("closkell-test-{}-{}", std::process::id(), suffix));

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("failed to create {}: {}", temp_dir.display(), err))?;

    let result = run_module_tests_in_temp(path, &temp_dir, json);
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

fn run_module_tests_in_temp(path: &Path, temp_dir: &Path, json: bool) -> Result<(), String> {
    let mut modules = HashMap::new();
    let mut checking = HashSet::new();
    check_file(path, &mut modules, &mut checking, false)?;

    copy_runtime_package(temp_dir)?;
    let output = temp_dir.join("__closkell_test_entry.mjs");
    let mut visited = HashSet::new();
    let mut artifacts = Vec::new();
    build_file(
        path,
        &output,
        &mut visited,
        &BuildOptions {
            source_maps: false,
            app: None,
            emit_options: emit_options_from_env(),
        },
        &modules,
        &mut artifacts,
        "entry",
    )?;
    run_node_test_module(&output, json)
}

fn run_node_test_module(output: &Path, json: bool) -> Result<(), String> {
    let run = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(if json {
            test_runner_json_script(output)
        } else {
            test_runner_script(output)
        })
        .output()
        .map_err(|err| format!("node is required for `closkell test`: {}", err))?;

    print!("{}", String::from_utf8_lossy(&run.stdout));
    eprint!("{}", String::from_utf8_lossy(&run.stderr));

    if run.status.success() {
        Ok(())
    } else {
        Err("module tests failed".to_string())
    }
}

fn test_runner_script(output: &Path) -> String {
    let module_path = json_string(&output.display().to_string());
    format!(
        r#"import {{ dirname, join }} from "node:path";
import {{ pathToFileURL }} from "node:url";

const modulePath = {module_path};
const moduleUrl = pathToFileURL(modulePath).href;
const runtimeUrl = pathToFileURL(join(dirname(modulePath), "node_modules", "@closkell", "runtime", "src", "index.js")).href;
const {{ render: __closkellPrepareDocument }} = await import(runtimeUrl);
__closkellPrepareDocument(null);
const module = await import(moduleUrl);
{shared}

const collected = collectModuleTests(module);
if (collected.error) {{
  console.error(collected.error);
  process.exit(2);
}}

const tests = collected.tests;
if (tests.length === 0) {{
  console.error("expected module `tests` to contain at least one test");
  process.exit(2);
}}

let failed = 0;
for (const [index, test] of tests.entries()) {{
  const result = runTest(test, index);
  if (result.ok) {{
    console.log("ok " + (index + 1) + " - " + result.name);
  }} else {{
    failed += 1;
    console.error("not ok " + (index + 1) + " - " + result.name);
    if (result.error) console.error("  " + result.error);
    if ("expected" in result) console.error("  expected " + result.expected);
    if ("actual" in result) console.error("  actual   " + result.actual);
  }}
}}

if (failed > 0) {{
  console.error(failed + "/" + tests.length + " tests failed");
  process.exit(1);
}}

console.log("ok " + tests.length + " tests");
"#,
        module_path = module_path,
        shared = test_runner_shared_script()
    )
}

fn test_runner_json_script(output: &Path) -> String {
    let module_path = json_string(&output.display().to_string());
    format!(
        r#"import {{ dirname, join }} from "node:path";
import {{ pathToFileURL }} from "node:url";

const modulePath = {module_path};
const moduleUrl = pathToFileURL(modulePath).href;
const runtimeUrl = pathToFileURL(join(dirname(modulePath), "node_modules", "@closkell", "runtime", "src", "index.js")).href;
const {{ render: __closkellPrepareDocument }} = await import(runtimeUrl);
__closkellPrepareDocument(null);
const module = await import(moduleUrl);
{shared}

function emit(report, status) {{
  console.log(JSON.stringify({{ file: modulePath, ...report }}));
  process.exit(status);
}}

const collected = collectModuleTests(module);
if (collected.error) {{
  emit({{
    ok: false,
    count: 0,
    passed: 0,
    failed: 1,
    error: collected.error,
    tests: []
  }}, 2);
}}

const tests = collected.tests;
if (tests.length === 0) {{
  emit({{
    ok: false,
    count: 0,
    passed: 0,
    failed: 1,
    error: "expected module `tests` to contain at least one test",
    tests: []
  }}, 2);
}}

const results = [];
let failed = 0;
for (const [index, test] of tests.entries()) {{
  const result = runTest(test, index);
  if (!result.ok) {{
    failed += 1;
  }}
  results.push(result);
}}

emit({{
  ok: failed === 0,
  count: tests.length,
  passed: tests.length - failed,
  failed,
  tests: results
}}, failed > 0 ? 1 : 0);
"#,
        module_path = module_path,
        shared = test_runner_shared_script()
    )
}

fn test_runner_shared_script() -> &'static str {
    r#"function symbolKey(value) {
  if (typeof value !== "symbol") return null;
  return Symbol.keyFor(value) ?? value.description ?? "";
}

function isObject(value) {
  return value !== null && typeof value === "object";
}

function deepEqual(left, right) {
  if (Object.is(left, right)) return true;
  if (typeof left === "symbol" || typeof right === "symbol") {
    return symbolKey(left) === symbolKey(right);
  }
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right)) return false;
    if (left.length !== right.length) return false;
    return left.every((value, index) => deepEqual(value, right[index]));
  }
  if (isObject(left) || isObject(right)) {
    if (!isObject(left) || !isObject(right)) return false;
    const leftKeys = Object.keys(left).sort();
    const rightKeys = Object.keys(right).sort();
    if (!deepEqual(leftKeys, rightKeys)) return false;
    return leftKeys.every((key) => deepEqual(left[key], right[key]));
  }
  return false;
}

function deepMatch(actual, pattern) {
  if (typeof pattern === "function") return pattern(actual) === true;
  if (Object.is(actual, pattern)) return true;
  if (typeof actual === "symbol" || typeof pattern === "symbol") {
    return symbolKey(actual) === symbolKey(pattern);
  }
  if (Array.isArray(pattern)) {
    if (!Array.isArray(actual) || actual.length !== pattern.length) return false;
    return pattern.every((value, index) => deepMatch(actual[index], value));
  }
  if (pattern instanceof Set) {
    if (!(actual instanceof Set)) return false;
    return Array.from(pattern).every((patternItem) =>
      Array.from(actual).some((actualItem) => deepEqual(actualItem, patternItem))
    );
  }
  if (pattern instanceof Map) {
    if (!(actual instanceof Map)) return false;
    return Array.from(pattern).every(([key, value]) =>
      actual.has(key) && deepMatch(actual.get(key), value)
    );
  }
  if (isObject(pattern)) {
    if (!isObject(actual)) return false;
    return Object.keys(pattern).every((key) => deepMatch(actual[key], pattern[key]));
  }
  return false;
}

function thrownMessage(error) {
  if (typeof error === "string") return error;
  return error?.message ?? String(error);
}

function formatValue(value) {
  if (typeof value === "symbol") return ":" + symbolKey(value);
  if (typeof value === "undefined") return "undefined";
  return JSON.stringify(
    value,
    (_key, next) => (typeof next === "symbol" ? ":" + symbolKey(next) : next)
  );
}

function testName(test, index) {
  if (test && typeof test.name === "string" && test.name.length > 0) {
    return test.name;
  }
  return "test " + (index + 1);
}

function fullTestName(prefix, name) {
  return [...prefix, name].filter((part) => part && part.length > 0).join(" / ");
}

function collectModuleTests(module) {
  if ("tests" in module) {
    return { tests: flattenTestEntries(module.tests, [], true) };
  }
  const exportedTests = Object.keys(module)
    .sort()
    .flatMap((name) => flattenTestEntries(module[name], [], false));
  if (exportedTests.length === 0) {
    return {
      error: "expected module to export `tests` or top-level describe/test forms",
      tests: []
    };
  }
  return { tests: exportedTests };
}

function flattenTestEntries(value, prefix = [], strict = true) {
  if (value == null || value === false) return [];
  if (Array.isArray(value)) {
    return value.flatMap((entry) => flattenTestEntries(entry, prefix, strict));
  }
  if (isObject(value) && value.__closkellTestGroup) {
    const nextPrefix = [...prefix, String(value.name ?? "")];
    return flattenTestEntries(value.tests ?? [], nextPrefix, strict);
  }
  if (isObject(value) && value.__closkellTest) {
    return [{ ...value, name: fullTestName(prefix, testName(value, 0)) }];
  }
  if (isObject(value) && ("actual" in value || "expected" in value)) {
    const name = value.name ? fullTestName(prefix, String(value.name)) : fullTestName(prefix, "");
    return [{ ...value, name }];
  }
  return strict ? [value] : [];
}

function assertionKind(assertion) {
  const kind = assertion?.__closkellAssert;
  if (typeof kind === "symbol") return symbolKey(kind);
  return String(kind ?? "");
}

function testAssertions(test) {
  if (test && test.__closkellTest) return test.assertions ?? [];
  return [test];
}

function runTest(test, index) {
  const name = testName(test, index);
  if (!test || typeof test !== "object") {
    return { name, ok: false, error: "expected a test record or closkell/test value" };
  }

  const assertions = testAssertions(test);
  if (assertions.length === 0) {
    return { name, ok: false, error: "expected at least one assertion" };
  }

  for (const assertion of assertions) {
    const result = runAssertion(assertion);
    if (!result.ok) return { name, ...result };
  }
  return { name, ok: true };
}

function runAssertion(assertion) {
  if (!assertion || typeof assertion !== "object") {
    return { ok: false, error: "expected an assertion record" };
  }
  const kind = assertionKind(assertion);
  if (!kind && ("actual" in assertion || "expected" in assertion)) {
    return runEqualAssertion(assertion.actual, assertion.expected, false);
  }
  switch (kind) {
    case "equal":
      return runEqualAssertion(assertion.actual, assertion.expected, false);
    case "not-equal":
      return runEqualAssertion(assertion.actual, assertion.expected, true);
    case "ok":
      return assertion.actual === true
        ? { ok: true }
        : { ok: false, expected: "true", actual: formatValue(assertion.actual) };
    case "err":
      return assertion.actual?.ok === false
        ? { ok: true }
        : { ok: false, expected: "err", actual: formatValue(assertion.actual) };
    case "some":
      return assertion.actual != null
        ? { ok: true }
        : { ok: false, expected: "some value", actual: formatValue(assertion.actual) };
    case "nil":
      return assertion.actual == null
        ? { ok: true }
        : { ok: false, expected: "nil", actual: formatValue(assertion.actual) };
    case "match":
      return runMatchAssertion(assertion.actual, assertion.pattern);
    case "throws":
      return runThrowsAssertion(assertion.thunk, assertion.expected);
    default:
      return { ok: false, error: "unknown assertion kind `" + kind + "`" };
  }
}

function runEqualAssertion(actual, expected, negated) {
  const equal = deepEqual(actual, expected);
  if (negated ? !equal : equal) return { ok: true };
  return {
    ok: false,
    expected: negated ? "not " + formatValue(expected) : formatValue(expected),
    actual: formatValue(actual)
  };
}

function runMatchAssertion(actual, pattern) {
  if (deepMatch(actual, pattern)) return { ok: true };
  return {
    ok: false,
    expected: "match " + formatValue(pattern),
    actual: formatValue(actual)
  };
}

function runThrowsAssertion(thunk, expected) {
  if (typeof thunk !== "function") {
    return { ok: false, expected: "function that throws", actual: formatValue(thunk) };
  }
  try {
    thunk();
  } catch (error) {
    const message = thrownMessage(error);
    if (expected === undefined || String(message).includes(String(expected))) return { ok: true };
    return {
      ok: false,
      expected: "throw containing " + formatValue(expected),
      actual: formatValue(message)
    };
  }
  return { ok: false, expected: "throw", actual: "no throw" };
}
"#
}

fn copy_runtime_package(temp_dir: &Path) -> Result<(), String> {
    copy_runtime_package_with_exports(temp_dir, None)
}

fn copy_runtime_package_with_exports(
    temp_dir: &Path,
    required_exports: Option<&BTreeSet<String>>,
) -> Result<(), String> {
    let package_dir = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime");
    let source_dir = workspace_root().join("runtime-js");
    fs::create_dir_all(package_dir.join("src")).map_err(|err| {
        format!(
            "failed to create runtime package directory {}: {}",
            package_dir.display(),
            err
        )
    })?;
    copy_runtime_file(
        &source_dir.join("package.json"),
        &package_dir.join("package.json"),
    )?;
    let source_entry = source_dir.join("src").join("index.js");
    let target_entry = package_dir.join("src").join("index.js");
    if let Some(required_exports) = required_exports {
        let source = fs::read_to_string(&source_entry)
            .map_err(|err| format!("failed to read {}: {}", source_entry.display(), err))?;
        let tailored = tailored_runtime_source(&source, required_exports)?;
        fs::write(&target_entry, tailored)
            .map_err(|err| format!("failed to write {}: {}", target_entry.display(), err))?;
        Ok(())
    } else {
        copy_runtime_file(&source_entry, &target_entry)
    }
}

fn copy_runtime_file(source: &Path, target: &Path) -> Result<(), String> {
    fs::copy(source, target).map(|_| ()).map_err(|err| {
        format!(
            "failed to copy runtime file {} -> {}: {}",
            source.display(),
            target.display(),
            err
        )
    })
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn run_dev_watch(args: &[String]) -> Result<(), String> {
    let options = parse_watch_options(args)?;
    println!(
        "watching {} -> {}",
        options.source.display(),
        options.output.display()
    );

    let mut last_snapshot = None;
    loop {
        let snapshot = watch_snapshot(&options.source);
        let changed = last_snapshot.as_ref() != Some(&snapshot);

        if changed {
            let result = dev_build_once_with_options(&options.source, &options.output, &options);
            match result {
                Ok(()) => {
                    println!(
                        "built {} -> {}",
                        options.source.display(),
                        options.output.display()
                    );
                    last_snapshot = Some(snapshot);
                    if options.once {
                        return Ok(());
                    }
                }
                Err(error) => {
                    eprintln!("build failed: {}", error);
                    last_snapshot = Some(snapshot);
                    if options.once {
                        return Err(error);
                    }
                }
            }
        }

        if options.once {
            return Ok(());
        }
        thread::sleep(options.poll_interval);
    }
}

fn parse_watch_options(args: &[String]) -> Result<WatchOptions, String> {
    let mut source = None;
    let mut output = None;
    let mut once = false;
    let mut poll_ms = 250_u64;
    let mut source_maps = false;
    let mut app = None;
    let mut index = 1;

    while index < args.len() {
        match args[index].as_str() {
            "--watch" => index += 1,
            "--once" => {
                once = true;
                index += 1;
            }
            "-o" | "--out" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(format!("{} expects a path", args[index]));
                };
                output = Some(PathBuf::from(value));
                index += 2;
            }
            "--poll-ms" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--poll-ms expects a number".to_string());
                };
                poll_ms = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid --poll-ms value `{}`", value))?;
                index += 2;
            }
            "--sourcemap" | "--source-map" => {
                source_maps = true;
                index += 1;
            }
            "--app" => {
                app = Some(app.unwrap_or_default());
                index += 1;
            }
            "--root" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--root expects an element id".to_string());
                };
                let options = app.get_or_insert_with(AppOptions::default);
                options.root_id = value.clone();
                index += 2;
            }
            "--css" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--css expects an import path".to_string());
                };
                let options = app.get_or_insert_with(AppOptions::default);
                options.css = Some(value.clone());
                index += 2;
            }
            "--vendor-runtime" => {
                let options = app.get_or_insert_with(AppOptions::default);
                options.vendor_runtime = true;
                index += 1;
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown dev --watch option `{}`", value));
            }
            value => {
                if source.is_some() {
                    return Err(format!("unexpected dev --watch argument `{}`", value));
                }
                source = Some(PathBuf::from(value));
                index += 1;
            }
        }
    }

    let source =
        source.ok_or_else(|| "expected a source file path after dev --watch".to_string())?;
    let output = output.unwrap_or_else(|| source.with_extension("mjs"));
    Ok(WatchOptions {
        source,
        output,
        once,
        poll_interval: Duration::from_millis(poll_ms.max(1)),
        source_maps,
        app,
        emit_options: emit_options_from_env(),
    })
}

fn dev_build_once_with_options(
    source: &Path,
    output: &Path,
    options: &WatchOptions,
) -> Result<(), String> {
    let mut modules = HashMap::new();
    let mut checking = HashSet::new();
    let module = check_file(source, &mut modules, &mut checking, false)?;
    if options.app.is_some() {
        require_app_exports(source, &module.exports)?;
    }

    let mut visited = HashSet::new();
    let build_options = BuildOptions {
        source_maps: options.source_maps,
        app: options.app.clone(),
        emit_options: options.emit_options.clone(),
    };
    let mut artifacts = Vec::new();
    build_file(
        source,
        output,
        &mut visited,
        &build_options,
        &modules,
        &mut artifacts,
        "entry",
    )
}

fn watch_snapshot(source: &Path) -> BTreeMap<PathBuf, WatchStamp> {
    let mut paths = BTreeSet::new();
    let _ = collect_source_paths(source, &mut paths);
    if paths.is_empty() {
        paths.insert(source.to_path_buf());
    }

    paths
        .into_iter()
        .map(|path| {
            let metadata = fs::metadata(&path);
            let stamp = match metadata {
                Ok(metadata) => WatchStamp {
                    modified: metadata.modified().ok(),
                    len: Some(metadata.len()),
                },
                Err(_) => WatchStamp {
                    modified: None,
                    len: None,
                },
            };
            (path, stamp)
        })
        .collect()
}

fn collect_source_paths(source: &Path, paths: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    let canonical = fs::canonicalize(source)
        .map_err(|err| format!("failed to resolve {}: {}", source.display(), err))?;
    if !paths.insert(canonical.clone()) {
        return Ok(());
    }

    let (input, parsed) = parse_file(&canonical)?;
    if parsed.has_errors() {
        return Ok(());
    }
    for import in parse_imports(&input, &parsed)? {
        if !is_closkell_import_path(&import.path) {
            continue;
        }
        let import_source = resolve_import_source(&canonical, &import.path)?;
        collect_source_paths(&import_source, paths)?;
    }
    Ok(())
}

fn output_path(args: &[String]) -> Option<PathBuf> {
    args.windows(2)
        .find_map(|pair| (pair[0] == "-o" || pair[0] == "--out").then(|| PathBuf::from(&pair[1])))
}

fn check_file(
    path: &Path,
    modules: &mut HashMap<PathBuf, ModuleInfo>,
    checking: &mut HashSet<PathBuf>,
    print_forms: bool,
) -> Result<ModuleInfo, String> {
    let mut reporter = CheckReporter::new(true);
    check_file_with_reporter(path, modules, checking, print_forms, &mut reporter, None)
}

fn check_file_with_reporter(
    path: &Path,
    modules: &mut HashMap<PathBuf, ModuleInfo>,
    checking: &mut HashSet<PathBuf>,
    print_forms: bool,
    reporter: &mut CheckReporter,
    source_override: Option<&SourceOverride>,
) -> Result<ModuleInfo, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    if let Some(info) = modules.get(&canonical) {
        return Ok(info.clone());
    }
    if !checking.insert(canonical.clone()) {
        return Err(format!("cyclic import while checking {}", path.display()));
    }

    let result = check_file_inner(
        path,
        modules,
        checking,
        print_forms,
        reporter,
        source_override,
    );
    checking.remove(&canonical);
    if let Ok(info) = &result {
        modules.insert(canonical, info.clone());
    }
    result
}

fn check_file_inner(
    path: &Path,
    modules: &mut HashMap<PathBuf, ModuleInfo>,
    checking: &mut HashSet<PathBuf>,
    print_forms: bool,
    reporter: &mut CheckReporter,
    source_override: Option<&SourceOverride>,
) -> Result<ModuleInfo, String> {
    let (input, source) = parse_check_file(path, source_override)?;
    reporter.report(path, &input, &source.diagnostics);
    if source.has_errors() {
        return Err(format!("check failed during parsing: {}", path.display()));
    }

    let imports = parse_imports(&input, &source)?;
    let mut import_diagnostics = Vec::new();
    let mut import_bindings = Vec::new();
    let mut import_type_declarations = Vec::new();
    let mut imported_macros = HashMap::new();
    let mut imported_command_shapes = HashMap::new();
    let mut imported_message_kinds = HashMap::new();
    for import in &imports {
        if !is_closkell_import_path(&import.path) {
            continue;
        }
        let import_source = resolve_import_source(path, &import.path)?;
        let imported = check_file_with_reporter(
            &import_source,
            modules,
            checking,
            false,
            reporter,
            source_override,
        )?;
        for name in &import.names {
            if !imported.exports.contains(&name.imported) {
                import_diagnostics.push(Diagnostic::error(
                    name.span,
                    format!(
                        "import `{}` is not exported by {}",
                        name.imported, import.path
                    ),
                ));
                continue;
            }
            if let Some(binding) = imported.bindings.iter().find(|binding| {
                binding.name == name.imported
                    && imported_binding_type_is_importable(&imported, binding)
            }) {
                let binding = binding.import_as(name.name.clone());
                if binding.returns_cmd() {
                    if let Some(shapes) = imported.command_shapes_by_binding.get(&name.imported) {
                        imported_command_shapes.insert(name.name.clone(), shapes.clone());
                    }
                }
                import_bindings.push(binding);
            }
            if let Some(kinds) = imported.message_kinds_by_binding.get(&name.imported) {
                imported_message_kinds.insert(name.name.clone(), kinds.clone());
            }
            if let Some(declaration) = imported
                .type_declarations
                .iter()
                .find(|declaration| declaration.name == name.imported)
            {
                import_type_declarations.push(declaration.import_as(name.name.clone()));
            }
            if let Some(macro_def) = imported.macros.get(&name.imported) {
                imported_macros.insert(name.name.clone(), macro_def.clone());
            }
        }
    }
    reporter.report(path, &input, &import_diagnostics);

    let local_macros = macro_expand::collect_macro_defs(&source).macros;
    let expansion = macro_expand::expand_source_with_imported_macros(&source, &imported_macros);
    reporter.report(path, &input, &expansion.diagnostics);

    let type_result = typecheck::check_source_with_module_imports(
        &expansion.source,
        &import_bindings,
        &import_type_declarations,
    );
    reporter.report(path, &input, &type_result.diagnostics);

    let imported_command_helpers = import_bindings
        .iter()
        .filter(|binding| binding.returns_cmd())
        .map(|binding| binding.name.clone())
        .collect::<HashSet<_>>();
    let effect_report = effects::validate_purity_with_imported_command_helpers(
        &expansion.source,
        &imported_command_helpers,
    );
    reporter.report(path, &input, &effect_report.diagnostics);

    if print_forms {
        for form in type_result.forms {
            println!("{} : {}", form.source, form.ty);
        }
        let templates = template_ir::lower_named_templates(&expansion.source);
        if !templates.is_empty() {
            println!("templates: {}", templates.len());
        }
    }

    if !import_diagnostics.is_empty()
        || !expansion.diagnostics.is_empty()
        || !type_result.diagnostics.is_empty()
        || !effect_report.diagnostics.is_empty()
    {
        return Err(format!("check failed: {}", path.display()));
    }

    let mut exports = collect_exports(&expansion.source);
    exports.extend(local_macros.keys().cloned());
    let command_binding_names = type_result
        .bindings
        .iter()
        .filter(|binding| binding.import_as(binding.name.clone()).returns_cmd())
        .map(|binding| binding.name.clone())
        .collect::<HashSet<_>>();
    let command_shapes_by_binding = collect_command_shapes_by_binding(
        &expansion.source,
        &command_binding_names,
        &imported_command_shapes,
    );
    let message_kinds_by_binding =
        collect_message_kinds_by_binding(&expansion.source, &imported_message_kinds);

    Ok(ModuleInfo {
        input,
        exports,
        bindings: type_result.bindings,
        type_declarations: type_result.type_declarations,
        macros: local_macros,
        command_shapes_by_binding,
        message_kinds_by_binding,
        source: expansion.source,
        expr_types: type_result.expr_types,
    })
}

#[derive(Clone, Debug)]
struct RuntimeChunk {
    name: String,
    code: String,
}

fn collect_runtime_import_exports(code: &str) -> BTreeSet<String> {
    let mut exports = BTreeSet::new();
    let runtime_from = "from \"@closkell/runtime\"";
    let mut search_start = 0;
    while let Some(relative_from) = code[search_start..].find(runtime_from) {
        let from_index = search_start + relative_from;
        let Some(import_index) = code[..from_index].rfind("import") else {
            search_start = from_index + runtime_from.len();
            continue;
        };
        let import_clause = &code[import_index..from_index];
        let Some(open_brace) = import_clause.find('{') else {
            search_start = from_index + runtime_from.len();
            continue;
        };
        let Some(close_brace) = import_clause.rfind('}') else {
            search_start = from_index + runtime_from.len();
            continue;
        };
        if close_brace <= open_brace {
            search_start = from_index + runtime_from.len();
            continue;
        }
        for part in import_clause[open_brace + 1..close_brace].split(',') {
            let name = part.trim().split_whitespace().next().unwrap_or("");
            if is_js_identifier(name) {
                exports.insert(name.to_string());
            }
        }
        search_start = from_index + runtime_from.len();
    }
    exports
}

fn tailored_runtime_source(
    runtime_source: &str,
    required_exports: &BTreeSet<String>,
) -> Result<String, String> {
    if required_exports.is_empty() {
        return Ok("\n".to_string());
    }
    let chunks = runtime_chunks(runtime_source);
    let chunk_by_name = chunks
        .iter()
        .map(|chunk| (chunk.name.clone(), chunk))
        .collect::<HashMap<_, _>>();
    let declarations = chunk_by_name.keys().cloned().collect::<BTreeSet<_>>();

    let mut included = BTreeSet::new();
    let mut pending = required_exports.iter().cloned().collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        if !included.insert(name.clone()) {
            continue;
        }
        let Some(chunk) = chunk_by_name.get(&name) else {
            return Err(format!(
                "runtime export `{}` was requested but no declaration was found",
                name
            ));
        };
        let mut dependencies = runtime_chunk_referenced_dependencies(chunk, &declarations);
        dependencies.extend(
            runtime_chunk_declared_dependencies(&chunk.name)
                .iter()
                .map(|dependency| (*dependency).to_string()),
        );
        for dependency in dependencies {
            if !declarations.contains(&dependency) {
                return Err(format!(
                    "runtime chunk `{}` depends on missing declaration `{}`",
                    chunk.name, dependency
                ));
            }
            if !included.contains(&dependency) {
                pending.push(dependency);
            }
        }
    }

    let mut output = String::new();
    for chunk in chunks {
        if !included.contains(&chunk.name) {
            continue;
        }
        output.push_str(&chunk.code);
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }
    Ok(output)
}

fn runtime_chunk_referenced_dependencies(
    chunk: &RuntimeChunk,
    declarations: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    let code = strip_js_strings_and_comments(&chunk.code);
    let local_bindings = runtime_chunk_local_bindings(&code);
    let mut chars = code.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if !is_js_identifier_start(ch) {
            continue;
        }

        let mut end = start + ch.len_utf8();
        while let Some((next_start, next)) = chars.peek().copied() {
            if !is_js_identifier_continue(next) {
                break;
            }
            chars.next();
            end = next_start + next.len_utf8();
        }

        let name = &code[start..end];
        if name == chunk.name {
            continue;
        }
        if local_bindings.contains(name) {
            continue;
        }
        if declarations.contains(name)
            && previous_non_ws(&code, start) != Some('.')
            && next_non_ws(&code, end) != Some(':')
        {
            dependencies.insert(name.to_string());
        }
    }
    dependencies
}

fn runtime_chunk_local_bindings(code: &str) -> BTreeSet<String> {
    let mut bindings = BTreeSet::new();
    let mut cursor = 0;
    while cursor < code.len() {
        let Some((start, word, end)) = next_js_identifier(code, cursor) else {
            break;
        };
        match word {
            "const" | "let" | "var" => collect_js_declaration_bindings(code, end, &mut bindings),
            "function" => {
                if let Some((_, name, name_end)) = next_js_identifier(code, end) {
                    bindings.insert(name.to_string());
                    collect_js_function_params(code, name_end, &mut bindings);
                }
            }
            "class" => {
                if let Some((_, name, _)) = next_js_identifier(code, end) {
                    bindings.insert(name.to_string());
                }
            }
            _ => {}
        }
        cursor = start + word.len();
        if cursor <= start {
            cursor = end;
        }
    }
    bindings
}

fn collect_js_declaration_bindings(code: &str, start: usize, bindings: &mut BTreeSet<String>) {
    let mut index = start;
    let mut expect_binding = true;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    while index < code.len() {
        let Some((offset, ch)) = code[index..].char_indices().next() else {
            break;
        };
        let pos = index + offset;
        if expect_binding {
            if ch.is_ascii_whitespace() || ch == ',' {
                index = pos + ch.len_utf8();
                continue;
            }
            if ch == '{' || ch == '[' {
                if let Some(end) = matching_js_delimiter(code, pos, ch) {
                    collect_js_pattern_bindings(&code[pos + 1..end], bindings);
                    index = end + 1;
                    expect_binding = false;
                    continue;
                }
            }
            if is_js_identifier_start(ch) {
                let (_, name, end) = read_js_identifier(code, pos);
                bindings.insert(name.to_string());
                index = end;
                expect_binding = false;
                continue;
            }
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                expect_binding = true
            }
            ';' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => break,
            _ => {}
        }
        index = pos + ch.len_utf8();
    }
}

fn collect_js_function_params(code: &str, after_name: usize, bindings: &mut BTreeSet<String>) {
    let Some(open) = code[after_name..]
        .find('(')
        .map(|offset| after_name + offset)
    else {
        return;
    };
    let Some(close) = matching_js_delimiter(code, open, '(') else {
        return;
    };
    collect_js_pattern_bindings(&code[open + 1..close], bindings);
}

fn collect_js_pattern_bindings(pattern: &str, bindings: &mut BTreeSet<String>) {
    let mut cursor = 0;
    while let Some((_, name, end)) = next_js_identifier(pattern, cursor) {
        if !is_js_reserved_word(name) {
            bindings.insert(name.to_string());
        }
        cursor = end;
    }
}

fn matching_js_delimiter(code: &str, open: usize, delimiter: char) -> Option<usize> {
    let close = match delimiter {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => return None,
    };
    let mut depth = 0usize;
    for (offset, ch) in code[open..].char_indices() {
        let pos = open + offset;
        if ch == delimiter {
            depth += 1;
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(pos);
            }
        }
    }
    None
}

fn next_js_identifier(source: &str, start: usize) -> Option<(usize, &str, usize)> {
    let mut cursor = start;
    while cursor < source.len() {
        let Some((offset, ch)) = source[cursor..].char_indices().next() else {
            break;
        };
        let pos = cursor + offset;
        if is_js_identifier_start(ch) {
            let (start, name, end) = read_js_identifier(source, pos);
            return Some((start, name, end));
        }
        cursor = pos + ch.len_utf8();
    }
    None
}

fn read_js_identifier(source: &str, start: usize) -> (usize, &str, usize) {
    let mut end = start;
    for (offset, ch) in source[start..].char_indices() {
        let pos = start + offset;
        if offset == 0 {
            end = pos + ch.len_utf8();
            continue;
        }
        if !is_js_identifier_continue(ch) {
            return (start, &source[start..pos], pos);
        }
        end = pos + ch.len_utf8();
    }
    (start, &source[start..end], end)
}

fn is_js_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "as" | "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "from"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "let"
            | "new"
            | "null"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "undefined"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

fn strip_js_strings_and_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' | '\'' | '`' => {
                let quote = ch;
                output.push(' ');
                let mut escaped = false;
                for next in chars.by_ref() {
                    output.push(if next == '\n' { '\n' } else { ' ' });
                    if escaped {
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == quote {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                output.push(' ');
                output.push(' ');
                chars.next();
                for next in chars.by_ref() {
                    if next == '\n' {
                        output.push('\n');
                        break;
                    }
                    output.push(' ');
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                output.push(' ');
                output.push(' ');
                chars.next();
                let mut previous = '\0';
                for next in chars.by_ref() {
                    output.push(if next == '\n' { '\n' } else { ' ' });
                    if previous == '*' && next == '/' {
                        break;
                    }
                    previous = next;
                }
            }
            _ => output.push(ch),
        }
    }
    output
}

fn previous_non_ws(source: &str, before: usize) -> Option<char> {
    source[..before]
        .chars()
        .rev()
        .find(|ch| !ch.is_ascii_whitespace())
}

fn next_non_ws(source: &str, after: usize) -> Option<char> {
    source[after..].chars().find(|ch| !ch.is_ascii_whitespace())
}

fn runtime_chunk_declared_dependencies(name: &str) -> &'static [&'static str] {
    match name {
        "CloskellTestTextNode" => &["siblingForNode"],
        "CloskellTestElement" => &[
            "CloskellTestStyle",
            "CloskellTestTextNode",
            "createTestEvent",
            "parseTestHtmlFragment",
            "querySelectorAllFrom",
            "selectorMatchesNode",
            "serializeTestNode",
            "siblingForNode",
            "style",
        ],
        "CloskellTestDocumentFragment" => &["CloskellTestElement"],
        "parseTestHtmlFragment" => &[
            "CloskellTestDocumentFragment",
            "CloskellTestTextNode",
            "createRuntimeTextNode",
            "decodeTestHtml",
            "isTestVoidElement",
            "parseTestHtmlAttrs",
        ],
        "createRuntimeTextNode" => &["CloskellTestTextNode"],
        "isTestVoidElement" => &[],
        "ensureRuntimeDocument" => &[
            "CloskellTestDocumentFragment",
            "CloskellTestElement",
            "CloskellTestTextNode",
        ],
        "htmlTemplate" => &["ensureRuntimeDocument"],
        "createDevtoolsOverlay" => &[
            "createDevtoolsOverlayRoot",
            "normalizeDevtoolsOverlayOptions",
            "positiveNumber",
            "renderDevtoolsOverlay",
        ],
        "createDevtoolsOverlayRoot" => &["style"],
        "renderDevtoolsOverlay" => &[
            "clearNode",
            "createDetachedOverlayRow",
            "devtoolsEventSummary",
            "style",
        ],
        "createDetachedOverlayRow" => &["style"],
        "createTemplateComponent" => &[
            "beginTemplateUpdate",
            "claimHydratedTemplateInstance",
            "disposeComponent",
            "disposeEventSlots",
            "disposeRefs",
            "emitDispatchDevtools",
            "endTemplateUpdate",
            "reportTemplateMount",
        ],
        "createCompiledTemplateComponent" => {
            &["disposeComponent", "disposeEventSlots", "disposeRefs"]
        }
        "createCompiledHtmlTemplateComponent" => &[
            "compiledHtmlTemplateNode",
            "compiledHtmlTemplateShape",
            "disposeComponent",
            "disposeEventSlots",
            "disposeRefs",
        ],
        "createCompiledHtmlTemplateFactory" => &[],
        "compiledHtmlTemplateShape" => &[
            "compiledHtmlTemplatePath",
            "compiledHtmlTemplateShapes",
            "createCompiledHtmlTemplateFactory",
        ],
        "bindCompiledComponent" => &[],
        "shouldUpdateSlot" => &["recordTemplateSlot", "shouldUpdateSlotForReads"],
        "shouldUpdateCompiledSlot" => &["shouldUpdateSlotForReads"],
        "claimHydratedTemplateInstance" => &["claimHydratedTree"],
        "claimHydratedTree" => &["hydrationNodesCompatible"],
        "shouldUpdateSlotForReads" => &[
            "changedPathsForUpdate",
            "isLocalReadPath",
            "isStatePath",
            "pathsOverlap",
        ],
        "endTemplateUpdate" => &["emitDevtools"],
        "reportTemplateMount" => &["emitDispatchDevtools"],
        "setAttr" => &[
            "applyClassValue",
            "applyStyleObject",
            "clearStyleAttribute",
            "clearStyleObject",
            "isStructuredClassValue",
            "isStyleObject",
            "setDomProperty",
            "style",
        ],
        "setCompiledAttr" => &["setDomProperty"],
        "setCompiledClass" => &["applyClassValue", "isStructuredClassValue"],
        "setCompiledStyle" => &[
            "applyStyleObject",
            "clearStyleAttribute",
            "clearStyleObject",
            "isStyleObject",
            "style",
        ],
        "setCompiledStyleRecord" => &["removeStyleProperty", "setStyleProperty"],
        "applyClassValue" => &["classValueToString"],
        "classValueToString" => &["appendClassTokens"],
        "appendClassTokens" => &["classTokenName"],
        "applyStyleObject" => &[
            "clearStyleAttribute",
            "hasStyleKey",
            "isStyleObject",
            "removeStyleProperty",
            "setStyleProperty",
            "style",
            "styleEntries",
            "styleKeys",
        ],
        "clearStyleObject" => &["removeStyleProperty", "styleKeys"],
        "clearStyleAttribute" => &["style"],
        "setStyleProperty" => &["cssStylePropertyName", "jsStylePropertyName"],
        "removeStyleProperty" => &["cssStylePropertyName", "jsStylePropertyName"],
        "cssStylePropertyName" => &["stylePropertyName"],
        "jsStylePropertyName" => &["stylePropertyName"],
        "setEvent" => &["dispatchTemplateEventResult"],
        "setCompiledEvent" => &[
            "canDelegateCompiledEvent",
            "dispatchTemplateEventResult",
            "removeDelegatedCompiledEventSlot",
            "setDelegatedCompiledEventSlot",
        ],
        "canDelegateCompiledEvent" => &["delegatedCompiledEvents"],
        "setDelegatedCompiledEventSlot" => &[
            "delegatedCompiledEventSlots",
            "ensureDelegatedCompiledEventListener",
        ],
        "removeDelegatedCompiledEventSlot" => &["delegatedCompiledEventSlots"],
        "ensureDelegatedCompiledEventListener" => &[
            "delegatedCompiledEventListeners",
            "dispatchDelegatedCompiledEvent",
        ],
        "dispatchDelegatedCompiledEvent" => &[
            "delegatedCompiledEvent",
            "delegatedCompiledEventSlots",
            "dispatchTemplateEventResult",
        ],
        "dispatchTemplateEventResult" => &[],
        "setRef" => &["refName", "registryForDispatch", "unregisterRef"],
        "setCompiledRef" => &[
            "compiledRefName",
            "compiledRegistryForDispatch",
            "unregisterRef",
        ],
        "setKeyedList" => &[
            "clearKeyedEntries",
            "disposeComponent",
            "duplicateStorageKey",
            "forceUpdateContext",
            "keyedItemUpdateContext",
            "reorderKeyedEntries",
            "updateKeyedComponent",
        ],
        "setCompiledKeyedList" => &[
            "clearKeyedEntries",
            "setCompiledKeyedListUnique",
            "setCompiledKeyedListWithDuplicates",
            "updateCompiledKeyedListSameOrder",
            "updateCompiledKeyedListSameSequence",
        ],
        "updateCompiledKeyedListSameOrder" => &[
            "canSkipCompiledKeyedEntry",
            "sameMapKey",
            "updateCompiledKeyedEntry",
        ],
        "updateCompiledKeyedListSameSequence" => &[
            "canSkipCompiledKeyedEntry",
            "compiledComponentArity",
            "disposeComponent",
            "disposeNewKeyedEntries",
            "insertKeyedEntries",
            "sameMapKey",
            "updateCompiledKeyedEntry",
        ],
        "setCompiledKeyedListUnique" => &[
            "canSkipCompiledKeyedEntry",
            "clearKeyedEntries",
            "compiledComponentArity",
            "disposeComponent",
            "disposeNewKeyedEntries",
            "insertKeyedEntries",
            "reorderKeyedEntries",
            "updateCompiledKeyedEntry",
        ],
        "setCompiledKeyedListWithDuplicates" => &[
            "canSkipCompiledKeyedEntry",
            "clearKeyedEntries",
            "compiledComponentArity",
            "disposeComponent",
            "duplicateStorageKey",
            "insertKeyedEntries",
            "reorderKeyedEntries",
            "updateCompiledKeyedEntry",
        ],
        "clearKeyedEntries" => &["disposeComponent"],
        "disposeNewKeyedEntries" => &["disposeComponent"],
        "insertKeyedEntries" => &[],
        "reorderKeyedEntries" => &["longestIncreasingSubsequenceIndexes"],
        "updateKeyedComponent" => &[],
        "updateCompiledKeyedEntry" => &["compiledEntryArity"],
        "canSkipCompiledKeyedEntry" => &["compiledEntryArity"],
        "compiledEntryArity" => &["compiledComponentArity"],
        "keyedItemUpdateContext" => &["changedStatePaths", "localUpdateContext"],
        "setConditional" => &["disposeComponent", "forceUpdateContext", "render"],
        "setCompiledConditional" => &["disposeComponent"],
        "setComponent" => &[
            "componentParams",
            "componentRenderKey",
            "componentUpdateContext",
            "disposeComponent",
            "forceUpdateContext",
            "render",
        ],
        "setCompiledComponent" => &[
            "compiledComponentArity",
            "disposeComponent",
            "sameCompiledComponentArgs",
        ],
        "disposeComponent" => &[],
        "componentUpdateContext" => &["changedStatePaths", "localUpdateContext"],
        "disposeEventSlots" => &["removeDelegatedCompiledEventSlot"],
        "disposeRefs" => &["unregisterRef"],
        "registryForDispatch" => &[],
        "compiledRegistryForDispatch" => &[],
        "Cmd" => &["commandOptions", "commands", "plainObject"],
        "Sub" => &["subscriptions"],
        "Decoder" => &[
            "decode",
            "decoderFieldPath",
            "decoderSpecEntries",
            "decoderTypeError",
            "decoderValueEqual",
            "hasOwn",
            "plainObject",
            "primitiveDecoder",
            "runDecoder",
        ],
        "decode" => &["runDecoder"],
        "describe" => &["flattenTestEntries"],
        "test" => &["flattenAssertions"],
        "collectCloskellTests" => &["flattenModuleTestEntries"],
        "runCloskellTest" => &[
            "closkellTestAssertions",
            "closkellTestName",
            "runCloskellAssertion",
        ],
        "runCloskellAssertion" => &[
            "assertionKind",
            "formatTestValue",
            "runEqualAssertion",
            "runMatchAssertion",
            "runThrowsAssertion",
        ],
        "registerVitestTests" => &[
            "describe",
            "moduleTestEntries",
            "registerVitestEntry",
            "test",
        ],
        "render" => &[
            "ensureRuntimeDocument",
            "messages",
            "testDispatchForHarness",
        ],
        "renderToString" => &[
            "annotateServerRenderedComponent",
            "ensureRuntimeDocument",
            "html",
            "serializeTestNode",
            "serverRenderDispatch",
        ],
        "render_to_string" => &["renderToString"],
        "rerender" => &["testDispatchForHarness"],
        "find" => &["harnessRoot"],
        "find_all" => &["harnessRoot"],
        "text" => &["find", "harnessRoot"],
        "html" => &["find", "harnessRoot"],
        "attr" => &["find"],
        "class_" => &["find"],
        "style" => &["cssStylePropertyName", "find", "jsStylePropertyName"],
        "subscriptions" => &["testVisibleSubscription"],
        "testVisibleSubscription" => &["commandValueName", "testVisibleSubscriptionKind"],
        "mount_app" => &[
            "commands",
            "createCommandHandlers",
            "createSubscriptionHandlers",
            "ensureRuntimeDocument",
            "messages",
            "normalizeTestCommandEnv",
            "normalizeTestHandlers",
            "startApp",
            "subscriptions",
            "testOption",
            "testVisibleSubscription",
        ],
        "fire" => &[
            "applyTestInputValue",
            "dispatchTestEvent",
            "find",
            "testEventInit",
        ],
        "scopeUpdate" => &["mapScopedCommand", "normalizeUpdateResult", "scopeKey"],
        "scopeSubscriptions" => &["mapScopedSubscription", "subscriptions"],
        "scopeView" => &[
            "forceUpdateContext",
            "scopedMessageDispatch",
            "scopedViewUpdateContext",
        ],
        "createCommandHandlers" => &[
            "addMediaQueryListener",
            "animationFrameMessage",
            "applyBrowserTheme",
            "applyCanvasOp",
            "applyCanvasState",
            "applyWindowEventControls",
            "bluetoothRequestOptions",
            "cancelAnimationFrameEntry",
            "canvasDrawSizing",
            "canvasMeasureTexts",
            "clearFileInput",
            "commandCancelMessage",
            "commandErrorMessage",
            "commandMessage",
            "commandValueName",
            "downloadWithBrowser",
            "eventListenerOptions",
            "find",
            "headersToObject",
            "httpRequestFetchArgs",
            "httpResponsePayload",
            "importWithBrowser",
            "loadAuthStorage",
            "loadBrowserTheme",
            "measureNode",
            "mediaQueryMessage",
            "namedCommandMessage",
            "nowMs",
            "numberOr",
            "numberOrZero",
            "parseHeartRateMeasurement",
            "parseStoredValue",
            "persistAuthStorage",
            "proxiedHttpUrl",
            "queueScrollIntoView",
            "readImportedFile",
            "rectFromResizeEntry",
            "refName",
            "removeMediaQueryListener",
            "removeResizeObserver",
            "removeWindowEventListener",
            "replaceBrowserSearchParam",
            "resizeMessage",
            "resolveRef",
            "runTask",
            "serializeStoredValue",
            "setBrowserCookie",
            "setCanvasCssSize",
            "setCanvasTransform",
            "simulationHeartRateBpm",
            "taskErrorMessage",
            "taskSuccessMessage",
            "text",
            "windowEventMessage",
            "writeBrowserRoute",
        ],
        "createSelectedCommandHandlers" => &[],
        "createCompiledCommandHandlers" => &[],
        "addCommandDisposer" => &[],
        "registerBluetoothCommandHandlers" => &[
            "addCommandDisposer",
            "bluetoothRequestOptions",
            "commandErrorMessage",
            "commandMessage",
            "namedCommandMessage",
            "parseHeartRateMeasurement",
        ],
        "registerCompiledBluetoothHeartRateCommandHandlers" => &[
            "addCommandDisposer",
            "compiledBluetoothRequestOptions",
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "compiledNamedCommandMessage",
            "parseHeartRateMeasurement",
        ],
        "registerTimerCommandHandlers" => &["addCommandDisposer", "commandMessage"],
        "registerCompiledTimerCommandHandlers" => &["addCommandDisposer", "compiledCommandMessage"],
        "registerAnimationCommandHandlers" => &[
            "addCommandDisposer",
            "animationFrameMessage",
            "cancelAnimationFrameEntry",
            "commandErrorMessage",
            "commandMessage",
        ],
        "registerCompiledAnimationCommandHandlers" => &[
            "addCommandDisposer",
            "cancelAnimationFrameEntry",
            "compiledAnimationFrameMessage",
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
        ],
        "registerTimeCommandHandlers" => &["commandMessage"],
        "registerCompiledTimeCommandHandlers" => &["compiledCommandMessage"],
        "registerStorageCommandHandlers" => &[
            "commandErrorMessage",
            "commandMessage",
            "parseStoredValue",
            "serializeStoredValue",
        ],
        "registerCompiledStorageCommandHandlers" => &[
            "registerCompiledStorageReadWriteCommandHandlers",
            "registerCompiledStorageRemoveCommandHandlers",
        ],
        "registerCompiledStorageReadWriteCommandHandlers" => &[
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "parseCompiledStoredValue",
            "serializeStoredValue",
        ],
        "registerCompiledStorageRemoveCommandHandlers" => {
            &["compiledCommandErrorMessage", "compiledCommandMessage"]
        }
        "registerBrowserCommandHandlers" => &[
            "applyBrowserTheme",
            "commandMessage",
            "loadBrowserTheme",
            "replaceBrowserSearchParam",
            "setBrowserCookie",
            "text",
            "writeBrowserRoute",
        ],
        "registerAuthStorageCommandHandlers" => {
            &["commandMessage", "loadAuthStorage", "persistAuthStorage"]
        }
        "registerRandomCommandHandlers" => &["commandMessage"],
        "registerCompiledRandomCommandHandlers" => &["compiledCommandMessage"],
        "registerSimulationCommandHandlers" => &[
            "addCommandDisposer",
            "commandMessage",
            "namedCommandMessage",
            "numberOr",
            "simulationHeartRateBpm",
        ],
        "registerCompiledSimulationCommandHandlers" => &[
            "addCommandDisposer",
            "compiledCommandMessage",
            "compiledNamedCommandMessage",
            "numberOr",
            "simulationHeartRateBpm",
        ],
        "registerTaskCommandHandlers" => &["runTask", "taskErrorMessage", "taskSuccessMessage"],
        "registerHttpCommandHandlers" => &[
            "commandErrorMessage",
            "commandMessage",
            "commandValueName",
            "headersToObject",
            "httpRequestFetchArgs",
            "httpResponsePayload",
            "nowMs",
            "proxiedHttpUrl",
        ],
        "registerCompiledSimulationStopCommandHandlers" => &["compiledCommandMessage"],
        "registerFileDownloadCommandHandlers" => &["commandMessage", "downloadWithBrowser"],
        "registerCompiledFileDownloadCommandHandlers" => &["compiledCommandMessage"],
        "registerFileImportCommandHandlers" => &[
            "commandCancelMessage",
            "commandErrorMessage",
            "commandMessage",
            "commandValueName",
            "importWithBrowser",
        ],
        "registerFileReadSelectedCommandHandlers" => &[
            "clearFileInput",
            "commandCancelMessage",
            "commandErrorMessage",
            "commandMessage",
            "commandValueName",
            "readImportedFile",
            "resolveRef",
        ],
        "registerCompiledFileReadSelectedCommandHandlers" => &[
            "clearFileInput",
            "compiledCommandCancelMessage",
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "readImportedFile",
            "resolveCompiledRef",
        ],
        "registerCanvasDrawCommandHandlers" => &[
            "applyCanvasOp",
            "canvasDrawSizing",
            "commandErrorMessage",
            "commandMessage",
            "refName",
            "resolveRef",
            "setCanvasCssSize",
            "setCanvasTransform",
        ],
        "registerCompiledCanvasDrawCommandHandlers" => &[
            "applyCompiledCanvasOp",
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "compiledRefName",
            "numberOrZero",
            "resolveCompiledRef",
            "setCanvasTransform",
        ],
        "registerCanvasMeasureTextCommandHandlers" => &[
            "applyCanvasState",
            "canvasMeasureTexts",
            "commandErrorMessage",
            "commandMessage",
            "numberOrZero",
            "refName",
            "resolveRef",
            "text",
        ],
        "registerDomRefCommandHandlers" => &[
            "commandErrorMessage",
            "commandMessage",
            "measureNode",
            "refName",
            "resolveRef",
        ],
        "registerCompiledDomRefCommandHandlers" => &[
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "compiledRefName",
            "measureNode",
            "resolveCompiledRef",
        ],
        "registerDomScrollCommandHandlers" => &["commandMessage", "queueScrollIntoView"],
        "registerDomResizeCommandHandlers" => &[
            "addCommandDisposer",
            "commandErrorMessage",
            "commandMessage",
            "find",
            "measureNode",
            "rectFromResizeEntry",
            "refName",
            "removeResizeObserver",
            "resizeMessage",
            "resolveRef",
        ],
        "registerCompiledDomResizeCommandHandlers" => &[
            "addCommandDisposer",
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "compiledResizeMessage",
            "find",
            "measureNode",
            "rectFromResizeEntry",
            "removeResizeObserver",
            "resolveCompiledRef",
        ],
        "registerCompiledDirectDomResizeCommandHandlers" => &[
            "addCommandDisposer",
            "compiledCommandErrorMessage",
            "removeResizeObserver",
            "resolveCompiledRef",
        ],
        "registerWindowEventCommandHandlers" => &[
            "addCommandDisposer",
            "applyWindowEventControls",
            "commandErrorMessage",
            "commandMessage",
            "eventListenerOptions",
            "removeWindowEventListener",
            "windowEventMessage",
        ],
        "registerCompiledWindowEventCommandHandlers" => &[
            "addCommandDisposer",
            "applyCompiledWindowEventControls",
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "compiledWindowEventMessage",
            "removeWindowEventListener",
        ],
        "registerCompiledDirectWindowEventCommandHandlers" => {
            &["addCommandDisposer", "removeWindowEventListener"]
        }
        "registerMediaQueryCommandHandlers" => &[
            "addCommandDisposer",
            "addMediaQueryListener",
            "commandErrorMessage",
            "commandMessage",
            "mediaQueryMessage",
            "removeMediaQueryListener",
        ],
        "registerCompiledMediaQueryCommandHandlers" => &[
            "addCommandDisposer",
            "addMediaQueryListener",
            "compiledCommandErrorMessage",
            "compiledCommandMessage",
            "compiledMediaQueryMessage",
            "removeMediaQueryListener",
        ],
        "registerCompiledDirectMediaQueryCommandHandlers" => &[
            "addCommandDisposer",
            "addMediaQueryListener",
            "removeMediaQueryListener",
        ],
        "createSubscriptionHandlersFor" => &[
            "commandErrorMessage",
            "commandKind",
            "startCommandForSubscription",
            "stopCommandForSubscription",
        ],
        "createSubscriptionHandlers" => &["createCommandHandlers", "createSubscriptionHandlersFor"],
        "startConfiguredApp" => &["createSubscriptionHandlersFor", "startAppCore"],
        "startApp" => &["createSubscriptionHandlers", "startAppCore"],
        "startCompiledApp" => &[
            "compiledCommandErrorMessage",
            "compiledRefName",
            "compiledStartCommandForSubscription",
            "compiledStopCommandForSubscription",
        ],
        "startCompiledAppWithoutSubscriptions" => {
            &["compiledCommandErrorMessage", "compiledRefName"]
        }
        "createCompiledSubscriptionHandlersFor" => &[
            "compiledCommandErrorMessage",
            "compiledCommandKind",
            "compiledStartCommandForSubscription",
            "compiledStopCommandForSubscription",
        ],
        "startCompiledAppCore" => &[
            "flattenSubscriptions",
            "normalizeUpdateResult",
            "reconcileCompiledSubscriptions",
            "refName",
            "runCompiledCommand",
            "stopAllCompiledSubscriptions",
            "subscriptions",
        ],
        "startAppCore" => &[
            "changedStatePaths",
            "commands",
            "emitDevtools",
            "emitDispatchDevtools",
            "flattenSubscriptions",
            "mountAppComponent",
            "normalizeUpdateResult",
            "reconcileSubscriptions",
            "refName",
            "runCommand",
            "stopAllSubscriptions",
            "subscriptions",
        ],
        "hydrateApp" => &["resolveHydrationRoot", "startApp"],
        "mountAppComponent" => &["hydrationCandidateForComponent"],
        "hydrationCandidateForComponent" => &["find"],
        "flattenSubscriptions" => &["commands", "subscriptionKind", "subscriptions"],
        "reconcileSubscriptions" => &[
            "startSubscription",
            "stopSubscription",
            "subscriptionKey",
            "subscriptionSignature",
        ],
        "reconcileCompiledSubscriptions" => &[
            "compiledSubscriptionKey",
            "compiledSubscriptionSignature",
            "runCompiledSubscriptionHandler",
        ],
        "stopAllSubscriptions" => &["stopSubscription"],
        "stopAllCompiledSubscriptions" => &["runCompiledSubscriptionHandler"],
        "startSubscription" => &[
            "emitDispatchDevtools",
            "runSubscriptionHandler",
            "subscriptionKind",
        ],
        "stopSubscription" => &[
            "emitDispatchDevtools",
            "runSubscriptionHandler",
            "subscriptionKind",
        ],
        "runSubscriptionHandler" => &["callSubscriptionHandler", "handleSubscriptionError"],
        "runCompiledSubscriptionHandler" => &[
            "callCompiledSubscriptionHandler",
            "dispatchCompiledCommandError",
        ],
        "callSubscriptionHandler" => &["subscriptionKind"],
        "callCompiledSubscriptionHandler" => &["compiledCommandKind"],
        "handleSubscriptionError" => &[
            "commandErrorMessage",
            "emitDispatchDevtools",
            "errorMessage",
            "subscriptionKind",
        ],
        "runCommand" => &[
            "commands",
            "compiledCommandKind",
            "emitDispatchDevtools",
            "handleCommandError",
        ],
        "runCompiledCommand" => &[
            "commands",
            "compiledCommandKind",
            "dispatchCompiledCommandError",
        ],
        "dispatchCompiledCommandError" => &["compiledCommandErrorMessage"],
        "runTask" => &["commandKind", "runHttpTask"],
        "runHttpTask" => &["errorMessage"],
        "taskSuccessMessage" => &["commandMessage"],
        "taskErrorMessage" => &["commandErrorMessage"],
        "moduleTestEntries" => &["normalizeModuleTestEntries"],
        "normalizeModuleTestEntries" => &["isLegacyTestRecord", "isTestCase", "isTestGroup"],
        "flattenModuleTestEntries" => &[
            "closkellTestName",
            "fullTestName",
            "isLegacyTestRecord",
            "isTestCase",
            "isTestGroup",
        ],
        "registerVitestEntry" => &[
            "closkellTestName",
            "formatTestFailure",
            "isLegacyTestRecord",
            "isTestCase",
            "isTestGroup",
            "normalizeModuleTestEntries",
            "runCloskellTest",
        ],
        "assertionKind" => &["symbolKey"],
        "closkellTestAssertions" => &["isTestCase"],
        "runEqualAssertion" => &["deepEqual", "formatTestValue"],
        "runMatchAssertion" => &["deepMatch", "formatTestValue"],
        "runThrowsAssertion" => &["errorMessage", "formatTestValue"],
        "deepEqual" => &["isPlainObject", "symbolKey"],
        "deepMatch" => &["deepEqual", "isPlainObject", "symbolKey"],
        "testDispatchForHarness" => &["messages", "testEventSnapshot"],
        "serverRenderDispatch" => &["dispatch"],
        "annotateServerRenderedComponent" => &["serverSlotMetadata"],
        "dispatchTestEvent" => &["createTestEvent", "testEventSnapshot"],
        "normalizeTestHandlers" => &["handlerKey"],
        "normalizeTestCommandEnv" => &["normalizeTestHandlers", "testStorageFromMap"],
        "querySelectorAllFrom" => &["descendantsOf", "selectorMatchesNode"],
        "selectorMatchesNode" => &["selectorChainMatches"],
        "selectorChainMatches" => &["simpleSelectorMatches"],
        "serializeTestNode" => &["escapeHtmlText", "serializableAttributes"],
        "serializableAttributes" => &["style"],
        "handleCommandError" => &[
            "commandErrorMessage",
            "commandKind",
            "emitDispatchDevtools",
            "errorMessage",
        ],
        "emitDispatchDevtools" => &["emitDevtools"],
        "changedStatePaths" => &["isChangeObject", "mapsEqual", "setsEqual"],
        "scopedMessageDispatch" => &["wrapScopedMessage"],
        "scopedViewUpdateContext" => &["changedStatePaths"],
        "mapScopedCommand" => &["commandKind", "commands", "mapScopedContinuations"],
        "mapScopedSubscription" => &[
            "commands",
            "mapScopedContinuations",
            "subscriptionKind",
            "subscriptions",
        ],
        "mapScopedContinuations" => &[
            "errorMessage",
            "mapScopedPayloadContinuation",
            "wrapScopedMessage",
        ],
        "mapScopedPayloadContinuation" => &["namedCommandMessage", "wrapScopedMessage"],
        "subscriptionKind" => &["commandKind"],
        "subscriptionKey" => &["subscriptionKind"],
        "compiledSubscriptionKey" => &["compiledCommandKind"],
        "startCommandForSubscription" => &["subscriptionKind"],
        "compiledStartCommandForSubscription" => &["compiledCommandKind"],
        "stopCommandForSubscription" => &["subscriptionKind"],
        "compiledStopCommandForSubscription" => &["compiledCommandKind"],
        "commandErrorMessage" => &["errorMessage"],
        "compiledCommandErrorMessage" => &["errorMessage"],
        "simulationHeartRateBpm" => &["clampNumber"],
        "httpRequestFetchArgs" => &[
            "HTTP_REQUEST_OPTION_FIELDS",
            "plainObject",
            "resolveHttpRequestBody",
        ],
        "httpResponsePayload" => &[
            "baseMime",
            "blobPreviewUrl",
            "headerValue",
            "isLikelyFileResponse",
            "looksLikeBinary",
            "looksLikeCsv",
            "resolveHttpFileName",
            "textFileResponse",
            "textMime",
        ],
        "httpExtensionFromContentType" => &["baseMime"],
        "inferHttpFileNameFromUrl" => &["httpExtensionFromContentType"],
        "resolveHttpFileName" => &["httpFileNameFromDisposition", "inferHttpFileNameFromUrl"],
        "looksLikeCsv" => &[],
        "looksLikeBinary" => &[],
        "isLikelyFileResponse" => &["baseMime"],
        "blobPreviewUrl" => &["baseMime"],
        "resolveHttpRequestBody" => &[
            "commandValueName",
            "multipartFormBody",
            "selectedFileByTestId",
        ],
        "multipartFormBody" => &["hasOwn", "selectedFileByTestId"],
        "primitiveDecoder" => &["decode", "decoderTypeError"],
        "runDecoder" => &["decode"],
        "decoderSpecEntries" => &["decoderFieldName", "plainObject"],
        "decoderFieldPath" => &[],
        "commandOptions" => &["hasOwn", "plainObject"],
        "resolveRef" => &["refName"],
        "resolveCompiledRef" => &["compiledRefName"],
        "measureNode" => &["numberOrZero"],
        "rectFromResizeEntry" => &["measureNode", "numberOr"],
        "resizeMessage" => &["namedCommandMessage", "refName"],
        "compiledResizeMessage" => &["compiledNamedCommandMessage", "compiledRefName"],
        "windowEventMessage" => &["namedCommandMessage", "windowEventPayload"],
        "compiledWindowEventMessage" => &["compiledNamedCommandMessage", "windowEventPayload"],
        "applyWindowEventControls" => &["eventControlMatches"],
        "applyCompiledWindowEventControls" => &["compiledEventControlMatches"],
        "queueScrollIntoView" => &["nodeFullyVisible", "scrollTargetNode"],
        "scrollTargetNode" => &["cssAttr"],
        "animationFrameMessage" => &["namedCommandMessage", "numberOrZero"],
        "compiledAnimationFrameMessage" => &["compiledNamedCommandMessage", "numberOrZero"],
        "canvasDrawSizing" => &["canvasPixelRatio", "numberOrUndefined"],
        "canvasPixelRatio" => &["commandValueName", "numberOrZero"],
        "setCanvasCssSize" => &[],
        "mediaQueryMessage" => &["namedCommandMessage"],
        "compiledMediaQueryMessage" => &["compiledNamedCommandMessage"],
        "canvasMeasureTexts" => &[],
        "applyCanvasOp" => &["applyCanvasState", "commandValueName", "text"],
        "applyCompiledCanvasOp" => &["applyCompiledCanvasState"],
        "applyCanvasState" => &["canvasTextStateValue"],
        "canvasTextStateValue" => &["commandValueName"],
        "parseStoredValue" => &["commandValueName"],
        "downloadWithBrowser" => &[],
        "importWithBrowser" => &["readImportedFile"],
        "readImportedFile" => &[],
        _ => &[],
    }
}

fn runtime_chunks(source: &str) -> Vec<RuntimeChunk> {
    let mut starts = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if let Some(name) = runtime_declaration_name(line.trim_end_matches(['\r', '\n'])) {
            starts.push((offset, name));
        }
        offset += line.len();
    }

    let mut chunks = Vec::new();
    for index in 0..starts.len() {
        let (start, name) = &starts[index];
        let end = starts
            .get(index + 1)
            .map(|(next_start, _)| *next_start)
            .unwrap_or(source.len());
        chunks.push(RuntimeChunk {
            name: name.clone(),
            code: source[*start..end].to_string(),
        });
    }
    chunks
}

fn runtime_declaration_name(line: &str) -> Option<String> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let line = line.strip_prefix("export ").unwrap_or(line);
    let line = line.strip_prefix("async ").unwrap_or(line);
    for keyword in ["class ", "function ", "const ", "let ", "var "] {
        let Some(rest) = line.strip_prefix(keyword) else {
            continue;
        };
        let name = rest
            .chars()
            .take_while(|ch| is_js_identifier_continue(*ch))
            .collect::<String>();
        if is_js_identifier(&name) {
            return Some(name);
        }
    }
    None
}

fn is_js_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_js_identifier_start(first) && chars.all(is_js_identifier_continue)
}

fn is_js_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_js_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

fn build_file(
    path: &Path,
    output: &Path,
    visited: &mut HashSet<PathBuf>,
    options: &BuildOptions,
    modules: &HashMap<PathBuf, ModuleInfo>,
    artifacts: &mut Vec<BuildArtifact>,
    kind: &str,
) -> Result<(), String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }

    let module = modules
        .get(&canonical)
        .ok_or_else(|| format!("missing checked module: {}", path.display()))?;
    let input = &module.input;
    let source = &module.source;

    let mut module_emit_options = options.emit_options.clone();
    if options.app.is_some() {
        module_emit_options.browser_app_runtime = true;
    }

    let mut import_emit_options = module_emit_options.clone();
    if options.app.is_some() {
        import_emit_options.message_field_reads =
            js_backend::collect_message_field_reads(&module.source);
    }

    for import in parse_imports(&input, &source)? {
        if !is_closkell_import_path(&import.path) {
            continue;
        }
        let import_output = output_for_import(output, &import.path)?;
        if !import_has_runtime_names(path, &import, modules)? {
            remove_stale_type_only_output(path, &import.path, &import_output, visited)?;
            continue;
        }
        let import_source = resolve_import_source(path, &import.path)?;
        build_file(
            &import_source,
            &import_output,
            visited,
            &BuildOptions {
                source_maps: options.source_maps,
                app: None,
                emit_options: import_emit_options.clone(),
            },
            modules,
            artifacts,
            "import",
        )?;
    }

    let mut emitted = emit_checked_module_from_checked(
        path,
        input,
        module,
        modules,
        &module_emit_options,
        options.app.is_some(),
    )?;
    if let Some(app) = &options.app {
        let runtime_registrations =
            collect_app_runtime_registrations(&emitted.runtime_effects, artifacts);
        let init_takes_boot = emitted_init_takes_boot(&emitted.code);
        let has_subscriptions = emitted_has_binding(&emitted.code, "subscriptions");
        wrap_app_module(
            &mut emitted,
            app,
            &runtime_registrations,
            init_takes_boot,
            has_subscriptions,
        );
    }
    let runtime_exports = collect_runtime_import_exports(&emitted.code);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
    }
    let code = if options.source_maps {
        with_source_mapping_url(emitted.code, output)
    } else {
        emitted.code
    };
    fs::write(output, code)
        .map_err(|err| format!("failed to write {}: {}", output.display(), err))?;
    if options.source_maps {
        write_source_map(path, &input, output, &emitted.source_mappings)?;
    } else {
        remove_file_if_exists(&source_map_path(output))?;
    }
    if let Some(app) = &options.app {
        if app.vendor_runtime {
            let mut required_exports = runtime_exports.clone();
            for artifact in artifacts.iter() {
                required_exports.extend(artifact.runtime_exports.iter().cloned());
            }
            copy_runtime_package_with_exports(
                &runtime_vendor_root(output),
                Some(&required_exports),
            )?;
        }
    }
    artifacts.push(BuildArtifact {
        kind: kind.to_string(),
        source: fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
        output: fs::canonicalize(output).unwrap_or_else(|_| output.to_path_buf()),
        source_map: options
            .source_maps
            .then(|| source_map_path(output))
            .map(|path| fs::canonicalize(&path).unwrap_or(path)),
        bytes: fs::metadata(output)
            .map(|metadata| metadata.len())
            .unwrap_or(0),
        runtime_effects: emitted.runtime_effects,
        runtime_exports,
    });
    Ok(())
}

fn import_has_runtime_names(
    source_path: &Path,
    import: &ImportSpec,
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<bool, String> {
    if !is_closkell_import_path(&import.path) {
        return Ok(import
            .names
            .iter()
            .any(|name| js_backend::is_runtime_import_name(&name.name)));
    }
    let import_source = resolve_import_source(source_path, &import.path)?;
    let canonical = fs::canonicalize(&import_source)
        .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
    let imported = modules.get(&canonical);
    Ok(import.names.iter().any(|name| {
        js_backend::is_runtime_import_name(&name.name)
            && !imported.is_some_and(|module| module.macros.contains_key(&name.imported))
    }))
}

fn remove_stale_type_only_output(
    source_path: &Path,
    import_path: &str,
    output: &Path,
    visited: &HashSet<PathBuf>,
) -> Result<(), String> {
    if !is_closkell_import_path(import_path) {
        return Ok(());
    }
    let import_source = resolve_import_source(source_path, import_path)?;
    let canonical = fs::canonicalize(&import_source)
        .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
    if visited.contains(&canonical) {
        return Ok(());
    }

    remove_file_if_exists(output)?;
    remove_file_if_exists(&source_map_path(output))
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to remove {}: {}", path.display(), err)),
    }
}

fn build_single_module(
    path: &Path,
    modules: &HashMap<PathBuf, ModuleInfo>,
    emit_options: &js_backend::EmitOptions,
) -> Result<String, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    let module = modules
        .get(&canonical)
        .ok_or_else(|| format!("missing checked module: {}", path.display()))?;
    emit_checked_module_from_checked(path, &module.input, module, modules, emit_options, false)
        .map(|emitted| emitted.code)
}

fn emit_checked_module_from_checked(
    path: &Path,
    input: &str,
    module: &ModuleInfo,
    modules: &HashMap<PathBuf, ModuleInfo>,
    emit_options: &js_backend::EmitOptions,
    prune_update_messages: bool,
) -> Result<js_backend::EmitResult, String> {
    let source = &module.source;
    let imports = parse_imports(input, source)?;
    let imported_message_kinds = message_imports_from_imports(path, &imports, modules)?;

    let mut emit_options = emit_options.clone();
    if prune_update_messages {
        emit_options.static_reads = collect_app_static_reads(source, &emit_options);
        emit_options.reachable_message_kinds =
            collect_reachable_update_message_kinds(source, &imported_message_kinds, &emit_options);
    }
    let local_specializations = collect_local_function_specializations(
        path,
        &imports,
        modules,
        source,
        &module.expr_types,
        &emit_options,
    )?;
    for specialization in local_specializations {
        emit_options
            .direct_call_replacements
            .insert(specialization.local_name, specialization.specialized_name);
        append_specialization_prelude(&mut emit_options.prelude_code, &specialization.code);
    }

    let emitted = js_backend::emit_module_with_types_and_options(
        source,
        module.expr_types.clone(),
        emit_options,
    );
    if !emitted.diagnostics.is_empty() {
        println!("{}", render_diagnostics(input, &emitted.diagnostics));
        return Err(format!(
            "build failed during JS emission: {}",
            path.display()
        ));
    }

    Ok(emitted)
}

#[derive(Clone, Debug)]
struct ImportFunctionSpecialization {
    local_name: String,
    specialized_name: String,
    code: String,
}

#[derive(Clone, Debug)]
struct ImportedFunctionTarget {
    imported_name: String,
    module_path: PathBuf,
    schema: String,
}

#[derive(Clone, Debug, Default)]
struct CallSignature {
    arg_types: Option<Vec<String>>,
    conflict: bool,
}

fn append_specialization_prelude(prelude: &mut String, code: &str) {
    for line in code.lines() {
        if specialization_helper_line_already_present(prelude, line) {
            continue;
        }
        prelude.push_str(line);
        prelude.push('\n');
    }
    if !prelude.ends_with('\n') {
        prelude.push('\n');
    }
}

fn specialization_helper_line_already_present(prelude: &str, line: &str) -> bool {
    [
        "const __closkellValueEqual",
        "const __closkellCount",
        "const __closkellIsObject",
        "const __closkellObjectEntries",
        "const __closkellNone",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix) && prelude.contains(prefix))
}

fn collect_local_function_specializations(
    path: &Path,
    imports: &[ImportSpec],
    modules: &HashMap<PathBuf, ModuleInfo>,
    source: &SourceFile,
    expr_types: &BTreeMap<usize, String>,
    emit_options: &js_backend::EmitOptions,
) -> Result<Vec<ImportFunctionSpecialization>, String> {
    let local_schemas = local_function_schemas(source, expr_types);
    if local_schemas.is_empty() {
        return Ok(Vec::new());
    }

    let imported_targets = imported_function_targets(path, imports, modules)?;
    let interesting_names =
        interesting_local_function_names(source, &local_schemas, &imported_targets, modules);
    let local_names = local_schemas.keys().cloned().collect::<HashSet<_>>();
    let mut root_signatures = BTreeMap::new();
    let mut bound = HashSet::new();
    for form in &source.forms {
        collect_direct_call_signatures(
            form,
            &local_names,
            expr_types,
            &mut bound,
            &mut root_signatures,
        );
    }

    let mut memo = BTreeMap::new();
    let mut visiting = HashSet::new();
    let mut emitted_code = Vec::new();
    let mut roots = Vec::new();
    for (local_name, signature) in root_signatures {
        if !interesting_names.contains(&local_name) {
            continue;
        }
        if signature.conflict {
            continue;
        }
        let Some(arg_types) = signature.arg_types else {
            continue;
        };
        let Some(specialized_name) = emit_recursive_local_specialization(
            &local_name,
            &arg_types,
            source,
            expr_types,
            &local_schemas,
            &imported_targets,
            modules,
            emit_options,
            &interesting_names,
            &mut memo,
            &mut visiting,
            &mut emitted_code,
        ) else {
            continue;
        };
        roots.push(ImportFunctionSpecialization {
            local_name,
            specialized_name,
            code: String::new(),
        });
    }

    if emitted_code.is_empty() {
        return Ok(Vec::new());
    }
    let prelude = emitted_code.join("");
    for (index, root) in roots.iter_mut().enumerate() {
        if index == 0 {
            root.code = prelude.clone();
        }
    }
    Ok(roots)
}

fn emit_recursive_local_specialization(
    name: &str,
    arg_types: &[String],
    source: &SourceFile,
    base_expr_types: &BTreeMap<usize, String>,
    local_schemas: &BTreeMap<String, String>,
    imported_targets: &BTreeMap<String, ImportedFunctionTarget>,
    modules: &HashMap<PathBuf, ModuleInfo>,
    emit_options: &js_backend::EmitOptions,
    interesting_names: &HashSet<String>,
    memo: &mut BTreeMap<String, String>,
    visiting: &mut HashSet<String>,
    emitted_code: &mut Vec<String>,
) -> Option<String> {
    let key = format!("local:{}|{}", name, arg_types.join("|"));
    if let Some(existing) = memo.get(&key) {
        return Some(existing.clone());
    }
    if !visiting.insert(key.clone()) {
        return None;
    }

    let schema = local_schemas.get(name)?;
    let param_types = type_fn_param_types_for_schema(schema)?;
    if param_types.len() != arg_types.len() {
        visiting.remove(&key);
        return None;
    }
    let mut substitutions = BTreeMap::new();
    if !param_types
        .iter()
        .zip(arg_types.iter())
        .all(|(pattern, concrete)| {
            collect_type_substitutions(pattern, concrete, &mut substitutions)
        })
    {
        visiting.remove(&key);
        return None;
    }
    let specialized_expr_types = substitute_expr_type_map(base_expr_types, &substitutions);
    let local_targets = local_schemas.keys().cloned().collect::<HashSet<_>>();
    let imported_names = imported_targets.keys().cloned().collect::<HashSet<_>>();
    let mut all_targets = local_targets.clone();
    all_targets.extend(imported_names.iter().cloned());
    let defn = find_defn_form(source, name)?;
    let nested_calls = concrete_calls_in_defn(defn, &all_targets, &specialized_expr_types);
    let mut replacements = BTreeMap::new();
    for (callee, callee_types) in nested_calls {
        if local_targets.contains(&callee) {
            if !interesting_names.contains(&callee) {
                continue;
            }
            if let Some(specialized) = emit_recursive_local_specialization(
                &callee,
                &callee_types,
                source,
                base_expr_types,
                local_schemas,
                imported_targets,
                modules,
                emit_options,
                interesting_names,
                memo,
                visiting,
                emitted_code,
            ) {
                replacements.insert(callee, specialized);
            }
        } else if imported_names.contains(&callee) {
            if let Some(specialized) = emit_imported_function_specialization(
                &callee,
                &callee_types,
                imported_targets,
                modules,
                emit_options,
                memo,
                emitted_code,
            ) {
                replacements.insert(callee, specialized);
            }
        }
    }

    let hash = stable_hash_hex(format!("{}|{}", name, arg_types.join("|")).as_bytes());
    let specialized_name = format!(
        "__closkell_specialized_{}_{}",
        sanitize_js_identifier(name),
        &hash[..8]
    );
    let mut specialization_options = emit_options.clone();
    specialization_options.direct_call_replacements = replacements;
    specialization_options.prelude_code.clear();
    let emitted = js_backend::emit_function_specialization(
        source,
        specialized_expr_types,
        name,
        &specialized_name,
        specialization_options,
    );
    if emitted.diagnostics.is_empty()
        && !emitted.code.contains("__closkellValueEqual")
        && !emitted.code.contains("__closkellCreateTemplate")
        && !emitted.code.contains("__closkellDecoder")
    {
        memo.insert(key.clone(), specialized_name.clone());
        emitted_code.push(emitted.code);
        visiting.remove(&key);
        Some(specialized_name)
    } else {
        visiting.remove(&key);
        None
    }
}

fn emit_imported_function_specialization(
    local_name: &str,
    arg_types: &[String],
    imported_targets: &BTreeMap<String, ImportedFunctionTarget>,
    modules: &HashMap<PathBuf, ModuleInfo>,
    emit_options: &js_backend::EmitOptions,
    memo: &mut BTreeMap<String, String>,
    emitted_code: &mut Vec<String>,
) -> Option<String> {
    let target = imported_targets.get(local_name)?;
    let key = format!(
        "import:{}:{}|{}",
        target.module_path.display(),
        target.imported_name,
        arg_types.join("|")
    );
    if let Some(existing) = memo.get(&key) {
        return Some(existing.clone());
    }
    let module = modules.get(&target.module_path)?;
    let defn = find_defn_form(&module.source, &target.imported_name)?;
    if !defn_is_self_contained(defn) {
        return None;
    }
    let param_types = type_fn_param_types_for_schema(&target.schema)?;
    if param_types.len() != arg_types.len() {
        return None;
    }
    let mut substitutions = BTreeMap::new();
    if !param_types
        .iter()
        .zip(arg_types.iter())
        .all(|(pattern, concrete)| {
            collect_type_substitutions(pattern, concrete, &mut substitutions)
        })
    {
        return None;
    }
    let specialized_expr_types = substitute_expr_type_map(&module.expr_types, &substitutions);
    let hash =
        stable_hash_hex(format!("{}|{}", target.imported_name, arg_types.join("|")).as_bytes());
    let specialized_name = format!(
        "__closkell_specialized_{}_{}",
        sanitize_js_identifier(local_name),
        &hash[..8]
    );
    let mut specialization_options = emit_options.clone();
    specialization_options.direct_call_replacements.clear();
    specialization_options.prelude_code.clear();
    let emitted = js_backend::emit_function_specialization(
        &module.source,
        specialized_expr_types,
        &target.imported_name,
        &specialized_name,
        specialization_options,
    );
    if emitted.diagnostics.is_empty()
        && !emitted.code.contains("__closkellValueEqual")
        && !emitted.code.contains("__closkellCreateTemplate")
        && !emitted.code.contains("__closkellDecoder")
    {
        memo.insert(key, specialized_name.clone());
        emitted_code.push(emitted.code);
        Some(specialized_name)
    } else {
        None
    }
}

fn local_function_schemas(
    source: &SourceFile,
    expr_types: &BTreeMap<usize, String>,
) -> BTreeMap<String, String> {
    let mut schemas = BTreeMap::new();
    for form in &source.forms {
        let ExprKind::List(items) = &form.kind else {
            continue;
        };
        if items.len() < 4 || !matches_symbol(&items[0], "defn") {
            continue;
        }
        let ExprKind::Symbol(name) = &items[1].kind else {
            continue;
        };
        let Some(schema) = expr_types.get(&form.span.start) else {
            continue;
        };
        if schema.trim_start().starts_with("(Fn ") {
            schemas.insert(name.clone(), schema.clone());
        }
    }
    schemas
}

fn interesting_local_function_names(
    source: &SourceFile,
    local_schemas: &BTreeMap<String, String>,
    imported_targets: &BTreeMap<String, ImportedFunctionTarget>,
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> HashSet<String> {
    let local_names = local_schemas.keys().cloned().collect::<HashSet<_>>();
    let imported_equality_names = imported_targets
        .iter()
        .filter_map(|(local_name, target)| {
            let module = modules.get(&target.module_path)?;
            let defn = find_defn_form(&module.source, &target.imported_name)?;
            defn_contains_value_equal(defn).then_some(local_name.clone())
        })
        .collect::<HashSet<_>>();
    let mut calls_by_function = BTreeMap::new();
    let mut interesting = HashSet::new();

    for form in &source.forms {
        let Some(name) = definition_name(form) else {
            continue;
        };
        if !local_schemas.contains_key(name) {
            continue;
        }
        let local_calls = called_names_in_defn(form, &local_names);
        let imported_calls = called_names_in_defn(form, &imported_equality_names);
        if defn_contains_value_equal(form) || !imported_calls.is_empty() {
            interesting.insert(name.to_string());
        }
        calls_by_function.insert(name.to_string(), local_calls);
    }

    loop {
        let mut changed = false;
        for (name, calls) in &calls_by_function {
            if interesting.contains(name) {
                continue;
            }
            if calls.iter().any(|callee| interesting.contains(callee)) {
                changed |= interesting.insert(name.clone());
            }
        }
        if !changed {
            break;
        }
    }

    interesting
}

fn called_names_in_defn(defn: &Expr, target_names: &HashSet<String>) -> HashSet<String> {
    let ExprKind::List(items) = &defn.kind else {
        return HashSet::new();
    };
    if items.len() < 4 {
        return HashSet::new();
    }
    let mut calls = HashSet::new();
    let mut bound = HashSet::new();
    collect_specialization_pattern_bindings(&items[2], &mut bound);
    for body in &items[3..] {
        collect_called_names(body, target_names, &mut bound, &mut calls);
    }
    calls
}

fn collect_called_names(
    expr: &Expr,
    target_names: &HashSet<String>,
    bound: &mut HashSet<String>,
    calls: &mut HashSet<String>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let Some((head, args)) = items.split_first() else {
                return;
            };
            if let ExprKind::Symbol(name) = &head.kind {
                match name.as_str() {
                    "fn" => {
                        let mut nested_bound = bound.clone();
                        if let Some(params) = args.first() {
                            collect_specialization_pattern_bindings(params, &mut nested_bound);
                        }
                        for body in args.iter().skip(1) {
                            collect_called_names(body, target_names, &mut nested_bound, calls);
                        }
                        return;
                    }
                    "let" => {
                        let mut nested_bound = bound.clone();
                        if let Some(bindings) = args.first() {
                            if let ExprKind::Vector(items) = &bindings.kind {
                                for pair in items.chunks(2) {
                                    if let Some(value) = pair.get(1) {
                                        collect_called_names(
                                            value,
                                            target_names,
                                            &mut nested_bound,
                                            calls,
                                        );
                                    }
                                    if let Some(pattern) = pair.first() {
                                        collect_specialization_pattern_bindings(
                                            pattern,
                                            &mut nested_bound,
                                        );
                                    }
                                }
                            }
                        }
                        for body in args.iter().skip(1) {
                            collect_called_names(body, target_names, &mut nested_bound, calls);
                        }
                        return;
                    }
                    _ => {
                        if target_names.contains(name) && !bound.contains(name) {
                            calls.insert(name.clone());
                        }
                    }
                }
            }
            for item in items {
                collect_called_names(item, target_names, bound, calls);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_called_names(item, target_names, bound, calls);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                if !matches!(
                    key.kind,
                    ExprKind::Keyword(_) | ExprKind::String(_) | ExprKind::Symbol(_)
                ) {
                    collect_called_names(key, target_names, bound, calls);
                }
                collect_called_names(value, target_names, bound, calls);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_called_names(inner, target_names, bound, calls)
        }
        ExprKind::HtmlTemplate(_) => {}
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_) => {}
    }
}

fn concrete_calls_in_defn(
    defn: &Expr,
    target_names: &HashSet<String>,
    expr_types: &BTreeMap<usize, String>,
) -> BTreeMap<String, Vec<String>> {
    let ExprKind::List(items) = &defn.kind else {
        return BTreeMap::new();
    };
    if items.len() < 4 {
        return BTreeMap::new();
    }
    let mut signatures = BTreeMap::new();
    let mut bound = HashSet::new();
    collect_specialization_pattern_bindings(&items[2], &mut bound);
    for body in &items[3..] {
        collect_direct_call_signatures(body, target_names, expr_types, &mut bound, &mut signatures);
    }
    signatures
        .into_iter()
        .filter_map(|(name, signature)| {
            if signature.conflict {
                return None;
            }
            let arg_types = signature.arg_types?;
            Some((name, arg_types))
        })
        .collect()
}

fn substitute_expr_type_map(
    expr_types: &BTreeMap<usize, String>,
    substitutions: &BTreeMap<String, String>,
) -> BTreeMap<usize, String> {
    expr_types
        .iter()
        .map(|(offset, schema)| (*offset, substitute_type_variables(schema, substitutions)))
        .collect()
}

fn imported_function_targets(
    path: &Path,
    imports: &[ImportSpec],
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<BTreeMap<String, ImportedFunctionTarget>, String> {
    let mut targets = BTreeMap::new();
    for import in imports {
        if !is_closkell_import_path(&import.path) {
            continue;
        }
        let import_source = resolve_import_source(path, &import.path)?;
        let canonical = fs::canonicalize(&import_source)
            .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
        let Some(module) = modules.get(&canonical) else {
            continue;
        };
        for name in &import.names {
            if name.default || module.macros.contains_key(&name.imported) {
                continue;
            }
            let Some(schema) = module
                .bindings
                .iter()
                .find(|binding| binding.name == name.imported)
                .map(|binding| binding.schema())
                .or_else(|| {
                    find_defn_form(&module.source, &name.imported)
                        .and_then(|form| module.expr_types.get(&form.span.start).cloned())
                })
            else {
                continue;
            };
            if !schema.trim_start().starts_with("(Fn ") {
                continue;
            }
            targets.insert(
                name.name.clone(),
                ImportedFunctionTarget {
                    imported_name: name.imported.clone(),
                    module_path: canonical.clone(),
                    schema,
                },
            );
        }
    }
    Ok(targets)
}

fn collect_direct_call_signatures(
    expr: &Expr,
    target_names: &HashSet<String>,
    expr_types: &BTreeMap<usize, String>,
    bound: &mut HashSet<String>,
    signatures: &mut BTreeMap<String, CallSignature>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let Some((head, args)) = items.split_first() else {
                return;
            };
            if let ExprKind::Symbol(name) = &head.kind {
                match name.as_str() {
                    "fn" => {
                        let mut nested_bound = bound.clone();
                        if let Some(params) = args.first() {
                            collect_specialization_pattern_bindings(params, &mut nested_bound);
                        }
                        for body in args.iter().skip(1) {
                            collect_direct_call_signatures(
                                body,
                                target_names,
                                expr_types,
                                &mut nested_bound,
                                signatures,
                            );
                        }
                        return;
                    }
                    "let" => {
                        let mut nested_bound = bound.clone();
                        if let Some(bindings) = args.first() {
                            if let ExprKind::Vector(items) = &bindings.kind {
                                for pair in items.chunks(2) {
                                    if let Some(value) = pair.get(1) {
                                        collect_direct_call_signatures(
                                            value,
                                            target_names,
                                            expr_types,
                                            &mut nested_bound,
                                            signatures,
                                        );
                                    }
                                    if let Some(pattern) = pair.first() {
                                        collect_specialization_pattern_bindings(
                                            pattern,
                                            &mut nested_bound,
                                        );
                                    }
                                }
                            }
                        }
                        for body in args.iter().skip(1) {
                            collect_direct_call_signatures(
                                body,
                                target_names,
                                expr_types,
                                &mut nested_bound,
                                signatures,
                            );
                        }
                        return;
                    }
                    _ => {}
                }

                if target_names.contains(name) && !bound.contains(name) {
                    let arg_types = args
                        .iter()
                        .map(|arg| expr_types.get(&arg.span.start).cloned())
                        .collect::<Option<Vec<_>>>();
                    let entry = signatures.entry(name.clone()).or_default();
                    match (&entry.arg_types, arg_types) {
                        (None, Some(arg_types)) => entry.arg_types = Some(arg_types),
                        (Some(existing), Some(arg_types)) if existing == &arg_types => {}
                        _ => entry.conflict = true,
                    }
                }
            }
            for item in items {
                collect_direct_call_signatures(item, target_names, expr_types, bound, signatures);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_direct_call_signatures(item, target_names, expr_types, bound, signatures);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                if !matches!(
                    key.kind,
                    ExprKind::Keyword(_) | ExprKind::String(_) | ExprKind::Symbol(_)
                ) {
                    collect_direct_call_signatures(
                        key,
                        target_names,
                        expr_types,
                        bound,
                        signatures,
                    );
                }
                collect_direct_call_signatures(value, target_names, expr_types, bound, signatures);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_direct_call_signatures(inner, target_names, expr_types, bound, signatures);
        }
        ExprKind::HtmlTemplate(node) => {
            collect_html_call_signatures(node, target_names, expr_types, bound, signatures);
        }
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_) => {}
    }
}

fn collect_html_call_signatures(
    node: &syntax::HtmlNode,
    target_names: &HashSet<String>,
    expr_types: &BTreeMap<usize, String>,
    bound: &mut HashSet<String>,
    signatures: &mut BTreeMap<String, CallSignature>,
) {
    match node {
        syntax::HtmlNode::Expr { expr, .. } => {
            collect_direct_call_signatures(expr, target_names, expr_types, bound, signatures);
        }
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                match &attr.value {
                    syntax::HtmlAttrValue::Dynamic { expr, .. } => {
                        collect_direct_call_signatures(
                            expr,
                            target_names,
                            expr_types,
                            bound,
                            signatures,
                        );
                    }
                    syntax::HtmlAttrValue::Bool(_) | syntax::HtmlAttrValue::Static(_) => {}
                }
            }
            for child in &element.children {
                collect_html_call_signatures(child, target_names, expr_types, bound, signatures);
            }
        }
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn find_defn_form<'a>(source: &'a SourceFile, name: &str) -> Option<&'a Expr> {
    source.forms.iter().find(|form| {
        let ExprKind::List(items) = &form.kind else {
            return false;
        };
        items.len() >= 4
            && matches_symbol(&items[0], "defn")
            && matches!(&items[1].kind, ExprKind::Symbol(defn_name) if defn_name == name)
    })
}

fn imported_binding_type_is_importable(
    module: &ModuleInfo,
    binding: &typecheck::ExportedBinding,
) -> bool {
    if binding.is_annotated_or_value() {
        return true;
    }
    find_defn_form(&module.source, &binding.name)
        .is_some_and(|defn| !defn_contains_dynamic_type_flow(defn))
}

fn defn_contains_dynamic_type_flow(defn: &Expr) -> bool {
    let ExprKind::List(items) = &defn.kind else {
        return false;
    };
    items.iter().skip(3).any(expr_contains_dynamic_type_flow)
}

fn expr_contains_dynamic_type_flow(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::List(items) => {
            if items.first().is_some_and(|head| {
                matches!(
                    &head.kind,
                    ExprKind::Symbol(name)
                        if matches!(
                            name.as_str(),
                            "get"
                                | "object-get"
                                | "get-in"
                                | "number?"
                                | "string?"
                                | "bool?"
                                | "keyword?"
                                | "object?"
                                | "list?"
                                | "vector?"
                                | "set?"
                                | "map?"
                                | "json-parse"
                                | "json-parse-result"
                        )
                )
            }) {
                return true;
            }
            items.iter().any(expr_contains_dynamic_type_flow)
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            items.iter().any(expr_contains_dynamic_type_flow)
        }
        ExprKind::Map(entries) => entries.iter().any(|(key, value)| {
            expr_contains_dynamic_type_flow(key) || expr_contains_dynamic_type_flow(value)
        }),
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => expr_contains_dynamic_type_flow(inner),
        ExprKind::HtmlTemplate(_) => false,
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_) => false,
    }
}

fn defn_contains_value_equal(defn: &Expr) -> bool {
    let ExprKind::List(items) = &defn.kind else {
        return false;
    };
    items.iter().skip(3).any(expr_contains_value_equal)
}

fn expr_contains_value_equal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::List(items) => {
            items.first().is_some_and(|head| matches_symbol(head, "="))
                || items.iter().any(expr_contains_value_equal)
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            items.iter().any(expr_contains_value_equal)
        }
        ExprKind::Map(entries) => entries
            .iter()
            .any(|(key, value)| expr_contains_value_equal(key) || expr_contains_value_equal(value)),
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => expr_contains_value_equal(inner),
        ExprKind::HtmlTemplate(_) => false,
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_) => false,
    }
}

fn defn_is_self_contained(defn: &Expr) -> bool {
    let ExprKind::List(items) = &defn.kind else {
        return false;
    };
    if items.len() < 4 {
        return false;
    }
    let mut bound = HashSet::new();
    collect_specialization_pattern_bindings(&items[2], &mut bound);
    let mut free = HashSet::new();
    for body in &items[3..] {
        collect_free_symbols(body, &mut bound, &mut free);
    }
    free.is_empty()
}

fn collect_free_symbols(expr: &Expr, bound: &mut HashSet<String>, free: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::Symbol(name) => {
            let root = symbol_root(name);
            if !bound.contains(root) && !known_compiler_symbol(name) {
                free.insert(root.to_string());
            }
        }
        ExprKind::List(items) => {
            let Some((head, args)) = items.split_first() else {
                return;
            };
            if let ExprKind::Symbol(name) = &head.kind {
                match name.as_str() {
                    "fn" => {
                        let mut nested_bound = bound.clone();
                        if let Some(params) = args.first() {
                            collect_specialization_pattern_bindings(params, &mut nested_bound);
                        }
                        for body in args.iter().skip(1) {
                            collect_free_symbols(body, &mut nested_bound, free);
                        }
                        return;
                    }
                    "let" => {
                        let mut nested_bound = bound.clone();
                        if let Some(bindings) = args.first() {
                            if let ExprKind::Vector(items) = &bindings.kind {
                                for pair in items.chunks(2) {
                                    if let Some(value) = pair.get(1) {
                                        collect_free_symbols(value, &mut nested_bound, free);
                                    }
                                    if let Some(pattern) = pair.first() {
                                        collect_specialization_pattern_bindings(
                                            pattern,
                                            &mut nested_bound,
                                        );
                                    }
                                }
                            }
                        }
                        for body in args.iter().skip(1) {
                            collect_free_symbols(body, &mut nested_bound, free);
                        }
                        return;
                    }
                    _ if known_compiler_symbol(name) => {
                        for arg in args {
                            collect_free_symbols(arg, bound, free);
                        }
                        return;
                    }
                    _ => {}
                }
            }
            collect_free_symbols(head, bound, free);
            for arg in args {
                collect_free_symbols(arg, bound, free);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_free_symbols(item, bound, free);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                if !matches!(
                    key.kind,
                    ExprKind::Keyword(_) | ExprKind::String(_) | ExprKind::Symbol(_)
                ) {
                    collect_free_symbols(key, bound, free);
                }
                collect_free_symbols(value, bound, free);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_free_symbols(inner, bound, free),
        ExprKind::HtmlTemplate(_) => {}
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_specialization_pattern_bindings(pattern: &Expr, bindings: &mut HashSet<String>) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name != "_" => {
            bindings.insert(symbol_root(name).to_string());
        }
        ExprKind::List(items) if items.first().is_some_and(|head| matches_symbol(head, "as")) => {
            if let Some(pattern) = items.get(1) {
                collect_specialization_pattern_bindings(pattern, bindings);
            }
            if let Some(alias) = items.get(2) {
                collect_specialization_pattern_bindings(alias, bindings);
            }
        }
        ExprKind::List(items) | ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_specialization_pattern_bindings(item, bindings);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_specialization_pattern_bindings(key, bindings);
                collect_specialization_pattern_bindings(value, bindings);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_specialization_pattern_bindings(inner, bindings)
        }
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_)
        | ExprKind::HtmlTemplate(_) => {}
    }
}

fn known_compiler_symbol(name: &str) -> bool {
    matches!(
        name,
        "fn" | "let"
            | "if"
            | "do"
            | "match"
            | "unsafe-cast"
            | "not"
            | "and"
            | "or"
            | "="
            | "identical?"
            | "+"
            | "-"
            | "*"
            | "/"
            | "<"
            | ">"
            | "<="
            | ">="
            | "%"
            | "mod"
            | "max"
            | "min"
            | "round"
            | "floor"
            | "ceil"
            | "abs"
            | "some?"
            | "nil?"
            | "number?"
            | "string?"
            | "bool?"
            | "vector?"
            | "list?"
            | "set?"
            | "map?"
            | "object?"
            | "count"
            | "empty?"
            | "get"
            | "object-get"
            | "first"
            | "second"
            | "nth"
            | "last"
            | "find"
            | "map"
            | "map-indexed"
            | "filter"
            | "any?"
            | "every?"
            | "reduce"
            | "reduce-indexed"
            | "conj"
            | "assoc"
            | "dissoc"
            | "merge"
            | "str"
            | "list"
            | "vector"
            | "set"
            | "slice"
            | "range"
            | "to-number"
            | "to-fixed"
            | "lower-case"
            | "trim"
            | "split"
            | "join"
            | "starts-with?"
            | "ends-with?"
            | "includes?"
            | "contains?"
            | "json-stringify"
            | "json-parse"
    )
}

fn symbol_root(name: &str) -> &str {
    name.split_once('.').map(|(root, _)| root).unwrap_or(name)
}

fn type_fn_param_types_for_schema(schema: &str) -> Option<Vec<String>> {
    let schema = schema.trim();
    if !schema.starts_with("(Fn ") {
        return None;
    }
    let open = schema.find('[')?;
    let close = find_matching_delimiter(schema, open, '[', ']')?;
    Some(split_top_level_type_terms(&schema[open + 1..close]))
}

fn collect_type_substitutions(
    pattern: &str,
    concrete: &str,
    substitutions: &mut BTreeMap<String, String>,
) -> bool {
    let pattern = pattern.trim();
    let concrete = concrete.trim();
    if pattern == concrete {
        return true;
    }
    if is_type_variable_name(pattern) {
        if type_contains_variables(concrete) {
            return true;
        }
        match substitutions.get(pattern) {
            Some(existing) => existing == concrete,
            None => {
                substitutions.insert(pattern.to_string(), concrete.to_string());
                true
            }
        }
    } else if let (Some(pattern_fields), Some(concrete_fields)) =
        (type_record_fields(pattern), type_record_fields(concrete))
    {
        pattern_fields.iter().all(|(field, field_type)| {
            concrete_fields.get(field).is_some_and(|concrete_type| {
                collect_type_substitutions(field_type, concrete_type, substitutions)
            })
        })
    } else if let (Some((pattern_head, pattern_args)), Some((concrete_head, concrete_args))) =
        (type_app_parts(pattern), type_app_parts(concrete))
    {
        pattern_head == concrete_head
            && pattern_args.len() == concrete_args.len()
            && pattern_args
                .iter()
                .zip(concrete_args.iter())
                .all(|(pattern_arg, concrete_arg)| {
                    collect_type_substitutions(pattern_arg, concrete_arg, substitutions)
                })
    } else {
        !type_contains_variables(pattern)
    }
}

fn substitute_type_variables(schema: &str, substitutions: &BTreeMap<String, String>) -> String {
    let mut output = String::new();
    let mut token = String::new();
    for ch in schema.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            push_substituted_type_token(&mut output, &mut token, substitutions);
            output.push(ch);
        }
    }
    push_substituted_type_token(&mut output, &mut token, substitutions);
    output
}

fn push_substituted_type_token(
    output: &mut String,
    token: &mut String,
    substitutions: &BTreeMap<String, String>,
) {
    if token.is_empty() {
        return;
    }
    if let Some(replacement) = substitutions.get(token) {
        output.push_str(replacement);
    } else {
        output.push_str(token);
    }
    token.clear();
}

fn type_contains_variables(schema: &str) -> bool {
    let mut token = String::new();
    for ch in schema.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            if is_type_variable_name(&token) {
                return true;
            }
            token.clear();
        }
    }
    is_type_variable_name(&token)
}

fn is_type_variable_name(value: &str) -> bool {
    value
        .strip_prefix('t')
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
}

fn type_app_parts(schema: &str) -> Option<(String, Vec<String>)> {
    let schema = schema.trim();
    let inner = schema.strip_prefix('(')?.strip_suffix(')')?;
    let terms = split_top_level_type_terms(inner);
    let (head, args) = terms.split_first()?;
    Some((head.clone(), args.to_vec()))
}

fn type_record_fields(schema: &str) -> Option<BTreeMap<String, String>> {
    let schema = schema.trim();
    let inner = schema.strip_prefix('{')?.strip_suffix('}')?;
    let terms = split_top_level_type_terms(inner);
    if terms.len() % 2 != 0 {
        return None;
    }
    let mut fields = BTreeMap::new();
    for pair in terms.chunks(2) {
        fields.insert(pair[0].clone(), pair[1].clone());
    }
    Some(fields)
}

fn split_top_level_type_terms(value: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut start = None;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ch if ch.is_whitespace()
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                if let Some(term_start) = start.take() {
                    terms.push(value[term_start..index].to_string());
                }
                continue;
            }
            _ => {}
        }
        if start.is_none() && !ch.is_whitespace() {
            start = Some(index);
        }
    }
    if let Some(term_start) = start {
        terms.push(value[term_start..].trim().to_string());
    }
    terms.into_iter().filter(|term| !term.is_empty()).collect()
}

fn find_matching_delimiter(
    value: &str,
    open_index: usize,
    open: char,
    close: char,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in value
        .char_indices()
        .skip_while(|(index, _)| *index < open_index)
    {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn sanitize_js_identifier(name: &str) -> String {
    let mut output = String::new();
    for (index, ch) in name.chars().enumerate() {
        let valid = ch.is_ascii_alphanumeric() || ch == '_';
        if index == 0 && ch.is_ascii_digit() {
            output.push('_');
        }
        output.push(if valid { ch } else { '_' });
    }
    if output.is_empty() {
        "_".to_string()
    } else {
        output
    }
}

fn message_imports_from_imports(
    path: &Path,
    imports: &[ImportSpec],
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<HashMap<String, BTreeSet<String>>, String> {
    let mut imported_messages = HashMap::new();
    for import in imports {
        if !is_closkell_import_path(&import.path) {
            continue;
        }
        let import_source = resolve_import_source(path, &import.path)?;
        let canonical = fs::canonicalize(&import_source)
            .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
        let Some(imported) = modules.get(&canonical) else {
            continue;
        };
        for name in &import.names {
            if let Some(kinds) = imported.message_kinds_by_binding.get(&name.imported) {
                imported_messages.insert(name.name.clone(), kinds.clone());
            }
        }
    }
    Ok(imported_messages)
}

struct AppRuntimeRegistration {
    import_name: &'static str,
    kinds: &'static [&'static str],
}

const APP_RUNTIME_REGISTRATIONS: &[AppRuntimeRegistration] = &[
    AppRuntimeRegistration {
        import_name: "registerCompiledAnimationCommandHandlers",
        kinds: &["animation/frame", "animation/cancel"],
    },
    AppRuntimeRegistration {
        import_name: "registerAuthStorageCommandHandlers",
        kinds: &["auth-storage/persist", "auth-storage/load"],
    },
    AppRuntimeRegistration {
        import_name: "registerBluetoothCommandHandlers",
        kinds: &["bluetooth/request-device"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledBluetoothHeartRateCommandHandlers",
        kinds: &[
            "bluetooth/connect-heart-rate",
            "bluetooth/disconnect",
            "sub/bluetooth/connect-heart-rate",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerBrowserCommandHandlers",
        kinds: &[
            "browser/history-replace-search-param",
            "browser/history-write-route",
            "browser/theme-load",
            "browser/theme-apply",
            "browser/clipboard-write",
            "browser/set-cookie",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledCanvasDrawCommandHandlers",
        kinds: &["canvas/draw"],
    },
    AppRuntimeRegistration {
        import_name: "registerCanvasMeasureTextCommandHandlers",
        kinds: &["canvas/measure-text"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDomRefCommandHandlers",
        kinds: &["dom-ref/focus", "dom-ref/click", "dom-ref/measure"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDomResizeCommandHandlers",
        kinds: &[
            "dom-ref/resize-watch",
            "dom-ref/resize-unwatch",
            "sub/dom-ref/resize",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDirectDomResizeCommandHandlers",
        kinds: &["dom-ref/resize-watch/direct"],
    },
    AppRuntimeRegistration {
        import_name: "registerDomScrollCommandHandlers",
        kinds: &["dom/scroll-into-view"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledFileDownloadCommandHandlers",
        kinds: &["file/download"],
    },
    AppRuntimeRegistration {
        import_name: "registerFileImportCommandHandlers",
        kinds: &["file/import"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledFileReadSelectedCommandHandlers",
        kinds: &["file/read-selected"],
    },
    AppRuntimeRegistration {
        import_name: "registerHttpCommandHandlers",
        kinds: &["http/request"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledMediaQueryCommandHandlers",
        kinds: &[
            "media-query/watch",
            "media-query/unwatch",
            "sub/media-query",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDirectMediaQueryCommandHandlers",
        kinds: &["media-query/watch/direct", "media-query/unwatch/direct"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledRandomCommandHandlers",
        kinds: &["random/number"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledSimulationCommandHandlers",
        kinds: &["simulation/heart-rate", "sub/simulation/heart-rate"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledSimulationStopCommandHandlers",
        kinds: &["simulation/stop"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledStorageReadWriteCommandHandlers",
        kinds: &["storage/get", "storage/set"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledStorageRemoveCommandHandlers",
        kinds: &["storage/remove"],
    },
    AppRuntimeRegistration {
        import_name: "registerTaskCommandHandlers",
        kinds: &["task/perform"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledTimerCommandHandlers",
        kinds: &[
            "timer/after",
            "timer/every",
            "timer/cancel",
            "sub/timer/every",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledTimeCommandHandlers",
        kinds: &["time/now"],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledWindowEventCommandHandlers",
        kinds: &[
            "window/event-watch",
            "window/event-unwatch",
            "sub/window/event",
        ],
    },
    AppRuntimeRegistration {
        import_name: "registerCompiledDirectWindowEventCommandHandlers",
        kinds: &["window/event-watch/direct", "window/event-unwatch/direct"],
    },
];

fn collect_app_runtime_registrations(
    entry_effects: &BTreeSet<String>,
    artifacts: &[BuildArtifact],
) -> BTreeSet<&'static str> {
    let mut registrations = BTreeSet::new();
    collect_app_runtime_registrations_from_effects(entry_effects, &mut registrations);
    for artifact in artifacts {
        collect_app_runtime_registrations_from_effects(
            &artifact.runtime_effects,
            &mut registrations,
        );
    }
    if registrations.contains("registerCompiledSimulationCommandHandlers") {
        registrations.remove("registerCompiledSimulationStopCommandHandlers");
    }
    registrations
}

fn collect_app_runtime_registrations_from_effects(
    effects: &BTreeSet<String>,
    registrations: &mut BTreeSet<&'static str>,
) {
    for registration in APP_RUNTIME_REGISTRATIONS {
        if registration
            .kinds
            .iter()
            .any(|kind| effects.contains(*kind))
        {
            registrations.insert(registration.import_name);
        }
    }
}

fn app_runtime_registration_alias(import_name: &str) -> String {
    let mut chars = import_name.chars();
    let Some(first) = chars.next() else {
        return "__closkellRegister".to_string();
    };
    format!("__closkell{}{}", first.to_ascii_uppercase(), chars.as_str())
}

fn wrap_app_module(
    emitted: &mut js_backend::EmitResult,
    options: &AppOptions,
    runtime_registrations: &BTreeSet<&'static str>,
    init_takes_boot: bool,
    has_subscriptions: bool,
) {
    let prelude = app_bootstrap_prelude(options, runtime_registrations, has_subscriptions);
    let postlude = app_bootstrap_postlude(
        options,
        runtime_registrations,
        init_takes_boot,
        has_subscriptions,
    );
    let inserted_lines = prelude.lines().count();
    for mapping in &mut emitted.source_mappings {
        mapping.generated_line += inserted_lines;
    }
    strip_app_entry_exports(&mut emitted.code);
    if !emitted.code.ends_with('\n') {
        emitted.code.push('\n');
    }
    emitted.code = format!("{}{}{}", prelude, emitted.code, postlude);
}

fn strip_app_entry_exports(code: &mut String) {
    *code = code
        .replace("export function ", "function ")
        .replace("export const ", "const ");
}

fn app_bootstrap_prelude(
    options: &AppOptions,
    runtime_registrations: &BTreeSet<&'static str>,
    has_subscriptions: bool,
) -> String {
    let mut code = String::new();
    let app_runner = if has_subscriptions {
        "startCompiledApp"
    } else {
        "startCompiledAppWithoutSubscriptions"
    };
    let mut imports = vec![format!("{} as __closkellStartApp", app_runner)];
    for import_name in runtime_registrations {
        imports.push(format!(
            "{} as {}",
            import_name,
            app_runtime_registration_alias(import_name)
        ));
    }
    if !imports.is_empty() {
        code.push_str("import { ");
        code.push_str(&imports.join(", "));
        code.push_str(" } from \"@closkell/runtime\";\n");
    }
    if let Some(css) = &options.css {
        code.push_str("import ");
        code.push_str(&json_string(css));
        code.push_str(";\n");
    }
    code
}

fn app_bootstrap_postlude(
    options: &AppOptions,
    runtime_registrations: &BTreeSet<&'static str>,
    init_takes_boot: bool,
    has_subscriptions: bool,
) -> String {
    let mut code = String::new();
    code.push_str("const __closkellRoot = document.getElementById(");
    code.push_str(&json_string(&options.root_id));
    code.push_str(");\n");
    code.push_str("if (!__closkellRoot) {\n");
    code.push_str("  throw new Error(");
    code.push_str(&json_string(&format!(
        "Root element #{} was not found.",
        options.root_id
    )));
    code.push_str(");\n");
    code.push_str("}\n");
    code.push_str(
        "const __closkellHandlerContext = { env: {}, host: globalThis, disposers: [] };\n",
    );
    code.push_str("const __closkellHandlers = {};\n");
    let registrations = runtime_registrations
        .iter()
        .map(|import_name| app_runtime_registration_alias(import_name))
        .collect::<Vec<_>>();
    for registration in registrations {
        code.push_str(&registration);
        code.push_str("(__closkellHandlers, __closkellHandlerContext);\n");
    }
    code.push_str("Object.defineProperty(__closkellHandlers, \"dispose\", { value() { for (const dispose of __closkellHandlerContext.disposers.splice(0)) dispose(); } });\n");
    code.push_str("export const __closkellApp = __closkellStartApp({\n");
    code.push_str("  root: __closkellRoot,\n");
    code.push_str("  init,\n");
    code.push_str("  update,\n");
    code.push_str("  view,\n");
    if init_takes_boot {
        code.push_str("  boot: { currentUrl: globalThis.location?.href ?? \"\" },\n");
    }
    if has_subscriptions {
        code.push_str("  subscriptions,\n");
    }
    code.push_str("  handlers: __closkellHandlers\n");
    code.push_str("});\n\n");
    code
}

fn emitted_has_binding(code: &str, name: &str) -> bool {
    code.contains(&format!("export function {}(", name))
        || code.contains(&format!("export const {} =", name))
        || code.contains(&format!("export let {} =", name))
        || code.contains(&format!("export var {} =", name))
}

fn emitted_init_takes_boot(code: &str) -> bool {
    let Some(start) = code.find("export function init(") else {
        return false;
    };
    let params_start = start + "export function init(".len();
    let Some(params_end) = code[params_start..].find(')') else {
        return false;
    };
    !code[params_start..params_start + params_end]
        .trim()
        .is_empty()
}

fn runtime_vendor_root(output: &Path) -> PathBuf {
    let start = output
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let resolved = fs::canonicalize(&start).unwrap_or(start);
    for ancestor in resolved.ancestors() {
        if ancestor.join("package.json").is_file() {
            return ancestor.to_path_buf();
        }
    }
    resolved
}

fn with_source_mapping_url(mut code: String, output: &Path) -> String {
    if !code.ends_with('\n') {
        code.push('\n');
    }
    code.push_str("//# sourceMappingURL=");
    code.push_str(&source_map_file_name(output));
    code.push('\n');
    code
}

fn write_source_map(
    source_path: &Path,
    input: &str,
    output: &Path,
    mappings: &[js_backend::SourceMapping],
) -> Result<(), String> {
    let map_path = source_map_path(output);
    let json = source_map_json(source_path, input, output, mappings);
    fs::write(&map_path, json)
        .map_err(|err| format!("failed to write {}: {}", map_path.display(), err))
}

fn source_map_json(
    source_path: &Path,
    input: &str,
    output: &Path,
    mappings: &[js_backend::SourceMapping],
) -> String {
    format!(
        "{{\n  \"version\": 3,\n  \"file\": {},\n  \"sources\": [{}],\n  \"sourcesContent\": [{}],\n  \"names\": [],\n  \"mappings\": {}\n}}\n",
        json_string(&output_file_name(output)),
        json_string(&source_name_for_map(source_path)),
        json_string(input),
        json_string(&source_map_mappings(input, mappings))
    )
}

fn source_map_mappings(input: &str, mappings: &[js_backend::SourceMapping]) -> String {
    let mut mappings = mappings.to_vec();
    mappings.sort_by_key(|mapping| (mapping.generated_line, mapping.generated_column));

    let mut output = String::new();
    let mut current_generated_line = 0_usize;
    let mut first_segment_on_line = true;
    let mut previous_generated_column = 0_i64;
    let mut previous_source_index = 0_i64;
    let mut previous_original_line = 0_i64;
    let mut previous_original_column = 0_i64;

    for mapping in mappings {
        while current_generated_line < mapping.generated_line {
            output.push(';');
            current_generated_line += 1;
            first_segment_on_line = true;
            previous_generated_column = 0;
        }
        if !first_segment_on_line {
            output.push(',');
        }

        let (original_line, original_column) = syntax::line_column(input, mapping.source_offset);
        let generated_column = mapping.generated_column as i64;
        let source_index = 0_i64;
        let original_line = original_line.saturating_sub(1) as i64;
        let original_column = original_column.saturating_sub(1) as i64;

        output.push_str(&source_map_segment(&[
            generated_column - previous_generated_column,
            source_index - previous_source_index,
            original_line - previous_original_line,
            original_column - previous_original_column,
        ]));

        previous_generated_column = generated_column;
        previous_source_index = source_index;
        previous_original_line = original_line;
        previous_original_column = original_column;
        first_segment_on_line = false;
    }

    output
}

fn source_map_segment(values: &[i64]) -> String {
    values
        .iter()
        .map(|value| source_map_vlq(*value))
        .collect::<Vec<_>>()
        .join("")
}

fn source_map_vlq(value: i64) -> String {
    const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut value = if value < 0 {
        ((-value) << 1) + 1
    } else {
        value << 1
    } as u64;
    let mut encoded = String::new();

    loop {
        let mut digit = value & 0b1_1111;
        value >>= 5;
        if value > 0 {
            digit |= 0b10_0000;
        }
        encoded.push(BASE64[digit as usize] as char);
        if value == 0 {
            break;
        }
    }

    encoded
}

fn source_map_path(output: &Path) -> PathBuf {
    output.with_file_name(source_map_file_name(output))
}

fn source_map_file_name(output: &Path) -> String {
    format!("{}.map", output_file_name(output))
}

fn output_file_name(output: &Path) -> String {
    output
        .file_name()
        .map(|name| name.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "out.mjs".to_string())
}

fn source_name_for_map(source_path: &Path) -> String {
    fs::canonicalize(source_path)
        .unwrap_or_else(|_| source_path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Clone, Debug)]
struct ImportSpec {
    path: String,
    names: Vec<ImportName>,
}

#[derive(Clone, Debug)]
struct ImportName {
    imported: String,
    name: String,
    default: bool,
    span: syntax::Span,
}

fn parse_imports(input: &str, source: &SourceFile) -> Result<Vec<ImportSpec>, String> {
    let mut imports = Vec::new();
    for form in &source.forms {
        if let Some(import) = parse_import_form(form) {
            match import {
                Ok(import) => imports.push(import),
                Err(diagnostic) => {
                    println!("{}", render_diagnostics(input, &[diagnostic]));
                    return Err("build failed during import parsing".to_string());
                }
            }
        }
    }
    Ok(imports)
}

fn parse_import_form(expr: &Expr) -> Option<Result<ImportSpec, Diagnostic>> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    if !items
        .first()
        .is_some_and(|head| matches!(&head.kind, ExprKind::Symbol(name) if name == "import"))
    {
        return None;
    }
    if items.len() != 3 {
        return Some(Err(Diagnostic::error(
            expr.span,
            "import expects a path string and a vector of symbols",
        )));
    }
    let ExprKind::String(path) = &items[1].kind else {
        return Some(Err(Diagnostic::error(
            items[1].span,
            "import path must be a string",
        )));
    };
    let ExprKind::Vector(names) = &items[2].kind else {
        return Some(Err(Diagnostic::error(
            items[2].span,
            "import names must be a vector",
        )));
    };
    if names.is_empty() {
        return Some(Err(Diagnostic::error(
            items[2].span,
            "import names vector cannot be empty",
        )));
    }
    let mut imported = Vec::new();
    for name in names {
        match parse_import_name(name) {
            Ok(parsed) => imported.push(parsed),
            Err(diagnostic) => return Some(Err(diagnostic)),
        }
    }

    Some(Ok(ImportSpec {
        path: path.clone(),
        names: imported,
    }))
}

fn parse_import_name(expr: &Expr) -> Result<ImportName, Diagnostic> {
    match &expr.kind {
        ExprKind::Symbol(symbol) => Ok(ImportName {
            imported: symbol.clone(),
            name: symbol.clone(),
            default: false,
            span: expr.span,
        }),
        ExprKind::List(items)
            if items.len() == 2
                && matches!(&items[0].kind, ExprKind::Symbol(name) if name == "default") =>
        {
            let ExprKind::Symbol(local) = &items[1].kind else {
                return Err(Diagnostic::error(
                    items[1].span,
                    "default import local name must be a symbol",
                ));
            };
            Ok(ImportName {
                imported: "default".to_string(),
                name: local.clone(),
                default: true,
                span: expr.span,
            })
        }
        ExprKind::List(items)
            if items.len() == 3
                && matches!(&items[1].kind, ExprKind::Symbol(name) if name == "as") =>
        {
            let ExprKind::Symbol(imported) = &items[0].kind else {
                return Err(Diagnostic::error(
                    items[0].span,
                    "aliased import name must be a symbol",
                ));
            };
            let ExprKind::Symbol(local) = &items[2].kind else {
                return Err(Diagnostic::error(
                    items[2].span,
                    "aliased import local name must be a symbol",
                ));
            };
            Ok(ImportName {
                imported: imported.clone(),
                name: local.clone(),
                default: false,
                span: expr.span,
            })
        }
        _ => Err(Diagnostic::error(
            expr.span,
            "imported name must be a symbol, (default local), or (name as local)",
        )),
    }
}

fn resolve_import_source(source_path: &Path, import_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(import_path);
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "only same-tree relative local imports are supported: {}",
            import_path
        ));
    }
    if relative.extension().and_then(|value| value.to_str()) != Some("clsk") {
        return Err(format!(
            "local import must reference a .clsk source file: {}",
            import_path
        ));
    }

    let parent = source_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(relative))
}

fn is_closkell_import_path(import_path: &str) -> bool {
    Path::new(import_path)
        .extension()
        .and_then(|value| value.to_str())
        == Some("clsk")
}

fn output_for_import(output: &Path, import_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(import_path).with_extension("mjs");
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(relative))
}

fn collect_exports(source: &SourceFile) -> HashSet<String> {
    source
        .forms
        .iter()
        .filter_map(|form| {
            let ExprKind::List(items) = &form.kind else {
                return None;
            };
            if let [head, name, _] = items.as_slice() {
                if matches_symbol(head, "def") {
                    if let ExprKind::Symbol(name) = &name.kind {
                        return Some(name.clone());
                    }
                }
            }
            if items.len() >= 4 && matches_symbol(&items[0], "defn") {
                if let ExprKind::Symbol(name) = &items[1].kind {
                    return Some(name.clone());
                }
            }
            if let [head, name, _] = items.as_slice() {
                if matches_symbol(head, "type") {
                    if let ExprKind::Symbol(name) = &name.kind {
                        return Some(name.clone());
                    }
                }
            }
            None
        })
        .collect()
}

fn require_app_exports(path: &Path, exports: &HashSet<String>) -> Result<(), String> {
    let missing = ["init", "update", "view"]
        .into_iter()
        .filter(|name| !exports.contains(*name))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "build --app expects {} to export {}; missing {}",
        path.display(),
        "init, update, and view",
        missing.join(", ")
    ))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CommandShape {
    kind: String,
    fields: Vec<String>,
    sources: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct CommandShapeData {
    fields: BTreeSet<String>,
    sources: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct UnsafeCastInfo {
    target: String,
    expr: String,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Clone, Debug)]
struct TestCaseInfo {
    name: String,
    group: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct UnusedReport {
    top_level: Vec<UnusedTopLevelInfo>,
    imports: Vec<UnusedImportInfo>,
}

#[derive(Clone, Debug)]
struct UnusedTopLevelInfo {
    name: String,
    kind: String,
    annotated: bool,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Clone, Debug)]
struct UnusedImportInfo {
    name: String,
    imported: String,
    path: String,
    default: bool,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Clone, Debug)]
struct TopLevelDefInfo {
    name: String,
    kind: String,
    annotated: bool,
    span: syntax::Span,
    deps: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ChangedPathSummary {
    source: String,
    operation: String,
    path: String,
    expr: String,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

fn inspect_file(path: &Path, modules: &HashMap<PathBuf, ModuleInfo>) -> Result<String, String> {
    let (input, source) = parse_file(path)?;
    let imports = parse_imports(&input, &source)?;
    let imported_macros = imported_macros_from_imports(path, &imports, modules)?;
    let imported_command_shapes = imported_command_shapes_from_imports(path, &imports, modules)?;
    let expansion = macro_expand::expand_source_with_imported_macros(&source, &imported_macros);
    if !expansion.diagnostics.is_empty() {
        return Err(format!(
            "inspect failed during macro expansion: {}",
            path.display()
        ));
    }

    let type_report = typecheck::collect_type_declarations(&expansion.source);
    if !type_report.diagnostics.is_empty() {
        println!("{}", render_diagnostics(&input, &type_report.diagnostics));
        return Err(format!(
            "inspect failed during type declaration parsing: {}",
            path.display()
        ));
    }
    let annotation_report = typecheck::collect_type_annotations(&expansion.source);
    if !annotation_report.diagnostics.is_empty() {
        println!(
            "{}",
            render_diagnostics(&input, &annotation_report.diagnostics)
        );
        return Err(format!(
            "inspect failed during type annotation parsing: {}",
            path.display()
        ));
    }
    let foreign_report = typecheck::collect_foreign_declarations(&expansion.source);
    if !foreign_report.diagnostics.is_empty() {
        println!(
            "{}",
            render_diagnostics(&input, &foreign_report.diagnostics)
        );
        return Err(format!(
            "inspect failed during foreign declaration parsing: {}",
            path.display()
        ));
    }

    let mut exports = collect_exports(&expansion.source);
    exports.extend(
        macro_expand::collect_macro_defs(&source)
            .macros
            .keys()
            .cloned(),
    );
    let mut exports = exports.into_iter().collect::<Vec<_>>();
    exports.sort();
    let templates = template_ir::lower_named_templates(&expansion.source);
    let mut commands = command_shape_map(&collect_command_shapes(&expansion.source));
    merge_command_shapes(&mut commands, imported_command_shapes);
    let commands = command_shapes_from_map(commands);
    let subscriptions = collect_subscription_shapes(&expansion.source);
    let public_signatures = public_signatures_for(path, modules)?;
    let unsafe_casts = collect_unsafe_casts(&input, &expansion.source);
    let tests = collect_test_cases(&expansion.source);
    let changed_path_summaries = collect_changed_path_summaries(&input, &expansion.source);
    let unused = collect_unused_report(
        &input,
        &source,
        &expansion.source,
        &imports,
        &annotation_report.annotations,
    );
    Ok(render_inspection_json(
        path,
        &imports,
        &exports,
        &public_signatures,
        &type_report.declarations,
        &annotation_report.annotations,
        &foreign_report.declarations,
        &templates,
        &commands,
        &subscriptions,
        &changed_path_summaries,
        &unsafe_casts,
        &tests,
        &unused,
    ))
}

fn imported_macros_from_imports(
    path: &Path,
    imports: &[ImportSpec],
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<HashMap<String, macro_expand::MacroDef>, String> {
    let mut macros = HashMap::new();
    for import in imports {
        if !is_closkell_import_path(&import.path) {
            continue;
        }
        let import_source = resolve_import_source(path, &import.path)?;
        let canonical = fs::canonicalize(&import_source)
            .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
        let Some(imported) = modules.get(&canonical) else {
            continue;
        };
        for name in &import.names {
            if let Some(macro_def) = imported.macros.get(&name.imported) {
                macros.insert(name.name.clone(), macro_def.clone());
            }
        }
    }
    Ok(macros)
}

fn imported_command_shapes_from_imports(
    path: &Path,
    imports: &[ImportSpec],
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<Vec<CommandShape>, String> {
    let mut shapes = BTreeMap::new();
    for import in imports {
        if !is_closkell_import_path(&import.path) {
            continue;
        }
        let import_source = resolve_import_source(path, &import.path)?;
        let canonical = fs::canonicalize(&import_source)
            .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
        let Some(imported) = modules.get(&canonical) else {
            continue;
        };
        for name in &import.names {
            if let Some(binding_shapes) = imported.command_shapes_by_binding.get(&name.imported) {
                merge_command_shapes(&mut shapes, binding_shapes.iter().cloned());
            }
        }
    }
    Ok(command_shapes_from_map(shapes))
}

fn render_inspection_json(
    path: &Path,
    imports: &[ImportSpec],
    exports: &[String],
    public_signatures: &[typecheck::ExportedBinding],
    types: &[typecheck::TypeDeclaration],
    annotations: &[typecheck::TypeAnnotation],
    foreigns: &[typecheck::ForeignDeclaration],
    templates: &[NamedTemplate],
    commands: &[CommandShape],
    subscriptions: &[CommandShape],
    changed_path_summaries: &[ChangedPathSummary],
    unsafe_casts: &[UnsafeCastInfo],
    tests: &[TestCaseInfo],
    unused: &UnusedReport,
) -> String {
    let mut lines = Vec::new();
    lines.push("{".to_string());
    lines.push(format!(
        "  \"file\": {},",
        json_string(&path.display().to_string())
    ));
    lines.push(format!("  \"imports\": {},", imports_json(path, imports)));
    lines.push(format!("  \"exports\": {},", json_string_array(exports)));
    lines.push(format!(
        "  \"publicSignatures\": {},",
        public_signatures_json(public_signatures)
    ));
    lines.push(format!("  \"types\": {},", types_json(types)));
    lines.push(format!(
        "  \"annotations\": {},",
        annotations_json(annotations)
    ));
    lines.push(format!(
        "  \"commandLogSchema\": {},",
        command_shapes_json(commands)
    ));
    lines.push(format!(
        "  \"subscriptionSchema\": {},",
        command_shapes_json(subscriptions)
    ));
    lines.push(format!("  \"jsInterop\": {},", foreigns_json(foreigns)));
    lines.push(format!(
        "  \"componentGraph\": {},",
        component_graph_json(templates)
    ));
    lines.push(format!(
        "  \"statePathToSlots\": {},",
        state_path_to_slots_json(templates)
    ));
    lines.push(format!(
        "  \"changedPathSummaries\": {},",
        changed_path_summaries_json(changed_path_summaries)
    ));
    lines.push(format!(
        "  \"unsafeCasts\": {},",
        unsafe_casts_json(unsafe_casts)
    ));
    lines.push(format!("  \"tests\": {},", tests_json(tests)));
    lines.push(format!("  \"unused\": {},", unused_report_json(unused)));
    lines.push(format!("  \"templates\": {}", templates_json(templates)));
    lines.push("}".to_string());
    lines.join("\n")
}

fn public_signatures_for(
    path: &Path,
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<Vec<typecheck::ExportedBinding>, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    let mut bindings = modules
        .get(&canonical)
        .map(|module| module.bindings.clone())
        .unwrap_or_default();
    bindings.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(bindings)
}

fn imports_json(path: &Path, imports: &[ImportSpec]) -> String {
    let entries = imports
        .iter()
        .map(|import| {
            let kind = if is_closkell_import_path(&import.path) {
                "closkell"
            } else {
                "js"
            };
            let resolved = if is_closkell_import_path(&import.path) {
                resolve_import_source(path, &import.path)
                    .ok()
                    .and_then(|resolved| fs::canonicalize(resolved).ok())
                    .map(|resolved| cache_path_string(&resolved))
            } else {
                None
            };
            format!(
                "{{\"path\":{},\"kind\":{},\"resolved\":{},\"names\":{}}}",
                json_string(&import.path),
                json_string(kind),
                optional_json_string(resolved.as_deref()),
                import_names_json(&import.names)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn import_names_json(names: &[ImportName]) -> String {
    let entries = names
        .iter()
        .map(|name| {
            format!(
                "{{\"imported\":{},\"local\":{},\"default\":{}}}",
                json_string(&name.imported),
                json_string(&name.name),
                name.default
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn public_signatures_json(bindings: &[typecheck::ExportedBinding]) -> String {
    let entries = bindings
        .iter()
        .map(|binding| {
            format!(
                "{{\"name\":{},\"schema\":{},\"annotated\":{}}}",
                json_string(&binding.name),
                json_string(&binding.schema()),
                binding.is_annotated()
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn types_json(types: &[typecheck::TypeDeclaration]) -> String {
    let entries = types
        .iter()
        .map(|ty| {
            format!(
                "{{\"name\":{},\"params\":{},\"schema\":{}}}",
                json_string(&ty.name),
                json_string_array(&ty.params),
                json_string(&ty.schema)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn annotations_json(annotations: &[typecheck::TypeAnnotation]) -> String {
    let entries = annotations
        .iter()
        .map(|annotation| {
            format!(
                "{{\"name\":{},\"schema\":{}}}",
                json_string(&annotation.name),
                json_string(&annotation.schema)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn foreigns_json(foreigns: &[typecheck::ForeignDeclaration]) -> String {
    let entries = foreigns
        .iter()
        .map(|foreign| {
            format!(
                "{{\"mode\":{},\"name\":{},\"schema\":{}}}",
                json_string(&foreign.mode),
                json_string(&foreign.name),
                json_string(&foreign.schema)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn unsafe_casts_json(casts: &[UnsafeCastInfo]) -> String {
    let entries = casts
        .iter()
        .map(|cast| {
            format!(
                "{{\"target\":{},\"expr\":{},\"span\":{{\"start\":{},\"end\":{}}},\"range\":{{\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}}}",
                json_string(&cast.target),
                json_string(&cast.expr),
                cast.start,
                cast.end,
                cast.line,
                cast.column,
                cast.end_line,
                cast.end_column
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn tests_json(tests: &[TestCaseInfo]) -> String {
    let entries = tests
        .iter()
        .map(|test| {
            format!(
                "{{\"name\":{},\"group\":{},\"path\":{}}}",
                json_string(&test.name),
                json_string_array(&test.group),
                json_string_array(&test_path(&test.group, &test.name))
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn test_path(group: &[String], name: &str) -> Vec<String> {
    let mut path = group.to_vec();
    path.push(name.to_string());
    path
}

fn unused_report_json(report: &UnusedReport) -> String {
    format!(
        "{{\"topLevel\":{},\"imports\":{}}}",
        unused_top_level_json(&report.top_level),
        unused_imports_json(&report.imports)
    )
}

fn unused_top_level_json(items: &[UnusedTopLevelInfo]) -> String {
    let entries = items
        .iter()
        .map(|item| {
            format!(
                "{{\"name\":{},\"kind\":{},\"annotated\":{},\"span\":{{\"start\":{},\"end\":{}}},\"range\":{{\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}}}",
                json_string(&item.name),
                json_string(&item.kind),
                item.annotated,
                item.start,
                item.end,
                item.line,
                item.column,
                item.end_line,
                item.end_column
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn unused_imports_json(items: &[UnusedImportInfo]) -> String {
    let entries = items
        .iter()
        .map(|item| {
            format!(
                "{{\"name\":{},\"imported\":{},\"path\":{},\"default\":{},\"span\":{{\"start\":{},\"end\":{}}},\"range\":{{\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}}}",
                json_string(&item.name),
                json_string(&item.imported),
                json_string(&item.path),
                item.default,
                item.start,
                item.end,
                item.line,
                item.column,
                item.end_line,
                item.end_column
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn templates_json(templates: &[NamedTemplate]) -> String {
    let entries = templates
        .iter()
        .map(|template| {
            format!(
                "{{\"name\":{},\"nodes\":{},\"slots\":{}}}",
                json_string(&template.name),
                nodes_json(&template.template.nodes),
                slots_json(&template.name, &template.template.slots)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn collect_unused_report(
    input: &str,
    source: &SourceFile,
    expanded: &SourceFile,
    imports: &[ImportSpec],
    annotations: &[typecheck::TypeAnnotation],
) -> UnusedReport {
    let mut defs = collect_top_level_definitions(source, annotations);
    let local_names = defs.keys().cloned().collect::<BTreeSet<_>>();
    let import_names = imports
        .iter()
        .flat_map(|import| import.names.iter().map(|name| name.name.clone()))
        .collect::<BTreeSet<_>>();
    let visible_names = local_names
        .iter()
        .chain(import_names.iter())
        .cloned()
        .collect::<BTreeSet<_>>();

    merge_definition_dependencies(&mut defs, source, &visible_names);
    merge_definition_dependencies(&mut defs, expanded, &visible_names);
    merge_annotation_dependencies(&mut defs, annotations, &visible_names);

    let roots = unused_roots(&defs);
    let mut reachable = BTreeSet::new();
    let mut stack = roots.into_iter().collect::<Vec<_>>();
    while let Some(name) = stack.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(def) = defs.get(&name) else {
            continue;
        };
        for dep in &def.deps {
            if defs.contains_key(dep) && !reachable.contains(dep) {
                stack.push(dep.clone());
            } else if import_names.contains(dep) {
                reachable.insert(dep.clone());
            }
        }
    }

    let mut top_level = defs
        .values()
        .filter(|def| !reachable.contains(&def.name))
        .map(|def| {
            let (line, column) = line_column(input, def.span.start);
            let (end_line, end_column) = line_column(input, def.span.end);
            UnusedTopLevelInfo {
                name: def.name.clone(),
                kind: def.kind.clone(),
                annotated: def.annotated,
                start: def.span.start,
                end: def.span.end,
                line,
                column,
                end_line,
                end_column,
            }
        })
        .collect::<Vec<_>>();
    top_level.sort_by(|left, right| left.name.cmp(&right.name));

    let mut unused_imports = Vec::new();
    for import in imports {
        for name in &import.names {
            if reachable.contains(&name.name) {
                continue;
            }
            let (line, column) = line_column(input, name.span.start);
            let (end_line, end_column) = line_column(input, name.span.end);
            unused_imports.push(UnusedImportInfo {
                name: name.name.clone(),
                imported: name.imported.clone(),
                path: import.path.clone(),
                default: name.default,
                start: name.span.start,
                end: name.span.end,
                line,
                column,
                end_line,
                end_column,
            });
        }
    }
    unused_imports.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.name.cmp(&right.name))
    });

    UnusedReport {
        top_level,
        imports: unused_imports,
    }
}

fn collect_top_level_definitions(
    source: &SourceFile,
    annotations: &[typecheck::TypeAnnotation],
) -> BTreeMap<String, TopLevelDefInfo> {
    let annotated = annotations
        .iter()
        .map(|annotation| annotation.name.clone())
        .collect::<BTreeSet<_>>();
    let mut defs = BTreeMap::new();
    for form in &source.forms {
        let Some((name, kind, span)) = top_level_definition(form) else {
            continue;
        };
        defs.insert(
            name.clone(),
            TopLevelDefInfo {
                annotated: annotated.contains(&name),
                name,
                kind,
                span,
                deps: BTreeSet::new(),
            },
        );
    }
    defs
}

fn top_level_definition(expr: &Expr) -> Option<(String, String, syntax::Span)> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let head = items.first().and_then(symbol_name)?;
    match head {
        "def" if items.len() >= 3 => {
            let name = items.get(1).and_then(symbol_name)?;
            Some((name.to_string(), "def".to_string(), expr.span))
        }
        "defn" if items.len() >= 4 => {
            let name = items.get(1).and_then(symbol_name)?;
            Some((name.to_string(), "defn".to_string(), expr.span))
        }
        "defmacro" if items.len() >= 4 => {
            let name = items.get(1).and_then(symbol_name)?;
            Some((name.to_string(), "defmacro".to_string(), expr.span))
        }
        "type" if items.len() >= 3 => {
            let name = items.get(1).and_then(symbol_name)?;
            Some((name.to_string(), "type".to_string(), expr.span))
        }
        _ => None,
    }
}

fn unused_roots(defs: &BTreeMap<String, TopLevelDefInfo>) -> BTreeSet<String> {
    let has_app_roots = ["init", "update", "view"]
        .into_iter()
        .any(|name| defs.contains_key(name));
    if has_app_roots {
        return ["init", "update", "view", "subscriptions", "tests"]
            .into_iter()
            .filter(|name| defs.contains_key(*name))
            .map(str::to_string)
            .collect();
    }

    let mut roots = defs
        .values()
        .filter(|def| def.annotated)
        .map(|def| def.name.clone())
        .collect::<BTreeSet<_>>();
    if defs.contains_key("tests") {
        roots.insert("tests".to_string());
    }
    roots
}

fn merge_definition_dependencies(
    defs: &mut BTreeMap<String, TopLevelDefInfo>,
    source: &SourceFile,
    visible_names: &BTreeSet<String>,
) {
    for form in &source.forms {
        let Some((name, _, _)) = top_level_definition(form) else {
            continue;
        };
        let deps = definition_dependencies(form, visible_names);
        if let Some(def) = defs.get_mut(&name) {
            def.deps.extend(deps);
            def.deps.remove(&name);
        }
    }
}

fn merge_annotation_dependencies(
    defs: &mut BTreeMap<String, TopLevelDefInfo>,
    annotations: &[typecheck::TypeAnnotation],
    visible_names: &BTreeSet<String>,
) {
    for annotation in annotations {
        let Some(def) = defs.get_mut(&annotation.name) else {
            continue;
        };
        let source = parse_source(&annotation.schema);
        if let Some(form) = source.forms.first() {
            let mut refs = BTreeSet::new();
            collect_symbol_refs_expr(form, &BTreeSet::new(), &mut refs);
            def.deps
                .extend(refs.into_iter().filter(|name| visible_names.contains(name)));
            def.deps.remove(&annotation.name);
        }
    }
}

fn definition_dependencies(expr: &Expr, visible_names: &BTreeSet<String>) -> BTreeSet<String> {
    let ExprKind::List(items) = &expr.kind else {
        return BTreeSet::new();
    };
    let Some(head) = items.first().and_then(symbol_name) else {
        return BTreeSet::new();
    };
    let mut refs = BTreeSet::new();
    match head {
        "def" if items.len() >= 3 => {
            collect_symbol_refs_expr(&items[2], &BTreeSet::new(), &mut refs);
        }
        "defn" | "defmacro" if items.len() >= 4 => {
            let mut scope = BTreeSet::new();
            if let Some(params) = items.get(2) {
                collect_pattern_bindings(params, &mut scope);
            }
            for body in items.iter().skip(3) {
                collect_symbol_refs_expr(body, &scope, &mut refs);
            }
        }
        "type" if items.len() >= 3 => {
            let mut scope = BTreeSet::new();
            let schema_start = if items.len() > 3 {
                for param in &items[2..items.len() - 1] {
                    if let Some(name) = symbol_name(param) {
                        scope.insert(name.to_string());
                    }
                }
                items.len() - 1
            } else {
                2
            };
            for schema in items.iter().skip(schema_start) {
                collect_symbol_refs_expr(schema, &scope, &mut refs);
            }
        }
        _ => {}
    }
    refs.into_iter()
        .filter(|name| visible_names.contains(name))
        .collect()
}

fn collect_symbol_refs_expr(expr: &Expr, scope: &BTreeSet<String>, refs: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Symbol(name) => {
            let base = name.split('.').next().unwrap_or(name);
            if !scope.contains(name) && !scope.contains(base) {
                refs.insert(name.clone());
                if base != name {
                    refs.insert(base.to_string());
                }
            }
        }
        ExprKind::List(items) => collect_symbol_refs_list(items, scope, refs),
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_symbol_refs_expr(item, scope, refs);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                if !matches!(
                    key.kind,
                    ExprKind::Keyword(_) | ExprKind::String(_) | ExprKind::Symbol(_)
                ) {
                    collect_symbol_refs_expr(key, scope, refs);
                }
                collect_symbol_refs_expr(value, scope, refs);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_symbol_refs_expr(inner, scope, refs),
        ExprKind::HtmlTemplate(node) => collect_symbol_refs_html_node(node, scope, refs),
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_symbol_refs_list(items: &[Expr], scope: &BTreeSet<String>, refs: &mut BTreeSet<String>) {
    let Some(head) = items.first().and_then(symbol_name) else {
        for item in items {
            collect_symbol_refs_expr(item, scope, refs);
        }
        return;
    };

    match head {
        "fn" if items.len() >= 3 => {
            let mut inner_scope = scope.clone();
            collect_pattern_bindings(&items[1], &mut inner_scope);
            for body in items.iter().skip(2) {
                collect_symbol_refs_expr(body, &inner_scope, refs);
            }
        }
        "let" if items.len() >= 3 => {
            let mut inner_scope = scope.clone();
            if let ExprKind::Vector(bindings) = &items[1].kind {
                for pair in bindings.chunks(2) {
                    if let [pattern, value] = pair {
                        collect_symbol_refs_expr(value, &inner_scope, refs);
                        collect_pattern_bindings(pattern, &mut inner_scope);
                    }
                }
            } else {
                collect_symbol_refs_expr(&items[1], scope, refs);
            }
            for body in items.iter().skip(2) {
                collect_symbol_refs_expr(body, &inner_scope, refs);
            }
        }
        "match" if items.len() >= 2 => {
            collect_symbol_refs_expr(&items[1], scope, refs);
            let mut index = 2;
            while index + 1 < items.len() {
                let mut inner_scope = scope.clone();
                collect_pattern_bindings(&items[index], &mut inner_scope);
                collect_symbol_refs_expr(&items[index + 1], &inner_scope, refs);
                index += 2;
            }
        }
        "for" if items.len() >= 3 => {
            let mut inner_scope = scope.clone();
            if let ExprKind::Vector(bindings) = &items[1].kind {
                if let Some(pattern) = bindings.first() {
                    collect_pattern_bindings(pattern, &mut inner_scope);
                }
                if let Some(collection) = bindings.get(1) {
                    collect_symbol_refs_expr(collection, scope, refs);
                }
                if bindings.len() > 3 {
                    for extra in bindings.iter().skip(3) {
                        collect_symbol_refs_expr(extra, &inner_scope, refs);
                    }
                }
            } else {
                collect_symbol_refs_expr(&items[1], scope, refs);
            }
            for body in items.iter().skip(2) {
                collect_symbol_refs_expr(body, &inner_scope, refs);
            }
        }
        "def" | "defn" | "defmacro" | "type" | "ann" | "import" => {}
        _ => {
            for item in items {
                collect_symbol_refs_expr(item, scope, refs);
            }
        }
    }
}

fn collect_pattern_bindings(pattern: &Expr, scope: &mut BTreeSet<String>) {
    match &pattern.kind {
        ExprKind::Symbol(name) if name != "_" => {
            scope.insert(name.clone());
        }
        ExprKind::Vector(items) | ExprKind::Set(items) | ExprKind::List(items) => {
            for item in items {
                collect_pattern_bindings(item, scope);
            }
        }
        ExprKind::Map(entries) => {
            for (_, value) in entries {
                collect_pattern_bindings(value, scope);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_pattern_bindings(inner, scope),
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_)
        | ExprKind::HtmlTemplate(_) => {}
    }
}

fn collect_symbol_refs_html_node(
    node: &syntax::HtmlNode,
    scope: &BTreeSet<String>,
    refs: &mut BTreeSet<String>,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    let mut attr_scope = scope.clone();
                    if attr.name.starts_with("on:") {
                        attr_scope.insert("event".to_string());
                    }
                    collect_symbol_refs_expr(expr, &attr_scope, refs);
                }
            }
            for child in &element.children {
                collect_symbol_refs_html_node(child, scope, refs);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_symbol_refs_expr(expr, scope, refs),
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn collect_unsafe_casts(input: &str, source: &SourceFile) -> Vec<UnsafeCastInfo> {
    let mut casts = Vec::new();
    for form in &source.forms {
        collect_unsafe_casts_expr(input, form, &mut casts);
    }
    casts
}

fn collect_unsafe_casts_expr(input: &str, expr: &Expr, casts: &mut Vec<UnsafeCastInfo>) {
    match &expr.kind {
        ExprKind::List(items) => {
            if items
                .first()
                .is_some_and(|head| matches_symbol(head, "unsafe-cast"))
            {
                if let [_, target, value, ..] = items.as_slice() {
                    let (line, column) = line_column(input, expr.span.start);
                    let (end_line, end_column) = line_column(input, expr.span.end);
                    casts.push(UnsafeCastInfo {
                        target: source_excerpt(input, target.span),
                        expr: source_excerpt(input, value.span),
                        start: expr.span.start,
                        end: expr.span.end,
                        line,
                        column,
                        end_line,
                        end_column,
                    });
                }
            }
            for item in items {
                collect_unsafe_casts_expr(input, item, casts);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_unsafe_casts_expr(input, item, casts);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_unsafe_casts_expr(input, key, casts);
                collect_unsafe_casts_expr(input, value, casts);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_unsafe_casts_expr(input, inner, casts),
        ExprKind::HtmlTemplate(node) => collect_unsafe_casts_html_node(input, node, casts),
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_) => {}
    }
}

fn collect_unsafe_casts_html_node(
    input: &str,
    node: &syntax::HtmlNode,
    casts: &mut Vec<UnsafeCastInfo>,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_unsafe_casts_expr(input, expr, casts);
                }
            }
            for child in &element.children {
                collect_unsafe_casts_html_node(input, child, casts);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_unsafe_casts_expr(input, expr, casts),
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn collect_test_cases(source: &SourceFile) -> Vec<TestCaseInfo> {
    let mut tests = Vec::new();
    let mut group = Vec::new();
    for form in &source.forms {
        collect_test_cases_expr(form, &mut group, &mut tests);
    }
    tests
}

fn collect_test_cases_expr(expr: &Expr, group: &mut Vec<String>, tests: &mut Vec<TestCaseInfo>) {
    match &expr.kind {
        ExprKind::List(items) => {
            if let Some(head) = items.first().and_then(symbol_name) {
                if head == "describe" {
                    if let Some(name) = items.get(1).and_then(string_literal) {
                        group.push(name.to_string());
                        for item in items.iter().skip(2) {
                            collect_test_cases_expr(item, group, tests);
                        }
                        group.pop();
                        return;
                    }
                }
                if head == "test" {
                    if let Some(name) = items.get(1).and_then(string_literal) {
                        tests.push(TestCaseInfo {
                            name: name.to_string(),
                            group: group.clone(),
                        });
                        return;
                    }
                }
            }
            for item in items {
                collect_test_cases_expr(item, group, tests);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_test_cases_expr(item, group, tests);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_test_cases_expr(key, group, tests);
                collect_test_cases_expr(value, group, tests);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_test_cases_expr(inner, group, tests),
        ExprKind::HtmlTemplate(node) => collect_test_cases_html_node(node, group, tests),
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_) => {}
    }
}

fn collect_test_cases_html_node(
    node: &syntax::HtmlNode,
    group: &mut Vec<String>,
    tests: &mut Vec<TestCaseInfo>,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_test_cases_expr(expr, group, tests);
                }
            }
            for child in &element.children {
                collect_test_cases_html_node(child, group, tests);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_test_cases_expr(expr, group, tests),
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn source_excerpt(input: &str, span: syntax::Span) -> String {
    input
        .get(span.start..span.end)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn string_literal(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::String(value) => Some(value),
        _ => None,
    }
}

fn nodes_json(nodes: &[template_ir::Node]) -> String {
    let entries = nodes
        .iter()
        .map(|node| {
            format!(
                "{{\"id\":{},\"parent\":{},\"kind\":{}}}",
                node.id,
                node.parent
                    .map(|parent| parent.to_string())
                    .unwrap_or_else(|| "null".to_string()),
                node_kind_json(&node.kind)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn node_kind_json(kind: &NodeKind) -> String {
    match kind {
        NodeKind::Element { tag, static_attrs } => format!(
            "{{\"element\":{},\"staticAttrs\":{}}}",
            json_string(tag),
            pair_array_json(static_attrs)
        ),
        NodeKind::Text(text) => format!("{{\"text\":{}}}", json_string(text)),
        NodeKind::DynamicText => json_string("dynamic-text"),
        NodeKind::KeyedListMarker => json_string("keyed-list-marker"),
        NodeKind::ConditionalMarker => json_string("conditional-marker"),
        NodeKind::ComponentMarker => json_string("component-marker"),
    }
}

fn slots_json(template_name: &str, slots: &[template_ir::Slot]) -> String {
    let entries = slots
        .iter()
        .map(|slot| {
            format!(
                "{{\"id\":{},\"node\":{},\"template\":{},\"kind\":{},\"expr\":{},\"reads\":{}}}",
                slot.id,
                slot.node_id,
                json_string(template_name),
                slot_kind_json(&slot.kind),
                json_string(&slot.expr),
                json_string_array(&slot.reads)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn slot_kind_json(kind: &SlotKind) -> String {
    match kind {
        SlotKind::Text => json_string("text"),
        SlotKind::Attr(name) => format!("{{\"attr\":{}}}", json_string(name)),
        SlotKind::Event(name) => format!("{{\"event\":{}}}", json_string(name)),
        SlotKind::Ref => "{\"ref\":true}".to_string(),
        SlotKind::KeyedList { item, index, key } => match index {
            Some(index) => format!(
                "{{\"keyed\":{},\"index\":{},\"key\":{}}}",
                json_string(item),
                json_string(index),
                json_string(key)
            ),
            None => format!(
                "{{\"keyed\":{},\"key\":{}}}",
                json_string(item),
                json_string(key)
            ),
        },
        SlotKind::Conditional => "{\"conditional\":true}".to_string(),
        SlotKind::Component { name } => format!("{{\"component\":{}}}", json_string(name)),
    }
}

fn component_graph_json(templates: &[NamedTemplate]) -> String {
    let entries = templates
        .iter()
        .map(|template| {
            let uses = template
                .template
                .slots
                .iter()
                .flat_map(|slot| {
                    let mut uses = slot.component_uses.clone();
                    if let SlotKind::Component { name } = &slot.kind {
                        if name != "scope-view" {
                            uses.push(name.clone());
                        }
                    }
                    uses
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            format!(
                "{{\"component\":{},\"uses\":{}}}",
                json_string(&template.name),
                json_string_array(&uses)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn state_path_to_slots_json(templates: &[NamedTemplate]) -> String {
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for template in templates {
        for slot in &template.template.slots {
            for read in &slot.reads {
                if read == "state" || read.starts_with("state.") {
                    by_path.entry(read.clone()).or_default().push(format!(
                        "{{\"template\":{},\"slot\":{},\"node\":{},\"kind\":{},\"expr\":{},\"reads\":{}}}",
                        json_string(&template.name),
                        slot.id,
                        slot.node_id,
                        slot_kind_json(&slot.kind),
                        json_string(&slot.expr),
                        json_string_array(&slot.reads)
                    ));
                }
            }
        }
    }

    let entries = by_path
        .into_iter()
        .map(|(path, slots)| {
            format!(
                "{{\"path\":{},\"slots\":[{}]}}",
                json_string(&path),
                slots.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn changed_path_summaries_json(summaries: &[ChangedPathSummary]) -> String {
    let entries = summaries
        .iter()
        .map(|summary| {
            format!(
                "{{\"source\":{},\"operation\":{},\"path\":{},\"expr\":{},\"span\":{{\"start\":{},\"end\":{}}},\"range\":{{\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}}}}",
                json_string(&summary.source),
                json_string(&summary.operation),
                json_string(&summary.path),
                json_string(&summary.expr),
                summary.start,
                summary.end,
                summary.line,
                summary.column,
                summary.end_line,
                summary.end_column
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn collect_changed_path_summaries(input: &str, source: &SourceFile) -> Vec<ChangedPathSummary> {
    let mut summaries = BTreeSet::new();
    for form in &source.forms {
        let source_name = definition_name(form).unwrap_or("module");
        collect_changed_path_summaries_expr(input, form, source_name, &mut summaries);
    }
    summaries.into_iter().collect()
}

fn collect_changed_path_summaries_expr(
    input: &str,
    expr: &Expr,
    source_name: &str,
    summaries: &mut BTreeSet<ChangedPathSummary>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            if let Some((head, args)) = items.split_first() {
                if let Some(operation) = symbol_name(head) {
                    collect_changed_path_summary_call(
                        input,
                        expr,
                        operation,
                        args,
                        source_name,
                        summaries,
                    );
                }
            }
            for item in items {
                collect_changed_path_summaries_expr(input, item, source_name, summaries);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_changed_path_summaries_expr(input, item, source_name, summaries);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_changed_path_summaries_expr(input, key, source_name, summaries);
                collect_changed_path_summaries_expr(input, value, source_name, summaries);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_changed_path_summaries_expr(input, inner, source_name, summaries)
        }
        ExprKind::HtmlTemplate(node) => {
            collect_changed_path_summaries_html_node(input, node, source_name, summaries)
        }
        ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_)
        | ExprKind::Symbol(_) => {}
    }
}

fn collect_changed_path_summary_call(
    input: &str,
    expr: &Expr,
    operation: &str,
    args: &[Expr],
    source_name: &str,
    summaries: &mut BTreeSet<ChangedPathSummary>,
) {
    match operation {
        "assoc" => {
            let Some(base) = args.first().and_then(changed_path_base) else {
                return;
            };
            for pair in args[1..].chunks(2) {
                let path = pair
                    .first()
                    .and_then(path_segment_literal)
                    .map(|segment| join_state_path(&base, &segment))
                    .unwrap_or_else(|| base.clone());
                push_changed_path_summary(input, expr, source_name, operation, path, summaries);
            }
        }
        "merge" => {
            let Some(base) = args.first().and_then(changed_path_base) else {
                return;
            };
            let mut reported_static_path = false;
            for arg in args.iter().skip(1) {
                if let ExprKind::Map(entries) = &arg.kind {
                    for (key, _) in entries {
                        if let Some(segment) = path_segment_literal(key) {
                            push_changed_path_summary(
                                input,
                                expr,
                                source_name,
                                operation,
                                join_state_path(&base, &segment),
                                summaries,
                            );
                            reported_static_path = true;
                        }
                    }
                }
            }
            if !reported_static_path {
                push_changed_path_summary(input, expr, source_name, operation, base, summaries);
            }
        }
        "dissoc" => {
            let Some(base) = args.first().and_then(changed_path_base) else {
                return;
            };
            if args.len() <= 1 {
                push_changed_path_summary(input, expr, source_name, operation, base, summaries);
                return;
            }
            for key in args.iter().skip(1) {
                let path = path_segment_literal(key)
                    .map(|segment| join_state_path(&base, &segment))
                    .unwrap_or_else(|| base.clone());
                push_changed_path_summary(input, expr, source_name, operation, path, summaries);
            }
        }
        "assoc-in" | "update-in" => {
            let Some(base) = args.first().and_then(changed_path_base) else {
                return;
            };
            let path = args
                .get(1)
                .and_then(literal_path_segments)
                .map(|segments| join_state_path_segments(&base, &segments))
                .unwrap_or(base);
            push_changed_path_summary(input, expr, source_name, operation, path, summaries);
        }
        "map-assoc" | "map-dissoc" => {
            let Some(base) = args.first().and_then(changed_path_base) else {
                return;
            };
            push_changed_path_summary(input, expr, source_name, operation, base, summaries);
        }
        "scope-update" => {
            let Some(base) = args.first().and_then(changed_path_base) else {
                return;
            };
            let path = args
                .get(1)
                .and_then(path_segment_literal)
                .map(|segment| join_state_path(&base, &segment))
                .unwrap_or(base);
            push_changed_path_summary(input, expr, source_name, operation, path, summaries);
        }
        _ => {}
    }
}

fn collect_changed_path_summaries_html_node(
    input: &str,
    node: &syntax::HtmlNode,
    source_name: &str,
    summaries: &mut BTreeSet<ChangedPathSummary>,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_changed_path_summaries_expr(input, expr, source_name, summaries);
                }
            }
            for child in &element.children {
                collect_changed_path_summaries_html_node(input, child, source_name, summaries);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => {
            collect_changed_path_summaries_expr(input, expr, source_name, summaries)
        }
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn push_changed_path_summary(
    input: &str,
    expr: &Expr,
    source_name: &str,
    operation: &str,
    path: String,
    summaries: &mut BTreeSet<ChangedPathSummary>,
) {
    let (line, column) = line_column(input, expr.span.start);
    let (end_line, end_column) = line_column(input, expr.span.end);
    summaries.insert(ChangedPathSummary {
        source: source_name.to_string(),
        operation: operation.to_string(),
        path,
        expr: source_excerpt(input, expr.span),
        start: expr.span.start,
        end: expr.span.end,
        line,
        column,
        end_line,
        end_column,
    });
}

fn changed_path_base(expr: &Expr) -> Option<String> {
    let ExprKind::Symbol(name) = &expr.kind else {
        return None;
    };
    (name == "state" || name.starts_with("state.")).then(|| name.clone())
}

fn path_segment_literal(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name)
        | ExprKind::Symbol(name)
        | ExprKind::String(name)
        | ExprKind::Number(name) => Some(name.clone()),
        _ => None,
    }
}

fn literal_path_segments(expr: &Expr) -> Option<Vec<String>> {
    let ExprKind::Vector(items) = &expr.kind else {
        return None;
    };
    items.iter().map(path_segment_literal).collect()
}

fn join_state_path(base: &str, segment: &str) -> String {
    if segment.is_empty() {
        return base.to_string();
    }
    format!("{}.{}", base, segment)
}

fn join_state_path_segments(base: &str, segments: &[String]) -> String {
    segments.iter().fold(base.to_string(), |path, segment| {
        join_state_path(&path, segment)
    })
}

fn collect_command_shapes(source: &SourceFile) -> Vec<CommandShape> {
    let mut shapes: BTreeMap<String, CommandShapeData> = BTreeMap::new();
    for form in &source.forms {
        let source_name = definition_name(form).unwrap_or("module");
        collect_command_shapes_expr(form, &mut shapes, source_name);
    }
    command_shapes_from_map(shapes)
}

fn collect_subscription_shapes(source: &SourceFile) -> Vec<CommandShape> {
    let mut shapes: BTreeMap<String, CommandShapeData> = BTreeMap::new();
    for form in &source.forms {
        let source_name = definition_name(form).unwrap_or("module");
        collect_subscription_shapes_expr(form, &mut shapes, source_name);
    }
    command_shapes_from_map(shapes)
}

fn collect_command_shapes_by_binding(
    source: &SourceFile,
    command_binding_names: &HashSet<String>,
    imported_command_shapes: &HashMap<String, Vec<CommandShape>>,
) -> HashMap<String, Vec<CommandShape>> {
    let bodies = collect_definition_bodies(source);
    let mut by_binding = HashMap::new();
    for name in command_binding_names {
        let mut shapes = BTreeMap::new();
        let mut visiting = HashSet::new();
        collect_command_shapes_for_binding(
            name,
            name,
            &bodies,
            imported_command_shapes,
            &mut visiting,
            &mut shapes,
        );
        by_binding.insert(name.clone(), command_shapes_from_map(shapes));
    }
    by_binding
}

#[derive(Clone, Debug, Default)]
struct MessageStaticContext {
    static_reads: BTreeMap<String, String>,
}

fn collect_message_kinds_by_binding(
    source: &SourceFile,
    imported_message_kinds: &HashMap<String, BTreeSet<String>>,
) -> HashMap<String, BTreeSet<String>> {
    collect_message_kinds_by_binding_with_static(
        source,
        imported_message_kinds,
        &MessageStaticContext::default(),
    )
}

fn collect_message_kinds_by_binding_with_static(
    source: &SourceFile,
    imported_message_kinds: &HashMap<String, BTreeSet<String>>,
    static_context: &MessageStaticContext,
) -> HashMap<String, BTreeSet<String>> {
    let bodies = collect_definition_bodies(source);
    let mut summaries = HashMap::<String, BTreeSet<String>>::new();

    loop {
        let mut changed = false;
        for (name, body) in &bodies {
            let mut kinds = BTreeSet::new();
            collect_message_kinds_expr_with_static(
                body,
                imported_message_kinds,
                &summaries,
                &mut kinds,
                static_context,
            );
            if summaries.get(name) != Some(&kinds) {
                summaries.insert(name.clone(), kinds);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    summaries
}

fn collect_reachable_update_message_kinds(
    source: &SourceFile,
    imported_message_kinds: &HashMap<String, BTreeSet<String>>,
    emit_options: &js_backend::EmitOptions,
) -> Option<BTreeSet<String>> {
    let static_context = MessageStaticContext {
        static_reads: emit_options.static_reads.clone(),
    };
    let summaries = collect_message_kinds_by_binding_with_static(
        source,
        imported_message_kinds,
        &static_context,
    );
    let update_body = collect_definition_bodies(source).get("update").copied()?;
    let arms = update_message_arms(update_body)?;

    let mut reachable = BTreeSet::new();
    for entry in ["init", "view", "subscriptions"] {
        if let Some(kinds) = summaries.get(entry) {
            reachable.extend(kinds.iter().cloned());
        }
    }

    loop {
        let before = reachable.len();
        for (kind, body) in &arms {
            if !reachable.contains(kind) {
                continue;
            }
            collect_message_kinds_expr_with_static(
                body,
                imported_message_kinds,
                &summaries,
                &mut reachable,
                &static_context,
            );
        }
        if reachable.len() == before {
            break;
        }
    }

    Some(reachable)
}

fn update_message_arms(expr: &Expr) -> Option<Vec<(String, &Expr)>> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let [head, value, arms @ ..] = items.as_slice() else {
        return None;
    };
    if !matches_symbol(head, "match") || !matches_symbol(value, "msg") || arms.len() < 2 {
        return None;
    }

    let mut output = Vec::new();
    for pair in arms.chunks(2) {
        let [pattern, body] = pair else {
            continue;
        };
        if let Some(kind) = message_kind_pattern(pattern) {
            output.push((kind, body));
        }
    }
    Some(output)
}

fn message_kind_pattern(pattern: &Expr) -> Option<String> {
    match &pattern.kind {
        ExprKind::Map(entries) => {
            kind_literal_from_entries(entries).filter(|kind| is_message_kind(kind))
        }
        ExprKind::List(items) if items.len() == 3 && matches_symbol(&items[0], "as") => {
            message_kind_pattern(&items[1])
        }
        _ => None,
    }
}

fn collect_app_static_reads(
    source: &SourceFile,
    emit_options: &js_backend::EmitOptions,
) -> BTreeMap<String, String> {
    let Some(fields) = init_static_state_fields(source, emit_options) else {
        return BTreeMap::new();
    };
    let reassigned = record_keys_outside_defn(source, "init");
    fields
        .into_iter()
        .filter(|(field, _)| !reassigned.contains(field))
        .map(|(field, value)| (format!("state.{}", field), value))
        .collect()
}

fn init_static_state_fields(
    source: &SourceFile,
    emit_options: &js_backend::EmitOptions,
) -> Option<BTreeMap<String, String>> {
    let ExprKind::List(items) = &source
        .forms
        .iter()
        .find_map(|form| {
            let ExprKind::List(items) = &form.kind else {
                return None;
            };
            if items.len() >= 4
                && matches_symbol(&items[0], "defn")
                && matches_symbol(&items[1], "init")
            {
                Some(form)
            } else {
                None
            }
        })?
        .kind
    else {
        return None;
    };

    let mut values = BTreeMap::new();
    let mut state_bindings = BTreeMap::new();
    init_static_state_fields_from_expr(
        items.last()?,
        emit_options,
        &mut values,
        &mut state_bindings,
    )
}

fn init_static_state_fields_from_expr(
    expr: &Expr,
    emit_options: &js_backend::EmitOptions,
    values: &mut BTreeMap<String, String>,
    state_bindings: &mut BTreeMap<String, BTreeMap<String, String>>,
) -> Option<BTreeMap<String, String>> {
    match &expr.kind {
        ExprKind::List(items) if items.len() == 3 && matches_symbol(&items[0], "let") => {
            let ExprKind::Vector(bindings) = &items[1].kind else {
                return None;
            };
            let mut values = values.clone();
            let mut state_bindings = state_bindings.clone();
            for pair in bindings.chunks(2) {
                let [binding, value] = pair else {
                    continue;
                };
                let Some(name) = symbol_name(binding) else {
                    continue;
                };
                if let Some(fields) =
                    static_state_fields_from_expr(value, emit_options, &values, &state_bindings)
                {
                    state_bindings.insert(name.to_string(), fields);
                }
                if let Some(value) = static_js_value(value, emit_options, &values) {
                    values.insert(name.to_string(), value);
                }
            }
            init_static_state_fields_from_expr(
                &items[2],
                emit_options,
                &mut values,
                &mut state_bindings,
            )
        }
        ExprKind::Vector(items) => {
            let first = items.first()?;
            static_state_fields_from_expr(first, emit_options, values, state_bindings)
        }
        _ => static_state_fields_from_expr(expr, emit_options, values, state_bindings),
    }
}

fn static_state_fields_from_expr(
    expr: &Expr,
    emit_options: &js_backend::EmitOptions,
    values: &BTreeMap<String, String>,
    state_bindings: &BTreeMap<String, BTreeMap<String, String>>,
) -> Option<BTreeMap<String, String>> {
    match &expr.kind {
        ExprKind::Symbol(name) => state_bindings.get(name).cloned(),
        ExprKind::Map(entries) => {
            let mut fields = BTreeMap::new();
            for (key, value) in entries {
                let Some(field) = record_key_name(key) else {
                    continue;
                };
                let Some(value) = static_js_value(value, emit_options, values) else {
                    continue;
                };
                fields.insert(field, value);
            }
            Some(fields)
        }
        ExprKind::List(items) => {
            let (head, args) = items.split_first()?;
            if !matches_symbol(head, "merge") {
                return None;
            }
            let mut fields = BTreeMap::new();
            for arg in args {
                if let Some(arg_fields) =
                    static_state_fields_from_expr(arg, emit_options, values, state_bindings)
                {
                    fields.extend(arg_fields);
                }
            }
            Some(fields)
        }
        _ => None,
    }
}

fn static_js_value(
    expr: &Expr,
    _emit_options: &js_backend::EmitOptions,
    values: &BTreeMap<String, String>,
) -> Option<String> {
    match &expr.kind {
        ExprKind::Bool(value) => Some(value.to_string()),
        ExprKind::Number(value) => Some(value.clone()),
        ExprKind::String(value) | ExprKind::Keyword(value) => Some(json_string(value)),
        ExprKind::Nil => Some("null".to_string()),
        ExprKind::Symbol(name) => values.get(name).cloned(),
        ExprKind::List(_) => None,
        _ => None,
    }
}

fn record_keys_outside_defn(source: &SourceFile, skipped_defn: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for form in &source.forms {
        if let ExprKind::List(items) = &form.kind {
            if items.first().is_some_and(|head| {
                matches_symbol(head, "type")
                    || matches_symbol(head, "ann")
                    || matches_symbol(head, "foreign")
                    || matches_symbol(head, "import")
            }) {
                continue;
            }
            if items.len() >= 4
                && matches_symbol(&items[0], "defn")
                && matches_symbol(&items[1], skipped_defn)
            {
                continue;
            }
        }
        collect_record_keys_expr(form, &mut keys);
    }
    keys
}

fn collect_record_keys_expr(expr: &Expr, keys: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                if let Some(name) = record_key_name(key) {
                    keys.insert(name);
                }
                collect_record_keys_expr(key, keys);
                collect_record_keys_expr(value, keys);
            }
        }
        ExprKind::List(items) | ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_record_keys_expr(item, keys);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_record_keys_expr(inner, keys),
        ExprKind::HtmlTemplate(node) => collect_record_keys_html_node(node, keys),
        ExprKind::Symbol(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_record_keys_html_node(node: &syntax::HtmlNode, keys: &mut BTreeSet<String>) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_record_keys_expr(expr, keys);
                }
            }
            for child in &element.children {
                collect_record_keys_html_node(child, keys);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_record_keys_expr(expr, keys),
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn collect_message_kinds_expr_with_static(
    expr: &Expr,
    imported_message_kinds: &HashMap<String, BTreeSet<String>>,
    summaries: &HashMap<String, BTreeSet<String>>,
    output: &mut BTreeSet<String>,
    static_context: &MessageStaticContext,
) {
    match &expr.kind {
        ExprKind::Map(entries) => {
            if let Some(kind) =
                kind_literal_from_entries(entries).filter(|kind| is_message_kind(kind))
            {
                output.insert(kind);
            }
            for (key, value) in entries {
                if record_key_name(key)
                    .as_deref()
                    .is_some_and(is_message_continuation_field)
                {
                    collect_message_value(
                        Some(value),
                        imported_message_kinds,
                        summaries,
                        output,
                        static_context,
                    );
                }
                collect_message_kinds_expr_with_static(
                    key,
                    imported_message_kinds,
                    summaries,
                    output,
                    static_context,
                );
                collect_message_kinds_expr_with_static(
                    value,
                    imported_message_kinds,
                    summaries,
                    output,
                    static_context,
                );
            }
        }
        ExprKind::List(items) => {
            if let [head, condition, then_branch, else_branch] = items.as_slice() {
                if matches_symbol(head, "if") {
                    if let Some(value) = message_static_bool(condition, static_context) {
                        collect_message_kinds_expr_with_static(
                            if value { then_branch } else { else_branch },
                            imported_message_kinds,
                            summaries,
                            output,
                            static_context,
                        );
                        return;
                    }
                }
            }
            if let Some(head) = items.first().and_then(symbol_name) {
                if let Some(kinds) = imported_message_kinds
                    .get(head)
                    .or_else(|| summaries.get(head))
                {
                    output.extend(kinds.iter().cloned());
                }
                collect_message_helper_kinds(
                    items,
                    imported_message_kinds,
                    summaries,
                    output,
                    static_context,
                );
            }
            for item in items {
                collect_message_kinds_expr_with_static(
                    item,
                    imported_message_kinds,
                    summaries,
                    output,
                    static_context,
                );
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_message_kinds_expr_with_static(
                    item,
                    imported_message_kinds,
                    summaries,
                    output,
                    static_context,
                );
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_message_kinds_expr_with_static(
            inner,
            imported_message_kinds,
            summaries,
            output,
            static_context,
        ),
        ExprKind::HtmlTemplate(node) => collect_message_kinds_html_node_with_static(
            node,
            imported_message_kinds,
            summaries,
            output,
            static_context,
        ),
        ExprKind::Symbol(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_message_kinds_html_node_with_static(
    node: &syntax::HtmlNode,
    imported_message_kinds: &HashMap<String, BTreeSet<String>>,
    summaries: &HashMap<String, BTreeSet<String>>,
    output: &mut BTreeSet<String>,
    static_context: &MessageStaticContext,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            let statically_disabled = html_element_statically_disabled(element, static_context);
            for attr in &element.attrs {
                if statically_disabled && attr.name.starts_with("on:") {
                    continue;
                }
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_message_kinds_expr_with_static(
                        expr,
                        imported_message_kinds,
                        summaries,
                        output,
                        static_context,
                    );
                }
            }
            for child in &element.children {
                collect_message_kinds_html_node_with_static(
                    child,
                    imported_message_kinds,
                    summaries,
                    output,
                    static_context,
                );
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_message_kinds_expr_with_static(
            expr,
            imported_message_kinds,
            summaries,
            output,
            static_context,
        ),
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn html_element_statically_disabled(
    element: &syntax::HtmlElement,
    static_context: &MessageStaticContext,
) -> bool {
    element.attrs.iter().any(|attr| {
        attr.name == "disabled"
            && match &attr.value {
                syntax::HtmlAttrValue::Bool(value) => *value,
                syntax::HtmlAttrValue::Static(_) => true,
                syntax::HtmlAttrValue::Dynamic { expr, .. } => {
                    message_static_bool(expr, static_context).is_some_and(|value| value)
                }
            }
    })
}

fn collect_message_helper_kinds(
    items: &[Expr],
    imported_message_kinds: &HashMap<String, BTreeSet<String>>,
    summaries: &HashMap<String, BTreeSet<String>>,
    output: &mut BTreeSet<String>,
    static_context: &MessageStaticContext,
) {
    let Some(head) = items.first().and_then(symbol_name) else {
        return;
    };
    let args = &items[1..];
    match head {
        "Msg.of" | "Msg.with" | "Msg.with2" | "Msg.mapper" => {
            collect_message_value(
                args.first(),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.storage/get" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(3),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.storage/set" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(3),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.time/now" => collect_message_value(
            args.first(),
            imported_message_kinds,
            summaries,
            output,
            static_context,
        ),
        "Cmd.random/number" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.timer/every" | "Cmd.timer/after" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.animation/frame" => {
            collect_message_value(
                args.get(1),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.animation/cancel" => {
            collect_message_value(
                args.get(1),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.dom-ref/click" | "Cmd.dom-ref/focus" => {
            collect_message_value(
                args.get(1),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.file/read-selected" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(3),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(4),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.file/download" => {
            collect_message_value(
                args.get(3),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(4),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.canvas/draw" => collect_message_value(
            args.get(4),
            imported_message_kinds,
            summaries,
            output,
            static_context,
        ),
        "Cmd.dom-ref/measure" => {
            collect_message_value(
                args.get(1),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.dom-ref/resize-watch" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(3),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.bluetooth/connect-heart-rate" => {
            for index in 2..=5 {
                collect_message_value(
                    args.get(index),
                    imported_message_kinds,
                    summaries,
                    output,
                    static_context,
                );
            }
        }
        "Cmd.bluetooth/disconnect" => {
            collect_message_value(
                args.get(1),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Cmd.simulation/heart-rate" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(3),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(4),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
            collect_message_value(
                args.get(5),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        "Sub.timer/every" => collect_message_value(
            args.get(2),
            imported_message_kinds,
            summaries,
            output,
            static_context,
        ),
        "Sub.media-query" | "Sub.window/event" | "Sub.window/event-with" | "Sub.dom-ref/resize" => {
            collect_message_value(
                args.get(2),
                imported_message_kinds,
                summaries,
                output,
                static_context,
            );
        }
        _ => {}
    }
}

fn collect_message_value(
    expr: Option<&Expr>,
    imported_message_kinds: &HashMap<String, BTreeSet<String>>,
    summaries: &HashMap<String, BTreeSet<String>>,
    output: &mut BTreeSet<String>,
    static_context: &MessageStaticContext,
) {
    let Some(expr) = expr else {
        return;
    };
    if let Some(kind) = literal_name(expr).filter(|kind| is_message_kind(kind)) {
        output.insert(kind);
    } else {
        collect_message_kinds_expr_with_static(
            expr,
            imported_message_kinds,
            summaries,
            output,
            static_context,
        );
    }
}

fn is_message_continuation_field(field: &str) -> bool {
    matches!(
        field,
        "msg"
            | "toMessage"
            | "onError"
            | "onCancel"
            | "onSuccess"
            | "onFrame"
            | "onReading"
            | "onDisconnected"
            | "onChange"
    )
}

fn message_static_bool(expr: &Expr, static_context: &MessageStaticContext) -> Option<bool> {
    match &expr.kind {
        ExprKind::Bool(value) => Some(*value),
        ExprKind::Symbol(name) => match static_context.static_reads.get(name).map(String::as_str) {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        },
        ExprKind::List(items) => {
            let (head, args) = items.split_first()?;
            let head = symbol_name(head)?;
            match head {
                "not" if args.len() == 1 => {
                    message_static_bool(&args[0], static_context).map(|value| !value)
                }
                "and" => {
                    let mut saw_unknown = false;
                    for arg in args {
                        match message_static_bool(arg, static_context) {
                            Some(false) => return Some(false),
                            Some(true) => {}
                            None => saw_unknown = true,
                        }
                    }
                    (!saw_unknown).then_some(true)
                }
                "or" => {
                    let mut saw_unknown = false;
                    for arg in args {
                        match message_static_bool(arg, static_context) {
                            Some(true) => return Some(true),
                            Some(false) => {}
                            None => saw_unknown = true,
                        }
                    }
                    (!saw_unknown).then_some(false)
                }
                "=" if args.len() == 2 => {
                    match (
                        message_static_value(&args[0], static_context),
                        message_static_value(&args[1], static_context),
                    ) {
                        (Some(left), Some(right)) => Some(left == right),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn message_static_value(expr: &Expr, static_context: &MessageStaticContext) -> Option<String> {
    match &expr.kind {
        ExprKind::Bool(value) => Some(value.to_string()),
        ExprKind::Number(value) | ExprKind::String(value) | ExprKind::Keyword(value) => {
            Some(value.clone())
        }
        ExprKind::Symbol(name) => static_context.static_reads.get(name).cloned(),
        ExprKind::List(_) => {
            message_static_bool(expr, static_context).map(|value| value.to_string())
        }
        _ => None,
    }
}

fn is_message_kind(kind: &str) -> bool {
    !kind.contains('/') && !matches!(kind, "none" | "batch")
}

fn collect_definition_bodies(source: &SourceFile) -> HashMap<String, &Expr> {
    source
        .forms
        .iter()
        .filter_map(|form| {
            let ExprKind::List(items) = &form.kind else {
                return None;
            };
            let head = items.first()?;
            if matches_symbol(head, "defn") && items.len() >= 4 {
                let ExprKind::Symbol(name) = &items[1].kind else {
                    return None;
                };
                return Some((name.clone(), items.last()?));
            }
            if matches_symbol(head, "def") && items.len() == 3 {
                let ExprKind::Symbol(name) = &items[1].kind else {
                    return None;
                };
                return Some((name.clone(), &items[2]));
            }
            None
        })
        .collect()
}

fn definition_name(expr: &Expr) -> Option<&str> {
    let ExprKind::List(items) = &expr.kind else {
        return None;
    };
    let head = items.first()?;
    if !(matches_symbol(head, "defn") || matches_symbol(head, "def")) {
        return None;
    }
    let ExprKind::Symbol(name) = &items[1].kind else {
        return None;
    };
    Some(name)
}

fn collect_command_shapes_for_binding(
    name: &str,
    source_name: &str,
    bodies: &HashMap<String, &Expr>,
    imported_command_shapes: &HashMap<String, Vec<CommandShape>>,
    visiting: &mut HashSet<String>,
    shapes: &mut BTreeMap<String, CommandShapeData>,
) {
    if let Some(imported_shapes) = imported_command_shapes.get(name) {
        merge_command_shapes_with_source(
            shapes,
            imported_shapes.iter().cloned(),
            Some(source_name),
        );
        return;
    }

    if !visiting.insert(name.to_string()) {
        return;
    }

    if let Some(body) = bodies.get(name) {
        collect_command_shapes_expr(body, shapes, source_name);
        let mut callees = BTreeSet::new();
        collect_function_call_names(body, &mut callees);
        for callee in callees {
            collect_command_shapes_for_binding(
                &callee,
                source_name,
                bodies,
                imported_command_shapes,
                visiting,
                shapes,
            );
        }
    }

    visiting.remove(name);
}

fn collect_function_call_names(expr: &Expr, calls: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::List(items) => {
            if let Some(head) = items.first() {
                if let ExprKind::Symbol(name) = &head.kind {
                    calls.insert(name.clone());
                }
            }
            for item in items {
                collect_function_call_names(item, calls);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_function_call_names(item, calls);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                collect_function_call_names(key, calls);
                collect_function_call_names(value, calls);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => collect_function_call_names(inner, calls),
        ExprKind::HtmlTemplate(node) => collect_function_call_names_html_node(node, calls),
        ExprKind::Symbol(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_function_call_names_html_node(node: &syntax::HtmlNode, calls: &mut BTreeSet<String>) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_function_call_names(expr, calls);
                }
            }
            for child in &element.children {
                collect_function_call_names_html_node(child, calls);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => collect_function_call_names(expr, calls),
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn collect_command_shapes_expr(
    expr: &Expr,
    shapes: &mut BTreeMap<String, CommandShapeData>,
    source_name: &str,
) {
    match &expr.kind {
        ExprKind::Map(entries) => {
            if let Some(kind) = kind_literal_from_entries(entries) {
                if effects::is_known_command_kind(&kind) {
                    let fields = entries
                        .iter()
                        .filter_map(|(key, _)| record_key_name(key))
                        .collect::<BTreeSet<_>>();
                    let entry = shapes.entry(kind).or_default();
                    entry.fields.extend(fields);
                    entry.sources.insert(source_name.to_string());
                }
            }
            for (key, value) in entries {
                collect_command_shapes_expr(key, shapes, source_name);
                collect_command_shapes_expr(value, shapes, source_name);
            }
        }
        ExprKind::List(items) => {
            collect_command_helper_shape(items, shapes, source_name);
            for item in items {
                collect_command_shapes_expr(item, shapes, source_name);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_command_shapes_expr(item, shapes, source_name);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_command_shapes_expr(inner, shapes, source_name)
        }
        ExprKind::HtmlTemplate(node) => collect_command_shapes_html_node(node, shapes, source_name),
        ExprKind::Symbol(name) if name == "Cmd.none" => {
            let entry = shapes.entry("none".to_string()).or_default();
            entry.fields.insert("kind".to_string());
            entry.sources.insert(source_name.to_string());
        }
        ExprKind::Symbol(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_command_helper_shape(
    items: &[Expr],
    shapes: &mut BTreeMap<String, CommandShapeData>,
    source_name: &str,
) {
    let Some(head) = items.first().and_then(symbol_name) else {
        return;
    };
    let (kind, fields): (&str, &[&str]) = match head {
        "Cmd.batch" => ("batch", &["kind", "commands"]),
        "Cmd.storage/get" => (
            "storage/get",
            &["kind", "key", "format", "toMessage", "onError"],
        ),
        "Cmd.storage/set" => ("storage/set", &["kind", "key", "value", "msg", "onError"]),
        "Cmd.storage/set-silent" => ("storage/set", &["kind", "key", "value", "onError"]),
        "Cmd.time/now" => ("time/now", &["kind", "toMessage"]),
        "Cmd.random/number" => ("random/number", &["kind", "min", "max", "toMessage"]),
        "Cmd.timer/every" => ("timer/every", &["kind", "id", "ms", "msg"]),
        "Cmd.timer/after" => ("timer/after", &["kind", "id", "ms", "msg"]),
        "Cmd.timer/cancel" => ("timer/cancel", &["kind", "id"]),
        "Cmd.animation/frame" => ("animation/frame", &["kind", "id", "onFrame"]),
        "Cmd.animation/cancel" => ("animation/cancel", &["kind", "id", "msg"]),
        "Cmd.dom-ref/click" => ("dom-ref/click", &["kind", "ref", "msg", "onError"]),
        "Cmd.dom-ref/focus" => ("dom-ref/focus", &["kind", "ref", "msg", "onError"]),
        "Cmd.dom-ref/measure" => ("dom-ref/measure", &["kind", "ref", "toMessage", "onError"]),
        "Cmd.dom-ref/resize-watch" => (
            "dom-ref/resize-watch",
            &["kind", "id", "ref", "onChange", "onError"],
        ),
        "Cmd.file/read-selected" => (
            "file/read-selected",
            &["kind", "ref", "format", "toMessage", "onError", "onCancel"],
        ),
        "Cmd.file/download" => (
            "file/download",
            &["kind", "name", "content", "mime", "msg", "onError"],
        ),
        "Cmd.canvas/draw" => (
            "canvas/draw",
            &["kind", "ref", "cssWidth", "cssHeight", "ops", "onError"],
        ),
        "Cmd.bluetooth/connect-heart-rate" => (
            "bluetooth/connect-heart-rate",
            &[
                "kind",
                "id",
                "filters",
                "optionalServices",
                "acceptAllDevices",
                "service",
                "characteristic",
                "toMessage",
                "onReading",
                "onDisconnected",
                "onError",
            ],
        ),
        "Cmd.bluetooth/disconnect" => ("bluetooth/disconnect", &["kind", "id", "msg"]),
        "Cmd.simulation/heart-rate" => (
            "simulation/heart-rate",
            &[
                "kind",
                "id",
                "ms",
                "min",
                "max",
                "jitter",
                "start",
                "deviceName",
                "toMessage",
                "onReading",
                "onDisconnected",
                "onError",
            ],
        ),
        "Cmd.simulation/stop" => ("simulation/stop", &["kind", "id"]),
        "Task.perform" => ("task/perform", &["kind", "task", "onSuccess", "onError"]),
        _ => return,
    };
    let entry = shapes.entry(kind.to_string()).or_default();
    entry
        .fields
        .extend(fields.iter().map(|field| (*field).to_string()));
    entry.sources.insert(source_name.to_string());
}

fn collect_subscription_shapes_expr(
    expr: &Expr,
    shapes: &mut BTreeMap<String, CommandShapeData>,
    source_name: &str,
) {
    match &expr.kind {
        ExprKind::Map(entries) => {
            if let Some(kind) = kind_literal_from_entries(entries) {
                if effects::is_known_subscription_kind(&kind) {
                    let fields = entries
                        .iter()
                        .filter_map(|(key, _)| record_key_name(key))
                        .collect::<BTreeSet<_>>();
                    let entry = shapes.entry(kind).or_default();
                    entry.fields.extend(fields);
                    entry.sources.insert(source_name.to_string());
                }
            }
            for (key, value) in entries {
                collect_subscription_shapes_expr(key, shapes, source_name);
                collect_subscription_shapes_expr(value, shapes, source_name);
            }
        }
        ExprKind::List(items) => {
            collect_subscription_helper_shape(items, shapes, source_name);
            for item in items {
                collect_subscription_shapes_expr(item, shapes, source_name);
            }
        }
        ExprKind::Vector(items) | ExprKind::Set(items) => {
            for item in items {
                collect_subscription_shapes_expr(item, shapes, source_name);
            }
        }
        ExprKind::Quote(inner)
        | ExprKind::QuasiQuote(inner)
        | ExprKind::Unquote(inner)
        | ExprKind::UnquoteSplicing(inner) => {
            collect_subscription_shapes_expr(inner, shapes, source_name)
        }
        ExprKind::HtmlTemplate(node) => {
            collect_subscription_shapes_html_node(node, shapes, source_name)
        }
        ExprKind::Symbol(name) if name == "Sub.none" => {
            let entry = shapes.entry("none".to_string()).or_default();
            entry.fields.insert("kind".to_string());
            entry.sources.insert(source_name.to_string());
        }
        ExprKind::Symbol(_)
        | ExprKind::Nil
        | ExprKind::Bool(_)
        | ExprKind::Number(_)
        | ExprKind::String(_)
        | ExprKind::Keyword(_) => {}
    }
}

fn collect_subscription_helper_shape(
    items: &[Expr],
    shapes: &mut BTreeMap<String, CommandShapeData>,
    source_name: &str,
) {
    let Some(head) = items.first().and_then(symbol_name) else {
        return;
    };
    let (kind, fields): (&str, &[&str]) = match head {
        "Sub.batch" => ("batch", &["kind", "subscriptions"]),
        "Sub.timer/every" => ("sub/timer/every", &["kind", "id", "ms", "msg"]),
        "Sub.media-query" => ("sub/media-query", &["kind", "id", "query", "onChange"]),
        "Sub.window/event" => ("sub/window/event", &["kind", "id", "type", "onEvent"]),
        "Sub.window/event-with" => (
            "sub/window/event",
            &["kind", "id", "type", "onEvent", "options", "preventDefault"],
        ),
        "Sub.dom-ref/resize" => ("sub/dom-ref/resize", &["kind", "id", "ref", "onChange"]),
        _ => return,
    };
    let entry = shapes.entry(kind.to_string()).or_default();
    entry
        .fields
        .extend(fields.iter().map(|field| (*field).to_string()));
    entry.sources.insert(source_name.to_string());
}

fn collect_subscription_shapes_html_node(
    node: &syntax::HtmlNode,
    shapes: &mut BTreeMap<String, CommandShapeData>,
    source_name: &str,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_subscription_shapes_expr(expr, shapes, source_name);
                }
            }
            for child in &element.children {
                collect_subscription_shapes_html_node(child, shapes, source_name);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => {
            collect_subscription_shapes_expr(expr, shapes, source_name)
        }
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn collect_command_shapes_html_node(
    node: &syntax::HtmlNode,
    shapes: &mut BTreeMap<String, CommandShapeData>,
    source_name: &str,
) {
    match node {
        syntax::HtmlNode::Element(element) => {
            for attr in &element.attrs {
                if let syntax::HtmlAttrValue::Dynamic { expr, .. } = &attr.value {
                    collect_command_shapes_expr(expr, shapes, source_name);
                }
            }
            for child in &element.children {
                collect_command_shapes_html_node(child, shapes, source_name);
            }
        }
        syntax::HtmlNode::Expr { expr, .. } => {
            collect_command_shapes_expr(expr, shapes, source_name)
        }
        syntax::HtmlNode::Text { .. } => {}
    }
}

fn kind_literal_from_entries(entries: &[(Expr, Expr)]) -> Option<String> {
    entries.iter().find_map(|(key, value)| {
        (record_key_name(key).as_deref() == Some("kind")).then(|| literal_name(value))?
    })
}

fn literal_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
            Some(name.clone())
        }
        _ => None,
    }
}

fn symbol_name(expr: &Expr) -> Option<&str> {
    let ExprKind::Symbol(name) = &expr.kind else {
        return None;
    };
    Some(name)
}

fn record_key_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Keyword(name) | ExprKind::Symbol(name) | ExprKind::String(name) => {
            Some(name.clone())
        }
        _ => None,
    }
}

fn command_shapes_json(commands: &[CommandShape]) -> String {
    let entries = commands
        .iter()
        .map(|command| {
            format!(
                "{{\"kind\":{},\"fields\":{},\"sources\":{}}}",
                json_string(&command.kind),
                json_string_array(&command.fields),
                json_string_array(&command.sources)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn command_shape_map(commands: &[CommandShape]) -> BTreeMap<String, CommandShapeData> {
    let mut shapes = BTreeMap::new();
    merge_command_shapes(&mut shapes, commands.iter().cloned());
    shapes
}

fn merge_command_shapes(
    shapes: &mut BTreeMap<String, CommandShapeData>,
    commands: impl IntoIterator<Item = CommandShape>,
) {
    merge_command_shapes_with_source(shapes, commands, None);
}

fn merge_command_shapes_with_source(
    shapes: &mut BTreeMap<String, CommandShapeData>,
    commands: impl IntoIterator<Item = CommandShape>,
    source: Option<&str>,
) {
    for command in commands {
        let entry = shapes.entry(command.kind).or_default();
        entry.fields.extend(command.fields);
        entry.sources.extend(command.sources);
        if let Some(source) = source {
            entry.sources.insert(source.to_string());
        }
    }
}

fn command_shapes_from_map(shapes: BTreeMap<String, CommandShapeData>) -> Vec<CommandShape> {
    shapes
        .into_iter()
        .map(|(kind, data)| CommandShape {
            kind,
            fields: data.fields.into_iter().collect(),
            sources: data.sources.into_iter().collect(),
        })
        .collect()
}

fn pair_array_json(pairs: &[(String, String)]) -> String {
    let entries = pairs
        .iter()
        .map(|(key, value)| format!("[{},{}]", json_string(key), json_string(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn json_string_array(values: &[String]) -> String {
    let entries = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn optional_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn build_report_json(
    entry: &Path,
    output: Option<&PathBuf>,
    ok: bool,
    artifacts: &[BuildArtifact],
    diagnostics: &[CollectedDiagnostic],
    error: Option<&str>,
) -> String {
    let error = error
        .map(|error| format!(",\"error\":{}", json_string(error)))
        .unwrap_or_default();
    format!(
        "{{\"ok\":{},\"entry\":{},\"output\":{},\"artifacts\":{},\"diagnostics\":{}{}}}",
        ok,
        json_path(entry),
        output
            .map(|path| json_path(path))
            .unwrap_or_else(|| "null".to_string()),
        build_artifacts_json(artifacts),
        diagnostics_array_json(diagnostics),
        error
    )
}

fn build_artifacts_json(artifacts: &[BuildArtifact]) -> String {
    let entries = artifacts
        .iter()
        .map(|artifact| {
            let runtime_effects = artifact
                .runtime_effects
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            format!(
                "{{\"kind\":{},\"source\":{},\"output\":{},\"sourceMap\":{},\"bytes\":{},\"runtimeEffects\":{}}}",
                json_string(&artifact.kind),
                json_path(&artifact.source),
                json_path(&artifact.output),
                artifact
                    .source_map
                    .as_ref()
                    .map(|path| json_path(path))
                    .unwrap_or_else(|| "null".to_string()),
                artifact.bytes,
                json_string_array(&runtime_effects)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn json_path(path: &Path) -> String {
    json_string(&output_path_string(
        &fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
    ))
}

fn diagnostics_json(diagnostics: &[CollectedDiagnostic]) -> String {
    format!(
        "{{\"diagnostics\":{}}}",
        diagnostics_array_json(diagnostics)
    )
}

fn diagnostics_array_json(diagnostics: &[CollectedDiagnostic]) -> String {
    let entries = diagnostics
        .iter()
        .map(|diagnostic| {
            let expected = diagnostic
                .expected
                .as_ref()
                .map(|expected| format!(",\"expected\":{}", json_string(expected)))
                .unwrap_or_default();
            let actual = diagnostic
                .actual
                .as_ref()
                .map(|actual| format!(",\"actual\":{}", json_string(actual)))
                .unwrap_or_default();
            format!(
                "{{\"file\":{},\"code\":{},\"severity\":{},\"message\":{}{}{},\"span\":{{\"start\":{},\"end\":{}}},\"range\":{{\"start\":{{\"line\":{},\"column\":{}}},\"end\":{{\"line\":{},\"column\":{}}}}},\"fixes\":{}}}",
                json_string(&diagnostic.file),
                json_string(&diagnostic.code),
                json_string(severity_name(&diagnostic.severity)),
                json_string(&diagnostic.message),
                expected,
                actual,
                diagnostic.start,
                diagnostic.end,
                diagnostic.line,
                diagnostic.column,
                diagnostic.end_line,
                diagnostic.end_column,
                diagnostic_fixes_json(&diagnostic.fixes)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn diagnostic_details(message: &str) -> DiagnosticDetails {
    let (code, expected, actual) = if let Some((expected, actual)) =
        parse_expected_actual(message, "type mismatch: expected ", ", found ")
    {
        (
            "clsk/type-mismatch".to_string(),
            Some(expected),
            Some(actual),
        )
    } else if let Some((expected, actual)) =
        parse_expected_actual(message, "function arity mismatch: expected ", ", found ")
    {
        (
            "clsk/function-arity".to_string(),
            Some(expected),
            Some(actual),
        )
    } else if let Some((expected, actual)) = parse_expects_found(message) {
        ("clsk/arity".to_string(), Some(expected), Some(actual))
    } else if message.starts_with("unknown symbol `") {
        ("clsk/unknown-symbol".to_string(), None, None)
    } else if message
        .contains(" is a browser API; pure code must return typed command data instead")
    {
        ("clsk/effect-browser-api".to_string(), None, None)
    } else if message.contains("requires TrustedHtml") {
        ("clsk/trusted-html-required".to_string(), None, None)
    } else if message.starts_with("expected ") || message.starts_with("unexpected ") {
        ("clsk/syntax".to_string(), None, None)
    } else {
        (diagnostic_code_fallback(message), None, None)
    };

    DiagnosticDetails {
        code,
        expected,
        actual,
        fixes: Vec::new(),
    }
}

fn diagnostic_fixes_json(fixes: &[DiagnosticFix]) -> String {
    let entries = fixes
        .iter()
        .map(|fix| {
            let replacement = fix
                .replacement
                .as_ref()
                .map(|replacement| format!(",\"replacement\":{}", json_string(replacement)))
                .unwrap_or_default();
            format!("{{\"title\":{}{} }}", json_string(&fix.title), replacement)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{}]", entries)
}

fn parse_expected_actual(message: &str, prefix: &str, separator: &str) -> Option<(String, String)> {
    let rest = message.strip_prefix(prefix)?;
    let (expected, actual) = rest.split_once(separator)?;
    Some((expected.trim().to_string(), actual.trim().to_string()))
}

fn parse_expects_found(message: &str) -> Option<(String, String)> {
    let (_, rest) = message.split_once(" expects ")?;
    let (expected, actual) = rest.split_once(", found ")?;
    Some((expected.trim().to_string(), actual.trim().to_string()))
}

fn check_cache_probe(path: &Path) -> Result<CacheProbe, String> {
    artifact_cache_probe(path, "check", "cache")
}

fn inspect_cache_probe(path: &Path) -> Result<CacheProbe, String> {
    artifact_cache_probe(path, "inspect", "json")
}

fn artifact_cache_probe(
    path: &Path,
    artifact: &str,
    extension: &str,
) -> Result<CacheProbe, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    let mut modules = BTreeMap::new();
    let mut visiting = HashSet::new();
    collect_module_fingerprints(&canonical, &mut modules, &mut visiting)?;

    let mut key_input = String::new();
    push_current_compiler_fingerprint(&mut key_input);
    key_input.push_str("entry=");
    key_input.push_str(&cache_path_string(&canonical));
    key_input.push('\n');
    for fingerprint in modules.values() {
        key_input.push_str("module=");
        key_input.push_str(&cache_path_string(&fingerprint.canonical));
        key_input.push('\n');
        key_input.push_str("hash=");
        key_input.push_str(&fingerprint.source_hash);
        key_input.push('\n');
        for import in &fingerprint.imports {
            key_input.push_str("import=");
            key_input.push_str(import);
            key_input.push('\n');
        }
    }

    let key = stable_hash_hex(key_input.as_bytes());
    let cache_file = cache_root_for(&canonical)
        .join(artifact)
        .join(format!("{}.{}", key, extension));
    Ok(CacheProbe {
        key,
        module_count: modules.len(),
        cache_file,
    })
}

fn push_current_compiler_fingerprint(output: &mut String) {
    let Ok(exe) = env::current_exe() else {
        return;
    };
    output.push_str("compiler=");
    output.push_str(&cache_path_string(&exe));
    output.push('\n');
    if let Ok(metadata) = fs::metadata(&exe) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                output.push_str("compilerMtime=");
                output.push_str(&duration.as_nanos().to_string());
                output.push('\n');
            }
        }
    }
}

fn collect_module_fingerprints(
    path: &Path,
    modules: &mut BTreeMap<PathBuf, ModuleFingerprint>,
    visiting: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    if modules.contains_key(&canonical) {
        return Ok(());
    }
    if !visiting.insert(canonical.clone()) {
        return Err(format!("cyclic import while caching {}", path.display()));
    }

    let input = fs::read_to_string(&canonical)
        .map_err(|err| format!("failed to read {}: {}", canonical.display(), err))?;
    let source = parse_source(&input);
    if source.has_errors() {
        return Err(format!("parse errors in {}", canonical.display()));
    }

    let imports = parse_imports_silent(&source)?;
    let mut import_fingerprints = Vec::new();
    for import in imports
        .iter()
        .filter(|import| is_closkell_import_path(&import.path))
    {
        let resolved = resolve_import_source(&canonical, &import.path)?;
        let resolved_canonical = fs::canonicalize(&resolved)
            .map_err(|err| format!("failed to resolve {}: {}", resolved.display(), err))?;
        import_fingerprints.push(format!(
            "{}=>{}",
            import.path,
            cache_path_string(&resolved_canonical)
        ));
        collect_module_fingerprints(&resolved_canonical, modules, visiting)?;
    }
    import_fingerprints.sort();
    visiting.remove(&canonical);

    modules.insert(
        canonical.clone(),
        ModuleFingerprint {
            canonical,
            source_hash: stable_hash_hex(input.as_bytes()),
            imports: import_fingerprints,
        },
    );
    Ok(())
}

fn parse_imports_silent(source: &SourceFile) -> Result<Vec<ImportSpec>, String> {
    let mut imports = Vec::new();
    for form in &source.forms {
        if let Some(import) = parse_import_form(form) {
            match import {
                Ok(import) => imports.push(import),
                Err(diagnostic) => return Err(diagnostic.message),
            }
        }
    }
    Ok(imports)
}

fn read_check_cache(probe: &CacheProbe) -> Option<CachedCheck> {
    let text = fs::read_to_string(&probe.cache_file).ok()?;
    let (header, diagnostics_json) = text.split_once("\n\n")?;
    let mut ok = None;
    let mut key = None;
    for line in header.lines() {
        if let Some(value) = line.strip_prefix("ok=") {
            ok = Some(value == "true");
        } else if let Some(value) = line.strip_prefix("key=") {
            key = Some(value);
        }
    }
    if key != Some(probe.key.as_str()) {
        return None;
    }
    Some(CachedCheck {
        ok: ok?,
        diagnostics_json: diagnostics_json.to_string(),
    })
}

fn write_check_cache(probe: &CacheProbe, ok: bool, diagnostics_json: &str) -> Result<(), String> {
    if let Some(parent) = probe.cache_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
    }
    let text = format!("key={}\nok={}\n\n{}", probe.key, ok, diagnostics_json);
    fs::write(&probe.cache_file, text)
        .map_err(|err| format!("failed to write {}: {}", probe.cache_file.display(), err))
}

fn read_inspect_cache(probe: &CacheProbe) -> Option<CachedInspect> {
    let text = fs::read_to_string(&probe.cache_file).ok()?;
    let (header, report_json) = text.split_once("\n\n")?;
    let mut key = None;
    for line in header.lines() {
        if let Some(value) = line.strip_prefix("key=") {
            key = Some(value);
        }
    }
    if key != Some(probe.key.as_str()) {
        return None;
    }
    Some(CachedInspect {
        report_json: report_json.to_string(),
    })
}

fn write_inspect_cache(probe: &CacheProbe, report_json: &str) -> Result<(), String> {
    if let Some(parent) = probe.cache_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
    }
    let text = format!("key={}\n\n{}", probe.key, report_json);
    fs::write(&probe.cache_file, text)
        .map_err(|err| format!("failed to write {}: {}", probe.cache_file.display(), err))
}

fn cache_root_for(path: &Path) -> PathBuf {
    project_root_for(path).join(".closkell").join("cache")
}

fn project_root_for(path: &Path) -> PathBuf {
    let start = path.parent().unwrap_or_else(|| Path::new("."));
    for ancestor in start.ancestors() {
        if ancestor.join("package.json").is_file() || ancestor.join("Cargo.toml").is_file() {
            return ancestor.to_path_buf();
        }
    }
    for ancestor in start.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == "src") {
            if let Some(parent) = ancestor.parent() {
                return parent.to_path_buf();
            }
        }
    }
    start.to_path_buf()
}

fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn cache_path_string(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn diagnostic_code_fallback(message: &str) -> String {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in message.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            words.push(current);
            current = String::new();
        }
        if words.len() >= 5 {
            break;
        }
    }
    if !current.is_empty() && words.len() < 5 {
        words.push(current);
    }
    if words.is_empty() {
        "clsk-diagnostic".to_string()
    } else {
        format!("clsk-{}", words.join("-"))
    }
}

fn severity_name(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn output_path_string(path: &Path) -> String {
    let value = path.display().to_string();
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{}", rest);
    }
    if let Some(rest) = value.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    value
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}

fn matches_symbol(expr: &Expr, expected: &str) -> bool {
    matches!(&expr.kind, ExprKind::Symbol(name) if name == expected)
}

fn parse_check_file(
    path: &Path,
    source_override: Option<&SourceOverride>,
) -> Result<(String, SourceFile), String> {
    if let Some(source_override) = source_override {
        let canonical = fs::canonicalize(path)
            .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
        if canonical == source_override.canonical {
            return Ok((
                source_override.input.clone(),
                parse_source(&source_override.input),
            ));
        }
    }
    parse_file(path)
}

fn parse_file(path: &Path) -> Result<(String, SourceFile), String> {
    let input = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {}", path.display(), err))?;
    let source = parse_source(&input);
    Ok((input, source))
}

fn read_stdin() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("failed to read stdin: {}", err))?;
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reachable_update_messages_include_literal_command_errors() {
        let source = parse_source(
            r#"
            (def initialState {:error nil})
            (defn init [] [initialState {:kind :start}])
            (defn view [state] #html <button></button>)
            (defn subscriptions [state] Sub.none)
            (defn loadCommand []
              {:kind :http/request
               :url "/missing.json"
               :toMessage (fn [response] {:kind :loaded :value response})
               :onError :failed})
            (defn update [state msg]
              (match msg
                {:kind :start} [state (loadCommand)]
                {:kind :loaded :value response} [(merge state {:loaded response}) Cmd.none]
                {:kind :failed :error error} [(merge state {:error error}) Cmd.none]
                _ [state Cmd.none]))
            "#,
        );

        let reachable = collect_reachable_update_message_kinds(
            &source,
            &HashMap::new(),
            &js_backend::EmitOptions::default(),
        )
        .expect("update match should be recognized");

        assert!(reachable.contains("start"));
        assert!(reachable.contains("loaded"));
        assert!(reachable.contains("failed"));
    }

    #[test]
    fn tailored_runtime_discovers_referenced_declarations() {
        let runtime = r#"
export function alpha() {
  return beta() + gamma();
}

function beta() {
  return 1;
}

const gamma = () => 2;

export function unused() {
  return 0;
}
"#;
        let required = BTreeSet::from(["alpha".to_string()]);
        let tailored =
            tailored_runtime_source(runtime, &required).expect("runtime should be tailored");

        assert!(tailored.contains("export function alpha"));
        assert!(tailored.contains("function beta"));
        assert!(tailored.contains("const gamma"));
        assert!(!tailored.contains("export function unused"));
    }

    #[test]
    fn tailored_runtime_ignores_local_bindings_that_match_declarations() {
        let runtime = r#"
export function alpha(options = {}) {
  const { subscriptions = () => null, dispatch } = options;
  function run(command) {
    const { commands = [] } = command;
    return commands.length + subscriptions().length + Number(Boolean(dispatch));
  }
  return run({ commands: [] });
}

export function subscriptions() {
  return ["test helper"];
}

export function dispatch() {
  return "test helper";
}

export function commands() {
  return ["test helper"];
}
"#;
        let required = BTreeSet::from(["alpha".to_string()]);
        let tailored =
            tailored_runtime_source(runtime, &required).expect("runtime should be tailored");

        assert!(tailored.contains("export function alpha"));
        assert!(!tailored.contains("export function subscriptions"));
        assert!(!tailored.contains("export function dispatch"));
        assert!(!tailored.contains("export function commands"));
    }

    #[test]
    fn tailored_browser_template_runtime_omits_test_document_fallback() {
        let runtime_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("runtime-js")
            .join("src")
            .join("index.js");
        let runtime =
            fs::read_to_string(&runtime_path).expect("workspace runtime source should be readable");
        let required = BTreeSet::from(["createBrowserCompiledHtmlTemplateComponent".to_string()]);
        let tailored =
            tailored_runtime_source(&runtime, &required).expect("runtime should be tailored");

        assert!(tailored.contains("export function createBrowserCompiledHtmlTemplateComponent"));
        assert!(tailored.contains("function browserRuntimeDocument"));
        assert!(!tailored.contains("function ensureRuntimeDocument"));
        assert!(!tailored.contains("class CloskellTestElement"));
        assert!(!tailored.contains("function parseTestHtmlFragment"));
    }

    #[test]
    fn tailored_compiled_app_runtime_keeps_subscription_stop_helpers() {
        let runtime_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("runtime-js")
            .join("src")
            .join("index.js");
        let runtime =
            fs::read_to_string(&runtime_path).expect("workspace runtime source should be readable");
        let required = BTreeSet::from(["startCompiledApp".to_string()]);
        let tailored =
            tailored_runtime_source(&runtime, &required).expect("runtime should be tailored");

        assert!(tailored.contains("export function startCompiledApp"));
        assert!(tailored.contains("function compiledStartCommandForSubscription"));
        assert!(tailored.contains("function compiledStopCommandForSubscription"));
    }
}

fn print_help() {
    println!(
        "closkell commands:\n  check <file> [--types] [--json] [--stdin] [--cache-debug]\n  build <file> [-o out.js] [--sourcemap] [--json] [--app] [--root id] [--css path] [--vendor-runtime]\n  expand <file>\n  fmt <file> [--stdin]\n  inspect <file> [--cache-debug]\n  test <file> [--json]\n  dev --watch <file> [--out out.js] [--sourcemap] [--app] [--root id] [--css path] [--vendor-runtime] [--poll-ms ms] [--once]"
    );
}
