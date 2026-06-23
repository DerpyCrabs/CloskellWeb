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
                let options = BuildOptions { source_maps, app };
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
                let emitted = build_single_module(&path, &modules)?;
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

#[derive(Clone, Debug, Default)]
struct ModuleInfo {
    exports: HashSet<String>,
    bindings: Vec<typecheck::ExportedBinding>,
    type_declarations: Vec<typecheck::TypeDeclaration>,
    macros: HashMap<String, macro_expand::MacroDef>,
    command_shapes_by_binding: HashMap<String, Vec<CommandShape>>,
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
}

#[derive(Clone, Debug)]
struct BuildOptions {
    source_maps: bool,
    app: Option<AppOptions>,
}

#[derive(Clone, Debug)]
struct BuildArtifact {
    kind: String,
    source: PathBuf,
    output: PathBuf,
    source_map: Option<PathBuf>,
    bytes: u64,
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
        r#"import {{ pathToFileURL }} from "node:url";

const modulePath = {module_path};
const moduleUrl = pathToFileURL(modulePath).href;
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
        r#"import {{ pathToFileURL }} from "node:url";

const modulePath = {module_path};
const moduleUrl = pathToFileURL(modulePath).href;
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
    copy_runtime_file(
        &source_dir.join("src").join("index.js"),
        &package_dir.join("src").join("index.js"),
    )
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
            if let Some(binding) = imported
                .bindings
                .iter()
                .find(|binding| binding.name == name.imported && binding.is_annotated())
            {
                let binding = binding.import_as(name.name.clone());
                if binding.returns_cmd() {
                    if let Some(shapes) = imported.command_shapes_by_binding.get(&name.imported) {
                        imported_command_shapes.insert(name.name.clone(), shapes.clone());
                    }
                }
                import_bindings.push(binding);
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

    Ok(ModuleInfo {
        exports,
        bindings: type_result.bindings,
        type_declarations: type_result.type_declarations,
        macros: local_macros,
        command_shapes_by_binding,
    })
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
    if !visited.insert(canonical) {
        return Ok(());
    }

    let (input, source) = parse_file(path)?;
    print_parse_diagnostics(&input, &source);
    if source.has_errors() {
        return Err(format!("build failed during parsing: {}", path.display()));
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
            },
            modules,
            artifacts,
            "import",
        )?;
    }

    let mut emitted = emit_checked_module(path, &input, &source, modules)?;
    if let Some(app) = &options.app {
        wrap_app_module(&mut emitted, app);
    }
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
            copy_runtime_package(&runtime_vendor_root(output))?;
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
) -> Result<String, String> {
    let (input, source) = parse_file(path)?;
    print_parse_diagnostics(&input, &source);
    if source.has_errors() {
        return Err("build failed during parsing".to_string());
    }
    emit_checked_module(path, &input, &source, modules).map(|emitted| emitted.code)
}

fn emit_checked_module(
    path: &Path,
    input: &str,
    source: &SourceFile,
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<js_backend::EmitResult, String> {
    let imports = parse_imports(input, source)?;
    let imported_macros = imported_macros_from_imports(path, &imports, modules)?;
    let expansion = macro_expand::expand_source_with_imported_macros(source, &imported_macros);
    if !expansion.diagnostics.is_empty() {
        println!("{}", render_diagnostics(input, &expansion.diagnostics));
        return Err(format!(
            "build failed during macro expansion: {}",
            path.display()
        ));
    }

    let emitted = js_backend::emit_module(&expansion.source);
    if !emitted.diagnostics.is_empty() {
        println!("{}", render_diagnostics(input, &emitted.diagnostics));
        return Err(format!(
            "build failed during JS emission: {}",
            path.display()
        ));
    }

    Ok(emitted)
}

fn wrap_app_module(emitted: &mut js_backend::EmitResult, options: &AppOptions) {
    let prelude = app_bootstrap_prelude(options);
    let postlude = app_bootstrap_postlude(options);
    let inserted_lines = prelude.lines().count();
    for mapping in &mut emitted.source_mappings {
        mapping.generated_line += inserted_lines;
    }
    if !emitted.code.ends_with('\n') {
        emitted.code.push('\n');
    }
    emitted.code = format!("{}{}{}", prelude, emitted.code, postlude);
}

fn app_bootstrap_prelude(options: &AppOptions) -> String {
    let mut code = String::new();
    code.push_str(
        "import { createBrowserBootInput as __closkellCreateBrowserBootInput, createCommandHandlers as __closkellCreateCommandHandlers, createSubscriptionHandlers as __closkellCreateSubscriptionHandlers, createDevtoolsOverlay as __closkellCreateDevtoolsOverlay, startApp as __closkellStartApp } from \"@closkell/runtime\";\n",
    );
    if let Some(css) = &options.css {
        code.push_str("import ");
        code.push_str(&json_string(css));
        code.push_str(";\n");
    }
    code
}

fn app_bootstrap_postlude(options: &AppOptions) -> String {
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
        "const __closkellDevtools = globalThis.__closkellDevtools ?? (globalThis.__closkellDevtoolsOverlay ? __closkellCreateDevtoolsOverlay(globalThis.__closkellDevtoolsOverlay) : null);\n",
    );
    code.push_str("if (__closkellDevtools && globalThis.__closkellDevtoolsOverlay && !globalThis.__closkellDevtoolsOverlayInstance) {\n");
    code.push_str("  globalThis.__closkellDevtoolsOverlayInstance = __closkellDevtools;\n");
    code.push_str("}\n");
    code.push_str("const __closkellHandlers = __closkellCreateCommandHandlers();\n");
    code.push_str("export const __closkellApp = __closkellStartApp({\n");
    code.push_str("  root: __closkellRoot,\n");
    code.push_str("  init,\n");
    code.push_str("  update,\n");
    code.push_str("  view,\n");
    code.push_str("  boot: __closkellCreateBrowserBootInput(),\n");
    code.push_str(
        "  subscriptions: typeof subscriptions === \"function\" ? subscriptions : undefined,\n",
    );
    code.push_str("  handlers: __closkellHandlers,\n");
    code.push_str("  subscriptionHandlers: __closkellCreateSubscriptionHandlers({ commandHandlers: __closkellHandlers }),\n");
    code.push_str("  devtools: __closkellDevtools\n");
    code.push_str("});\n\n");
    code
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
            format!(
                "{{\"kind\":{},\"source\":{},\"output\":{},\"sourceMap\":{},\"bytes\":{}}}",
                json_string(&artifact.kind),
                json_path(&artifact.source),
                json_path(&artifact.output),
                artifact
                    .source_map
                    .as_ref()
                    .map(|path| json_path(path))
                    .unwrap_or_else(|| "null".to_string()),
                artifact.bytes
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

const CHECK_CACHE_VERSION: &str = "closkell-check-cache-v1";
const INSPECT_CACHE_VERSION: &str = "closkell-inspect-cache-v2";

fn check_cache_probe(path: &Path) -> Result<CacheProbe, String> {
    artifact_cache_probe(path, CHECK_CACHE_VERSION, "check", "cache")
}

fn inspect_cache_probe(path: &Path) -> Result<CacheProbe, String> {
    artifact_cache_probe(path, INSPECT_CACHE_VERSION, "inspect", "json")
}

fn artifact_cache_probe(
    path: &Path,
    version: &str,
    artifact: &str,
    extension: &str,
) -> Result<CacheProbe, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    let mut modules = BTreeMap::new();
    let mut visiting = HashSet::new();
    collect_module_fingerprints(&canonical, &mut modules, &mut visiting)?;

    let mut key_input = String::new();
    key_input.push_str(version);
    key_input.push('\n');
    key_input.push_str("compiler=");
    key_input.push_str(env!("CARGO_PKG_VERSION"));
    key_input.push('\n');
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
    let text = format!(
        "{}\nkey={}\nok={}\n\n{}",
        CHECK_CACHE_VERSION, probe.key, ok, diagnostics_json
    );
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
    let text = format!(
        "{}\nkey={}\n\n{}",
        INSPECT_CACHE_VERSION, probe.key, report_json
    );
    fs::write(&probe.cache_file, text)
        .map_err(|err| format!("failed to write {}: {}", probe.cache_file.display(), err))
}

fn cache_root_for(path: &Path) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".closkell")
        .join("cache")
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

fn print_parse_diagnostics(input: &str, source: &SourceFile) {
    if !source.diagnostics.is_empty() {
        println!("{}", render_diagnostics(input, &source.diagnostics));
    }
}

fn print_help() {
    println!(
        "closkell commands:\n  check <file> [--types] [--json] [--stdin] [--cache-debug]\n  build <file> [-o out.js] [--sourcemap] [--json] [--app] [--root id] [--css path] [--vendor-runtime]\n  expand <file>\n  fmt <file> [--stdin]\n  inspect <file> [--cache-debug]\n  test <file> [--json]\n  dev --watch <file> [--out out.js] [--sourcemap] [--app] [--root id] [--css path] [--vendor-runtime] [--poll-ms ms] [--once]"
    );
}
