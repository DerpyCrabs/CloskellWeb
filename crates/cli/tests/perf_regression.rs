use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

const CHECK_HRWEB_BUDGET: Duration = Duration::from_millis(1_100);
const NPM_CHECK_HRWEB_BUDGET: Duration = Duration::from_millis(1_200);
const BUILD_HRWEB_BUDGET: Duration = Duration::from_millis(1_100);
const FMT_HRWEB_BUDGET: Duration = Duration::from_millis(200);
const VITE_WARM_BUILD_BUDGET: Duration = Duration::from_millis(500);

#[test]
fn perf_check_hrweb_app_stays_under_budget() {
    let Some(bin) = perf_bin() else {
        return;
    };
    let app = workspace_root()
        .join("projects")
        .join("hrweb")
        .join("src")
        .join("app.clsk");
    let samples = command_samples(3, || {
        let mut command = Command::new(&bin);
        command
            .arg("check")
            .arg(&app)
            .current_dir(workspace_root().join("projects").join("hrweb"));
        command
    });
    assert_under("closkell check hrweb", &samples, CHECK_HRWEB_BUDGET);
}

#[test]
fn perf_npm_check_hrweb_script_stays_under_budget() {
    let Some(bin) = perf_bin() else {
        return;
    };
    let Some(npm) = npm_bin() else {
        eprintln!("skipping npm check perf test because npm was not found");
        return;
    };
    let hrweb = workspace_root().join("projects").join("hrweb");
    let samples = command_samples(3, || {
        let mut command = Command::new(&npm);
        command
            .arg("run")
            .arg("check:closkell")
            .arg("--silent")
            .current_dir(&hrweb)
            .env("CLOSKELL_BIN", &bin);
        command
    });
    assert_under("npm run check:closkell", &samples, NPM_CHECK_HRWEB_BUDGET);
}

#[test]
fn perf_build_hrweb_app_stays_under_budget_and_omits_sourcemaps() {
    let Some(bin) = perf_bin() else {
        return;
    };
    let root = workspace_root();
    let hrweb = root.join("projects").join("hrweb");
    let app = hrweb.join("src").join("app.clsk");
    let temp_dir = env::temp_dir().join(format!("closkell-perf-build-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("perf temp dir should be created");
    let output = temp_dir.join("main.mjs");
    fs::write(temp_dir.join("main.mjs.map"), "{}\n").expect("stale source map should be seeded");

    let samples = command_samples(3, || {
        let mut command = Command::new(&bin);
        command
            .arg("build")
            .arg(&app)
            .arg("--out")
            .arg(&output)
            .arg("--app")
            .arg("--root")
            .arg("root")
            .arg("--css")
            .arg("src/styles.css")
            .arg("--vendor-runtime")
            .current_dir(&hrweb);
        command
    });

    assert_under("closkell build hrweb", &samples, BUILD_HRWEB_BUDGET);
    assert!(output.is_file(), "build did not write {}", output.display());
    assert!(
        !output.with_extension("mjs.map").exists(),
        "non-sourcemap build left a stale source map"
    );
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn perf_fmt_hrweb_app_stays_under_budget() {
    let Some(bin) = perf_bin() else {
        return;
    };
    let app = workspace_root()
        .join("projects")
        .join("hrweb")
        .join("src")
        .join("app.clsk");
    let samples = command_samples(5, || {
        let mut command = Command::new(&bin);
        command.arg("fmt").arg(&app);
        command
    });
    assert_under("closkell fmt hrweb", &samples, FMT_HRWEB_BUDGET);
}

#[test]
fn perf_vite_plugin_warm_hrweb_build_stays_under_budget() {
    let Some(bin) = perf_bin() else {
        return;
    };
    let root = workspace_root();
    let hrweb = root.join("projects").join("hrweb");
    if !vite_bin(&hrweb).is_file() {
        eprintln!("skipping Vite perf test because hrweb npm dependencies are not installed");
        return;
    }

    let mut prebuild = Command::new(vite_bin(&hrweb));
    prebuild
        .arg("build")
        .arg("--logLevel")
        .arg("error")
        .current_dir(&hrweb)
        .env("CLOSKELL_BIN", &bin);
    run_success(&mut prebuild, "prebuild warm Vite input");

    let samples = command_samples(5, || {
        let mut command = Command::new(vite_bin(&hrweb));
        command
            .arg("build")
            .arg("--logLevel")
            .arg("error")
            .current_dir(&hrweb)
            .env("CLOSKELL_BIN", &bin);
        command
    });
    assert_under(
        "Vite plugin warm hrweb build",
        &samples,
        VITE_WARM_BUILD_BUDGET,
    );
}

fn command_samples(mut count: usize, mut build_command: impl FnMut() -> Command) -> Vec<Duration> {
    count = count.max(1);
    (0..count)
        .map(|_| {
            let mut command = build_command();
            let started = Instant::now();
            run_success(&mut command, "perf command");
            started.elapsed()
        })
        .collect()
}

fn assert_under(label: &str, samples: &[Duration], budget: Duration) {
    let median = median_duration(samples);
    assert!(
        median <= budget,
        "{} median {:?} exceeded {:?}; samples: {:?}",
        label,
        median,
        budget,
        samples
    );
}

fn median_duration(samples: &[Duration]) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

fn run_success(command: &mut Command, label: &str) {
    let output = command.output().unwrap_or_else(|err| {
        panic!("failed to run {}: {}", label, err);
    });
    assert!(
        output.status.success(),
        "{} failed\nstdout:\n{}\nstderr:\n{}",
        label,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn perf_bin() -> Option<PathBuf> {
    if env::var("CLOSKELL_PERF_TESTS").ok().as_deref() != Some("1") {
        eprintln!("skipping perf regression test; set CLOSKELL_PERF_TESTS=1 to run");
        return None;
    }
    if let Some(path) = env::var_os("CLOSKELL_PERF_BIN").map(PathBuf::from) {
        return Some(path);
    }

    let exe = if cfg!(windows) {
        "closkell.exe"
    } else {
        "closkell"
    };
    let release_bin = workspace_root().join("target").join("release").join(exe);
    if release_bin.is_file() {
        return Some(release_bin);
    }

    eprintln!(
        "skipping perf regression test because {} is missing; run `cargo build -p cli --release`",
        release_bin.display()
    );
    None
}

fn vite_bin(hrweb: &Path) -> PathBuf {
    let name = if cfg!(windows) { "vite.cmd" } else { "vite" };
    hrweb.join("node_modules").join(".bin").join(name)
}

fn npm_bin() -> Option<PathBuf> {
    let name = if cfg!(windows) { "npm.cmd" } else { "npm" };
    Command::new(name)
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| PathBuf::from(name))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}
