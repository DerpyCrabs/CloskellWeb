use std::{
    env, fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[test]
fn dev_watch_once_builds_entry_and_imports() {
    let temp_dir = temp_dir("closkell-dev-watch-once");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let dep = temp_dir.join("dep.clsk");
    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("dist").join("app.mjs");
    let dep_output = temp_dir.join("dist").join("dep.mjs");
    let output_map = temp_dir.join("dist").join("app.mjs.map");
    let dep_output_map = temp_dir.join("dist").join("dep.mjs.map");
    fs::write(&dep, "(def value 41)\n").expect("dep should be written");
    fs::write(
        &app,
        "(import \"./dep.clsk\" [value])\n(def summary {:answer (+ value 1)})\n",
    )
    .expect("app should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("dev")
        .arg("--watch")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--sourcemap")
        .arg("--once")
        .output()
        .expect("closkell dev --watch --once should run");

    assert!(
        run.status.success(),
        "dev --watch --once failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(output.exists(), "watch once did not emit app module");
    assert!(
        dep_output.exists(),
        "watch once did not emit imported module"
    );
    assert!(
        output_map.exists(),
        "watch once did not emit app source map"
    );
    assert!(
        dep_output_map.exists(),
        "watch once did not emit imported source map"
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("built"),
        "watch once did not report a build:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );

    let app_js = fs::read_to_string(&output).expect("app module should be readable");
    let dep_js = fs::read_to_string(&dep_output).expect("dep module should be readable");
    assert!(
        app_js.contains("from \"./dep.mjs\""),
        "app module did not import generated dependency:\n{}",
        app_js
    );
    assert!(
        app_js.contains("//# sourceMappingURL=app.mjs.map"),
        "app module did not reference its source map:\n{}",
        app_js
    );
    assert!(
        dep_js.contains("export const value = 41;"),
        "dependency module did not compile expected value:\n{}",
        dep_js
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn dev_watch_once_app_vendors_runtime() {
    let temp_dir = temp_dir("closkell-dev-watch-app-runtime");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("src").join("main.mjs");
    let runtime_entry = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");

    fs::write(temp_dir.join("package.json"), "{\"type\":\"module\"}\n")
        .expect("package.json should be written");
    fs::write(
        &app,
        "(def init {:label \"Ready\"})\n\
         (defn update [state msg] [state {:kind :none}])\n\
         (defn view [state] #html <p>{state.label}</p>)\n",
    )
    .expect("app source should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("dev")
        .arg("--watch")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--app")
        .arg("--vendor-runtime")
        .arg("--once")
        .output()
        .expect("closkell dev --watch --app should run");

    assert!(
        run.status.success(),
        "dev --watch --app failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let entry = fs::read_to_string(&output).expect("app entry should be readable");
    assert!(entry.contains("__closkellStartApp"));
    assert!(
        runtime_entry.is_file(),
        "watch app build did not vendor runtime"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn dev_watch_rebuilds_when_import_changes() {
    let temp_dir = temp_dir("closkell-dev-watch-rebuild");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let dep = temp_dir.join("dep.clsk");
    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("dist").join("app.mjs");
    let dep_output = temp_dir.join("dist").join("dep.mjs");
    fs::write(&dep, "(def value 1)\n").expect("dep should be written");
    fs::write(
        &app,
        "(import \"./dep.clsk\" [value])\n(def summary {:value value})\n",
    )
    .expect("app should be written");

    let child = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("dev")
        .arg("--watch")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--poll-ms")
        .arg("25")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("closkell dev --watch should start");
    let mut child = ChildGuard::new(child);

    wait_for(
        "initial app and dependency build",
        Duration::from_secs(5),
        || {
            output.exists()
                && dep_output.exists()
                && fs::read_to_string(&dep_output)
                    .map(|contents| contents.contains("export const value = 1;"))
                    .unwrap_or(false)
        },
    );

    thread::sleep(Duration::from_millis(100));
    fs::write(&dep, "(def value 2222)\n").expect("dep should update");

    wait_for(
        "dependency rebuild after import change",
        Duration::from_secs(5),
        || {
            fs::read_to_string(&dep_output)
                .map(|contents| contents.contains("export const value = 2222;"))
                .unwrap_or(false)
        },
    );

    child.kill();
    let _ = fs::remove_dir_all(&temp_dir);
}

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn kill(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.kill();
    }
}

fn wait_for(label: &str, timeout: Duration, mut predicate: impl FnMut() -> bool) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if predicate() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for {}", label);
}

fn temp_dir(name: &str) -> PathBuf {
    env::temp_dir().join(format!("{}-{}", name, std::process::id()))
}
