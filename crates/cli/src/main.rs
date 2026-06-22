use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    env, fs,
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode},
    thread,
    time::{Duration, SystemTime},
};

use syntax::{Diagnostic, Expr, ExprKind, SourceFile, parse_source, render_diagnostics};
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
            let print_forms = has_flag(&args, "--types") || has_flag(&args, "--verbose");
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            check_file(&path, &mut modules, &mut checking, print_forms)?;
            Ok(())
        }
        "expand" => {
            let path = require_path(&args)?;
            let (input, source) = parse_file(&path)?;
            let imports = parse_imports(&input, &source)?;
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            for import in &imports {
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
            let source_maps = has_flag(&args, "--sourcemap") || has_flag(&args, "--source-map");
            let app = parse_app_options(&args)?;
            if source_maps && output.is_none() {
                return Err("build --sourcemap expects --out".to_string());
            }
            if app.is_some() && output.is_none() {
                return Err("build --app expects --out".to_string());
            }
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            let module = check_file(&path, &mut modules, &mut checking, false)?;
            if app.is_some() {
                require_app_exports(&path, &module.exports)?;
            }

            if let Some(output) = output {
                let mut visited = HashSet::new();
                let options = BuildOptions { source_maps, app };
                build_file(&path, &output, &mut visited, &options, &modules)?;
            } else {
                let emitted = build_single_module(&path, &modules)?;
                print!("{}", emitted);
            }
            Ok(())
        }
        "fmt" => {
            let path = require_path(&args)?;
            let (_, source) = parse_file(&path)?;
            println!("{}", source.pretty());
            Ok(())
        }
        "inspect" => {
            let path = require_path(&args)?;
            let mut modules = HashMap::new();
            let mut checking = HashSet::new();
            check_file(&path, &mut modules, &mut checking, false)?;
            let report = inspect_file(&path, &modules)?;
            println!("{}", report);
            Ok(())
        }
        "test" => {
            let path = require_path(&args)?;
            run_module_tests(&path)
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

fn require_path(args: &[String]) -> Result<PathBuf, String> {
    args.get(1)
        .map(PathBuf::from)
        .ok_or_else(|| "expected a source file path".to_string())
}

fn require_check_path(args: &[String]) -> Result<PathBuf, String> {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| "check expects a source file path".to_string())
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

fn run_module_tests(path: &Path) -> Result<(), String> {
    let suffix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let temp_dir = env::temp_dir().join(format!("closkell-test-{}-{}", std::process::id(), suffix));

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("failed to create {}: {}", temp_dir.display(), err))?;

    let result = run_module_tests_in_temp(path, &temp_dir);
    let _ = fs::remove_dir_all(&temp_dir);
    result
}

fn run_module_tests_in_temp(path: &Path, temp_dir: &Path) -> Result<(), String> {
    let mut modules = HashMap::new();
    let mut checking = HashSet::new();
    check_file(path, &mut modules, &mut checking, false)?;

    copy_runtime_package(temp_dir)?;
    let output = temp_dir.join("__closkell_test_entry.mjs");
    let mut visited = HashSet::new();
    build_file(
        path,
        &output,
        &mut visited,
        &BuildOptions {
            source_maps: false,
            app: None,
        },
        &modules,
    )?;
    run_node_test_module(&output)
}

fn run_node_test_module(output: &Path) -> Result<(), String> {
    let run = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(test_runner_script(output))
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
const tests = module.tests;

if (!Array.isArray(tests)) {{
  console.error("expected module to export `tests` as a vector of records");
  process.exit(2);
}}

if (tests.length === 0) {{
  console.error("expected module `tests` to contain at least one test");
  process.exit(2);
}}

function symbolKey(value) {{
  if (typeof value !== "symbol") return null;
  return Symbol.keyFor(value) ?? value.description ?? "";
}}

function isObject(value) {{
  return value !== null && typeof value === "object";
}}

function deepEqual(left, right) {{
  if (Object.is(left, right)) return true;
  if (typeof left === "symbol" || typeof right === "symbol") {{
    return symbolKey(left) === symbolKey(right);
  }}
  if (Array.isArray(left) || Array.isArray(right)) {{
    if (!Array.isArray(left) || !Array.isArray(right)) return false;
    if (left.length !== right.length) return false;
    return left.every((value, index) => deepEqual(value, right[index]));
  }}
  if (isObject(left) || isObject(right)) {{
    if (!isObject(left) || !isObject(right)) return false;
    const leftKeys = Object.keys(left).sort();
    const rightKeys = Object.keys(right).sort();
    if (!deepEqual(leftKeys, rightKeys)) return false;
    return leftKeys.every((key) => deepEqual(left[key], right[key]));
  }}
  return false;
}}

function formatValue(value) {{
  if (typeof value === "symbol") return ":" + symbolKey(value);
  if (typeof value === "undefined") return "undefined";
  return JSON.stringify(
    value,
    (_key, next) => (typeof next === "symbol" ? ":" + symbolKey(next) : next)
  );
}}

function testName(test, index) {{
  if (test && typeof test.name === "string" && test.name.length > 0) {{
    return test.name;
  }}
  return `test ${{index + 1}}`;
}}

let failed = 0;
for (const [index, test] of tests.entries()) {{
  const name = testName(test, index);
  if (!test || typeof test !== "object") {{
    failed += 1;
    console.error(`not ok ${{index + 1}} - ${{name}}`);
    console.error("  expected a test record with actual and expected fields");
    continue;
  }}
  if (!("actual" in test) || !("expected" in test)) {{
    failed += 1;
    console.error(`not ok ${{index + 1}} - ${{name}}`);
    console.error("  expected fields: actual, expected");
    continue;
  }}
  if (deepEqual(test.actual, test.expected)) {{
    console.log(`ok ${{index + 1}} - ${{name}}`);
  }} else {{
    failed += 1;
    console.error(`not ok ${{index + 1}} - ${{name}}`);
    console.error(`  expected ${{formatValue(test.expected)}}`);
    console.error(`  actual   ${{formatValue(test.actual)}}`);
  }}
}}

if (failed > 0) {{
  console.error(`${{failed}}/${{tests.length}} tests failed`);
  process.exit(1);
}}

console.log(`ok ${{tests.length}} tests`);
"#
    )
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
    build_file(source, output, &mut visited, &build_options, &modules)
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
    let canonical = fs::canonicalize(path)
        .map_err(|err| format!("failed to resolve {}: {}", path.display(), err))?;
    if let Some(info) = modules.get(&canonical) {
        return Ok(info.clone());
    }
    if !checking.insert(canonical.clone()) {
        return Err(format!("cyclic import while checking {}", path.display()));
    }

    let result = check_file_inner(path, modules, checking, print_forms);
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
) -> Result<ModuleInfo, String> {
    let (input, source) = parse_file(path)?;
    print_parse_diagnostics(&input, &source);
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
        let import_source = resolve_import_source(path, &import.path)?;
        let imported = check_file(&import_source, modules, checking, false)?;
        for name in &import.names {
            if !imported.exports.contains(&name.name) {
                import_diagnostics.push(Diagnostic::error(
                    name.span,
                    format!("import `{}` is not exported by {}", name.name, import.path),
                ));
                continue;
            }
            if let Some(binding) = imported
                .bindings
                .iter()
                .find(|binding| binding.name == name.name && binding.is_annotated())
            {
                let binding = binding.import_as(name.name.clone());
                if binding.returns_cmd() {
                    if let Some(shapes) = imported.command_shapes_by_binding.get(&name.name) {
                        imported_command_shapes.insert(name.name.clone(), shapes.clone());
                    }
                }
                import_bindings.push(binding);
            }
            if let Some(declaration) = imported
                .type_declarations
                .iter()
                .find(|declaration| declaration.name == name.name)
            {
                import_type_declarations.push(declaration.import_as(name.name.clone()));
            }
            if let Some(macro_def) = imported.macros.get(&name.name) {
                imported_macros.insert(name.name.clone(), macro_def.clone());
            }
        }
    }
    if !import_diagnostics.is_empty() {
        println!("{}", render_diagnostics(&input, &import_diagnostics));
    }

    let local_macros = macro_expand::collect_macro_defs(&source).macros;
    let expansion = macro_expand::expand_source_with_imported_macros(&source, &imported_macros);
    if !expansion.diagnostics.is_empty() {
        println!("{}", render_diagnostics(&input, &expansion.diagnostics));
    }

    let type_result = typecheck::check_source_with_module_imports(
        &expansion.source,
        &import_bindings,
        &import_type_declarations,
    );
    if !type_result.diagnostics.is_empty() {
        println!("{}", render_diagnostics(&input, &type_result.diagnostics));
    }

    let imported_command_helpers = import_bindings
        .iter()
        .filter(|binding| binding.returns_cmd())
        .map(|binding| binding.name.clone())
        .collect::<HashSet<_>>();
    let effect_report = effects::validate_purity_with_imported_command_helpers(
        &expansion.source,
        &imported_command_helpers,
    );
    if !effect_report.diagnostics.is_empty() {
        println!("{}", render_diagnostics(&input, &effect_report.diagnostics));
    }

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
    }
    if let Some(app) = &options.app {
        if app.vendor_runtime {
            copy_runtime_package(&runtime_vendor_root(output))?;
        }
    }
    Ok(())
}

fn import_has_runtime_names(
    source_path: &Path,
    import: &ImportSpec,
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<bool, String> {
    let import_source = resolve_import_source(source_path, &import.path)?;
    let canonical = fs::canonicalize(&import_source)
        .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
    let imported = modules.get(&canonical);
    Ok(import.names.iter().any(|name| {
        js_backend::is_runtime_import_name(&name.name)
            && !imported.is_some_and(|module| module.macros.contains_key(&name.name))
    }))
}

fn remove_stale_type_only_output(
    source_path: &Path,
    import_path: &str,
    output: &Path,
    visited: &HashSet<PathBuf>,
) -> Result<(), String> {
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
        "import { createCommandHandlers as __closkellCreateCommandHandlers, createDevtoolsOverlay as __closkellCreateDevtoolsOverlay, startApp as __closkellStartApp } from \"@closkell/runtime\";\n",
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
    code.push_str("export const __closkellApp = __closkellStartApp({\n");
    code.push_str("  root: __closkellRoot,\n");
    code.push_str("  init,\n");
    code.push_str("  update,\n");
    code.push_str("  view,\n");
    code.push_str("  handlers: __closkellCreateCommandHandlers(),\n");
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
    name: String,
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
        let ExprKind::Symbol(symbol) = &name.kind else {
            return Some(Err(Diagnostic::error(
                name.span,
                "imported name must be a symbol",
            )));
        };
        imported.push(ImportName {
            name: symbol.clone(),
            span: name.span,
        });
    }

    Some(Ok(ImportSpec {
        path: path.clone(),
        names: imported,
    }))
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
            "only same-tree relative .clsk imports are supported: {}",
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
    Ok(render_inspection_json(
        path,
        &exports,
        &type_report.declarations,
        &annotation_report.annotations,
        &templates,
        &commands,
    ))
}

fn imported_macros_from_imports(
    path: &Path,
    imports: &[ImportSpec],
    modules: &HashMap<PathBuf, ModuleInfo>,
) -> Result<HashMap<String, macro_expand::MacroDef>, String> {
    let mut macros = HashMap::new();
    for import in imports {
        let import_source = resolve_import_source(path, &import.path)?;
        let canonical = fs::canonicalize(&import_source)
            .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
        let Some(imported) = modules.get(&canonical) else {
            continue;
        };
        for name in &import.names {
            if let Some(macro_def) = imported.macros.get(&name.name) {
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
        let import_source = resolve_import_source(path, &import.path)?;
        let canonical = fs::canonicalize(&import_source)
            .map_err(|err| format!("failed to resolve {}: {}", import_source.display(), err))?;
        let Some(imported) = modules.get(&canonical) else {
            continue;
        };
        for name in &import.names {
            if let Some(binding_shapes) = imported.command_shapes_by_binding.get(&name.name) {
                merge_command_shapes(&mut shapes, binding_shapes.iter().cloned());
            }
        }
    }
    Ok(command_shapes_from_map(shapes))
}

fn render_inspection_json(
    path: &Path,
    exports: &[String],
    types: &[typecheck::TypeDeclaration],
    annotations: &[typecheck::TypeAnnotation],
    templates: &[NamedTemplate],
    commands: &[CommandShape],
) -> String {
    let mut lines = Vec::new();
    lines.push("{".to_string());
    lines.push(format!(
        "  \"file\": {},",
        json_string(&path.display().to_string())
    ));
    lines.push(format!("  \"exports\": {},", json_string_array(exports)));
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
        "  \"componentGraph\": {},",
        component_graph_json(templates)
    ));
    lines.push(format!(
        "  \"statePathToSlots\": {},",
        state_path_to_slots_json(templates)
    ));
    lines.push(format!("  \"templates\": {}", templates_json(templates)));
    lines.push("}".to_string());
    lines.join("\n")
}

fn types_json(types: &[typecheck::TypeDeclaration]) -> String {
    let entries = types
        .iter()
        .map(|ty| {
            format!(
                "{{\"name\":{},\"schema\":{}}}",
                json_string(&ty.name),
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
                        uses.push(name.clone());
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

fn collect_command_shapes(source: &SourceFile) -> Vec<CommandShape> {
    let mut shapes: BTreeMap<String, CommandShapeData> = BTreeMap::new();
    for form in &source.forms {
        let source_name = definition_name(form).unwrap_or("module");
        collect_command_shapes_expr(form, &mut shapes, source_name);
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
        ExprKind::List(items) | ExprKind::Vector(items) | ExprKind::Set(items) => {
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

fn parse_file(path: &Path) -> Result<(String, SourceFile), String> {
    let input = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {}", path.display(), err))?;
    let source = parse_source(&input);
    Ok((input, source))
}

fn print_parse_diagnostics(input: &str, source: &SourceFile) {
    if !source.diagnostics.is_empty() {
        println!("{}", render_diagnostics(input, &source.diagnostics));
    }
}

fn print_help() {
    println!(
        "closkell commands:\n  check <file> [--types]\n  build <file> [-o out.js] [--sourcemap] [--app] [--root id] [--css path] [--vendor-runtime]\n  expand <file>\n  fmt <file>\n  inspect <file>\n  test <file>\n  dev --watch <file> [--out out.js] [--sourcemap] [--app] [--root id] [--css path] [--vendor-runtime] [--poll-ms ms] [--once]"
    );
}
