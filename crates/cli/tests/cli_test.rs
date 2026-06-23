use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[test]
fn cli_test_runs_hrweb_pure_suite() {
    if !node_available() {
        eprintln!("skipping closkell test integration test because node is unavailable");
        return;
    }

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(
            workspace_root()
                .join("fixtures")
                .join("hrweb")
                .join("hrweb_test_suite.clsk"),
        )
        .output()
        .expect("closkell test should run");

    assert!(
        run.status.success(),
        "closkell test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - formats short workout duration"),
        "test runner did not report named tests:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 7 - binds whole values with as patterns"),
        "test runner did not report the as-pattern suite case:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 9 - matches option constructors"),
        "test runner did not report the option-pattern suite case:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 10 - matches fixed list patterns"),
        "test runner did not report the list-pattern suite case:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 11 - matches cons list patterns"),
        "test runner did not report the cons-pattern suite case:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 12 - destructures let binding patterns"),
        "test runner did not report the let-destructuring suite case:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 13 - destructures fn parameter patterns"),
        "test runner did not report the fn-parameter destructuring suite case:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 14 - destructures defn parameter patterns"),
        "test runner did not report the defn-parameter destructuring suite case:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 14 tests"),
        "test runner did not report a passing summary:\n{}",
        stdout
    );
}

#[test]
fn cli_test_runs_explicit_gensym_macro_suite() {
    if !node_available() {
        eprintln!("skipping closkell gensym macro integration test because node is unavailable");
        return;
    }

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(
            workspace_root()
                .join("fixtures")
                .join("hrweb")
                .join("hrweb_gensym_macro.clsk"),
        )
        .output()
        .expect("closkell test should run");

    assert!(
        run.status.success(),
        "closkell test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - explicit gensym macro reuses fresh binding"),
        "test runner did not report the gensym macro test:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 2 - with-gensyms macro binds multiple fresh symbols"),
        "test runner did not report the with-gensyms macro test:\n{}",
        stdout
    );
    assert!(
        stdout.contains("ok 2 tests"),
        "test runner did not report a passing summary:\n{}",
        stdout
    );
}

#[test]
fn cli_test_runs_list_ops_suite() {
    if !node_available() {
        eprintln!("skipping closkell list ops integration test because node is unavailable");
        return;
    }

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(
            workspace_root()
                .join("fixtures")
                .join("hrweb")
                .join("hrweb_list_ops.clsk"),
        )
        .output()
        .expect("closkell test should run");

    assert!(
        run.status.success(),
        "closkell list ops test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - constructs persistent lists") && stdout.contains("ok 3 tests"),
        "test runner did not report the list ops suite:\n{}",
        stdout
    );
}

#[test]
fn cli_test_reports_json_for_passing_module_tests() {
    if !node_available() {
        eprintln!("skipping closkell test json integration test because node is unavailable");
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-json-pass");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("passing.clsk");
    fs::write(
        &source,
        "(def tests\n  [{:name \"adds numbers\" :actual (+ 1 1) :expected 2}\n   {:name \"multiplies numbers\" :actual (* 2 3) :expected 6}])\n",
    )
    .expect("passing test module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .arg("--json")
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell test --json failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("\"ok\":true")
            && stdout.contains("\"count\":2")
            && stdout.contains("\"passed\":2")
            && stdout.contains("\"failed\":0")
            && stdout.contains("\"name\":\"adds numbers\""),
        "json test output did not report passing tests:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("ok 1 -"),
        "json mode should not include text runner lines:\n{}",
        stdout
    );
}

#[test]
fn cli_test_runs_closkell_test_api_groups() {
    if !node_available() {
        eprintln!("skipping closkell/test API integration test because node is unavailable");
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-api");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app_test.clsk");
    fs::write(
        &source,
        "(import \"closkell/test\" [describe test expect= expect-not= expect-ok expect-err expect-some expect-match expect-throws])\n\
         \n\
         (describe \"math\"\n\
           (test \"runs grouped assertions\"\n\
             (expect= (+ 1 1) 2)\n\
             (expect-not= (+ 1 1) 3)\n\
             (expect-ok true)\n\
             (expect-err (err \"bad\"))\n\
             (expect-some (find [1 2 3] (fn [value] (= value 2))))\n\
             (expect-match {:kind :loaded :value 42 :meta {:source \"cache\"}}\n\
                           {:kind :loaded :meta {:source \"cache\"}})\n\
             (expect-throws (fn [] (fail \"boom\")) \"boom\")))\n",
    )
    .expect("test API module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell test API module failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - math / runs grouped assertions") && stdout.contains("ok 1 tests"),
        "test API output did not report the grouped test:\n{}",
        stdout
    );
}

#[test]
fn cli_test_reports_json_for_closkell_test_api_groups() {
    if !node_available() {
        eprintln!("skipping closkell/test API json integration test because node is unavailable");
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-api-json");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app_test.clsk");
    fs::write(
        &source,
        "(import \"closkell/test\" [describe test expect=])\n\
         \n\
         (def tests\n\
           [(describe \"numbers\"\n\
              (test \"adds\"\n\
                (expect= (+ 2 3) 5)))])\n",
    )
    .expect("test API module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .arg("--json")
        .output()
        .expect("closkell test --json should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell test API json module failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("\"count\":1")
            && stdout.contains("\"passed\":1")
            && stdout.contains("\"name\":\"numbers / adds\""),
        "json test API output did not report the grouped test:\n{}",
        stdout
    );
}

#[test]
fn cli_test_runs_closkell_component_harness() {
    if !node_available() {
        eprintln!(
            "skipping closkell/test component harness integration test because node is unavailable"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-component-harness");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("component_test.clsk");
    fs::write(
        &source,
        "(import \"closkell/test\" [describe test expect= expect-ok render fire text attr class? messages])\n\
         \n\
         (defn button-view [label]\n\
           #html <button data-testid=\"go\" class={{:primary true}} on:click={{:kind :go :label label}}>{label}</button>)\n\
         \n\
         (describe \"button-view\"\n\
           (test \"renders and captures messages\"\n\
             (let [h (render (button-view \"Ready\"))\n\
                   _ (fire.click h \"[data-testid='go']\")]\n\
               (expect= (text h \"[data-testid='go']\") \"Ready\")\n\
               (expect= (attr h \"[data-testid='go']\" \"data-testid\") \"go\")\n\
               (expect-ok (class? h \"[data-testid='go']\" \"primary\"))\n\
               (expect= (messages h) [{:kind :go :label \"Ready\"}]))))\n",
    )
    .expect("component harness module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell component harness test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - button-view / renders and captures messages")
            && stdout.contains("ok 1 tests"),
        "component harness output did not report the grouped test:\n{}",
        stdout
    );
}

#[test]
fn cli_test_runs_closkell_render_to_string() {
    if !node_available() {
        eprintln!(
            "skipping closkell render-to-string integration test because node is unavailable"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-render-to-string");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("ssr_test.clsk");
    fs::write(
        &source,
        "(import \"closkell/test\" [describe test expect-ok])\n\
         \n\
         (defn card-view [state]\n\
           #html <article data-testid=\"card\" class={{:ready state.ready?}} style={{:color state.color}}>\n\
             <h1>{state.title}</h1>\n\
             <button ref=\"primary\" on:click={{:kind :clicked}}>Go</button>\n\
           </article>)\n\
         \n\
         (describe \"ssr\"\n\
           (test \"renders html with hydration metadata\"\n\
             (let [markup (render-to-string card-view {:title \"Pulse & Go\" :ready? true :color \"red\"})]\n\
               (expect-ok (contains? markup \"<h1>Pulse &amp; Go</h1>\"))\n\
               (expect-ok (contains? markup \"class=\\\"ready\\\"\"))\n\
               (expect-ok (contains? markup \"style=\\\"color: red;\\\"\"))\n\
               (expect-ok (contains? markup \"data-closkell-template=\\\"template\"))\n\
               (expect-ok (contains? markup \"data-closkell-slots=\"))\n\
               (expect-ok (not (contains? markup \"on:click\")))\n\
               (expect-ok (not (contains? markup \" ref=\"))))))\n",
    )
    .expect("render-to-string module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell render-to-string test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - ssr / renders html with hydration metadata")
            && stdout.contains("ok 1 tests"),
        "render-to-string output did not report the grouped test:\n{}",
        stdout
    );
}

#[test]
fn cli_test_runs_closkell_decoders() {
    if !node_available() {
        eprintln!("skipping closkell decoder integration test because node is unavailable");
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-decoders");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("decoder_test.clsk");
    fs::write(
        &source,
        "(import \"closkell/test\" [describe test expect= expect-ok])\n\
         \n\
         (def spec-decoder\n\
           (decoder-record {:title decoder-string\n\
                            :tags (decoder-vector decoder-string)\n\
                            :draft (decoder-optional decoder-number)}))\n\
         \n\
         (describe \"decoder\"\n\
           (test \"validates json records\"\n\
             (let [valid (decode spec-decoder (json-parse \"{\\\"title\\\":\\\"Pulse\\\",\\\"tags\\\":[\\\"zone\\\"]}\"))\n\
                   invalid (decode spec-decoder (json-parse \"{\\\"title\\\":5,\\\"tags\\\":[]}\"))\n\
                   value (unwrap-or valid {:title \"\" :tags [] :draft nil})]\n\
               (expect-ok (ok? valid))\n\
               (expect= value.title \"Pulse\")\n\
               (expect= value.tags [\"zone\"])\n\
               (expect= value.draft nil)\n\
               (expect-ok (err? invalid)))))\n",
    )
    .expect("decoder module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell decoder test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - decoder / validates json records") && stdout.contains("ok 1 tests"),
        "decoder output did not report the grouped test:\n{}",
        stdout
    );
}

#[test]
fn cli_test_runs_closkell_app_harness() {
    if !node_available() {
        eprintln!(
            "skipping closkell/test app harness integration test because node is unavailable"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-app-harness");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app_harness_test.clsk");
    fs::write(
        &source,
        "(import \"closkell/test\" [describe test expect= mount-app commands text (subscriptions as active-subscriptions)])\n\
         \n\
         (def init [{:status \"Idle\" :running? false} {:kind :none}])\n\
         \n\
         (defn update [state msg]\n\
           (match msg\n\
             {:kind :start}\n\
             [(assoc state :status \"Starting\" :running? true) {:kind :time/now :onSuccess :started}]\n\
             \n\
             {:kind :started :value value}\n\
             [(assoc state :status (str \"Started \" value)) {:kind :none}]\n\
             \n\
             _ [state {:kind :none}]))\n\
         \n\
         (defn subscriptions [state]\n\
           (if state.running?\n\
             (Sub.timer/every \"clock\" 250 {:kind :tick})\n\
             Sub.none))\n\
         \n\
         (defn view [state]\n\
           #html <section data-testid=\"status\">{state.status}</section>)\n\
         \n\
         (describe \"app harness\"\n\
           (test \"dispatches through update and records effects\"\n\
             (let [app (mount-app {:init init :update update :view view :subscriptions subscriptions}\n\
                                  {:handlers {:time/now (fn [cmd dispatch] {:kind :started :value 42})}\n\
                                   :subscriptionHandlers {:start (fn [sub dispatch] nil)\n\
                                                          :stop (fn [sub dispatch] nil)}})\n\
                   _ (app.dispatch {:kind :start})]\n\
               (expect= (text app \"[data-testid='status']\") \"Started 42\")\n\
               (expect= (commands app) [{:kind :time/now :onSuccess :started}])\n\
               (expect= (active-subscriptions app) [{:kind :sub/timer/every :id \"clock\" :ms 250 :msg {:kind :tick}}]))))\n",
    )
    .expect("app harness module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell app harness test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("ok 1 - app harness / dispatches through update and records effects")
            && stdout.contains("ok 1 tests"),
        "app harness output did not report the grouped test:\n{}",
        stdout
    );
}

#[test]
fn cli_test_reports_failing_module_test() {
    if !node_available() {
        eprintln!("skipping closkell test failure integration test because node is unavailable");
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-fail");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("failing.clsk");
    fs::write(
        &source,
        "(def tests\n  [{:name \"detects mismatch\"\n    :actual (+ 1 1)\n    :expected 3}])\n",
    )
    .expect("failing test module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell test unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let output = combined_output(&run.stdout, &run.stderr);
    assert!(
        output.contains("not ok 1 - detects mismatch"),
        "test runner did not report the failing test:\n{}",
        output
    );
    assert!(
        output.contains("expected 3") && output.contains("actual   2"),
        "test runner did not show expected and actual values:\n{}",
        output
    );
}

#[test]
fn cli_test_reports_json_for_failing_module_tests() {
    if !node_available() {
        eprintln!(
            "skipping closkell test json failure integration test because node is unavailable"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-json-fail");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("failing.clsk");
    fs::write(
        &source,
        "(def tests\n  [{:name \"detects mismatch\"\n    :actual (+ 1 1)\n    :expected 3}])\n",
    )
    .expect("failing test module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .arg("--json")
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell test --json unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("\"ok\":false")
            && stdout.contains("\"failed\":1")
            && stdout.contains("\"name\":\"detects mismatch\"")
            && stdout.contains("\"expected\":\"3\"")
            && stdout.contains("\"actual\":\"2\""),
        "json test output did not report failing details:\n{}",
        stdout
    );
}

#[test]
fn cli_test_requires_tests_export() {
    if !node_available() {
        eprintln!(
            "skipping closkell test missing-export integration test because node is unavailable"
        );
        return;
    }

    let temp_dir = temp_dir("closkell-cli-test-missing-export");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("missing.clsk");
    fs::write(&source, "(def answer 42)\n").expect("test module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("test")
        .arg(&source)
        .output()
        .expect("closkell test should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell test unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let output = combined_output(&run.stdout, &run.stderr);
    assert!(
        output.contains("expected module to export `tests`"),
        "test runner did not explain the missing tests export:\n{}",
        output
    );
}

#[test]
fn cli_check_rejects_browser_api_access_inside_html() {
    let temp_dir = temp_dir("closkell-cli-check-html-browser-api");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("bad-view.clsk");
    fs::write(
        &source,
        "(defn view [state]\n  #html <button on:click={(fetch \"/api/workouts\")}>{document.title}</button>)\n",
    )
    .expect("bad html module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("closkell check should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell check unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let output = combined_output(&run.stdout, &run.stderr);
    assert!(
        output.contains("fetch") && output.contains("document.title"),
        "check did not report browser API access in html expressions:\n{}",
        output
    );
}

#[test]
fn cli_check_json_reports_machine_readable_diagnostics() {
    let temp_dir = temp_dir("closkell-cli-check-json");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("bad-view.clsk");
    fs::write(
        &source,
        "(defn view [state]\n  #html <button on:click={(fetch \"/api/workouts\")}>{state.label}</button>)\n",
    )
    .expect("bad html module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--json")
        .arg(&source)
        .output()
        .expect("closkell check --json should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell check --json unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.trim_start().starts_with("{\"diagnostics\":["),
        "check --json did not emit a diagnostics object:\n{}",
        stdout
    );
    assert!(
        stdout.contains("bad-view.clsk")
            && stdout.contains("\"severity\":\"error\"")
            && stdout.contains("\"code\":\"clsk/effect-browser-api\"")
            && stdout.contains("browser API")
            && stdout.contains("fetch")
            && stdout.contains("\"fixes\":[]")
            && stdout.contains("\"range\":{\"start\":{\"line\":2"),
        "check --json did not include the expected diagnostic fields:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("Error at"),
        "check --json should not mix human diagnostics into stdout:\n{}",
        stdout
    );
}

#[test]
fn cli_check_json_reports_expected_actual_for_type_mismatch() {
    let temp_dir = temp_dir("closkell-cli-check-json-type-mismatch");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("bad-types.clsk");
    fs::write(&source, "(ann bad Number)\n(def bad \"two\")\n")
        .expect("bad type module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--json")
        .arg(&source)
        .output()
        .expect("closkell check --json should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell check --json unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("\"code\":\"clsk/type-mismatch\"")
            && stdout.contains("\"expected\":\"Number\"")
            && stdout.contains("\"actual\":\"String\"")
            && stdout.contains("\"fixes\":[]"),
        "check --json did not include type mismatch metadata:\n{}",
        stdout
    );
}

#[test]
fn cli_check_json_uses_persistent_cache_for_unchanged_module_graph() {
    let temp_dir = temp_dir("closkell-cli-check-cache-hit");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app.clsk");
    fs::write(&source, "(def answer 42)\n").expect("source module should be written");

    let first = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--json")
        .arg("--cache-debug")
        .arg(&source)
        .output()
        .expect("closkell check --json should run");
    assert!(
        first.status.success(),
        "first cached check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        String::from_utf8_lossy(&first.stderr).contains("closkell cache miss")
            && String::from_utf8_lossy(&first.stderr).contains("closkell cache write"),
        "first check did not report miss/write:\n{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--json")
        .arg("--cache-debug")
        .arg(&source)
        .output()
        .expect("closkell check --json should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        second.status.success(),
        "second cached check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("closkell cache hit"),
        "second check did not report cache hit:\n{}",
        String::from_utf8_lossy(&second.stderr)
    );
}

#[test]
fn cli_check_json_invalidates_cache_when_import_changes() {
    let temp_dir = temp_dir("closkell-cli-check-cache-invalidation");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let api = temp_dir.join("api.clsk");
    let app = temp_dir.join("app.clsk");
    fs::write(&api, "(ann answer Number)\n(def answer 1)\n").expect("api module should be written");
    fs::write(
        &app,
        "(import \"./api.clsk\" [answer])\n(def total (+ answer 1))\n",
    )
    .expect("app module should be written");

    let first = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--json")
        .arg("--cache-debug")
        .arg(&app)
        .output()
        .expect("closkell check --json should run");
    assert!(
        first.status.success(),
        "initial cached check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );

    fs::write(&api, "(ann answer Number)\n(def answer \"one\")\n")
        .expect("api module should be changed");

    let second = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--json")
        .arg("--cache-debug")
        .arg(&app)
        .output()
        .expect("closkell check --json should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !second.status.success(),
        "changed dependency unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    let stderr = String::from_utf8_lossy(&second.stderr);
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stderr.contains("closkell cache miss") && !stderr.contains("closkell cache hit"),
        "dependency change did not invalidate cache:\n{}",
        stderr
    );
    assert!(
        stdout.contains("\"code\":\"clsk/type-mismatch\"")
            && stdout.contains("\"file\":")
            && stdout.contains("api.clsk"),
        "changed dependency diagnostics were not recomputed:\n{}",
        stdout
    );
}

#[test]
fn cli_check_stdin_uses_buffer_without_creating_side_files() {
    let temp_dir = temp_dir("closkell-cli-check-stdin");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app.clsk");
    fs::write(&source, "(def answer 42)\n").expect("source module should be written");

    let mut child = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--json")
        .arg("--stdin")
        .arg(&source)
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("closkell check --stdin should start");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(b"(def answer (fetch \"/api/workouts\"))\n")
        .expect("stdin source should be written");

    let run = child
        .wait_with_output()
        .expect("closkell check --stdin should run");
    let files = fs::read_dir(&temp_dir)
        .expect("temp dir should be readable")
        .map(|entry| {
            entry
                .expect("temp dir entry should be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell check --stdin unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("browser API") && stdout.contains("fetch"),
        "check --stdin did not use the piped buffer:\n{}",
        stdout
    );
    assert_eq!(
        files,
        vec!["app.clsk".to_string()],
        "check --stdin should not create temporary side files"
    );
}

#[test]
fn cli_check_is_quiet_on_success_by_default() {
    let temp_dir = temp_dir("closkell-cli-check-quiet");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app.clsk");
    fs::write(
        &source,
        "(def answer 42)\n(defn view [state] #html <p>{state.label}</p>)\n",
    )
    .expect("source module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg(&source)
        .output()
        .expect("closkell check should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).trim().is_empty(),
        "successful check should not dump inferred forms by default:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );
}

#[test]
fn cli_fmt_rejects_parse_errors_without_partial_output() {
    let temp_dir = temp_dir("closkell-cli-fmt-parse-error");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("broken.clsk");
    fs::write(&source, "#html <div>{name}\n").expect("broken source should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("fmt")
        .arg(&source)
        .output()
        .expect("closkell fmt should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell fmt unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).trim().is_empty(),
        "fmt should not emit a partial pretty-print for parse errors:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("fmt failed during parsing") && stderr.contains("missing closing tag"),
        "fmt did not explain the parse failure:\n{}",
        stderr
    );
}

#[test]
fn cli_fmt_stdin_formats_buffer_without_source_file() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("fmt")
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("closkell fmt --stdin should start");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(b"(def answer (+ 40 2))\n")
        .expect("stdin source should be written");

    let run = child
        .wait_with_output()
        .expect("closkell fmt --stdin should run");

    assert!(
        run.status.success(),
        "closkell fmt --stdin failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "(def answer (+ 40 2))"
    );
}

#[test]
fn cli_check_types_flag_prints_inferred_forms_and_templates() {
    let temp_dir = temp_dir("closkell-cli-check-types");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app.clsk");
    fs::write(
        &source,
        "(def answer 42)\n(defn view [state] #html <p>{state.label}</p>)\n",
    )
    .expect("source module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg("--types")
        .arg(&source)
        .output()
        .expect("closkell check --types should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell check --types failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("(def answer 42) : Number")
            && stdout.contains("(defn view [state]")
            && stdout.contains("Html")
            && stdout.contains("templates: 1"),
        "check --types did not print inferred forms and template count:\n{}",
        stdout
    );
}

#[test]
fn cli_build_writes_source_maps_for_entry_and_imports() {
    let temp_dir = temp_dir("closkell-cli-build-sourcemap");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let dep = temp_dir.join("dep.clsk");
    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("dist").join("app.mjs");
    let dep_output = temp_dir.join("dist").join("dep.mjs");
    let app_map = temp_dir.join("dist").join("app.mjs.map");
    let dep_map = temp_dir.join("dist").join("dep.mjs.map");

    fs::write(&dep, "(def value 41)\n").expect("dependency should be written");
    fs::write(
        &app,
        "(import \"./dep.clsk\" [value])\n(def answer (+ value 1))\n",
    )
    .expect("app should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--sourcemap")
        .output()
        .expect("closkell build should run");

    assert!(
        run.status.success(),
        "closkell build --sourcemap failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let app_js = fs::read_to_string(&output).expect("entry JS should be readable");
    let dep_js = fs::read_to_string(&dep_output).expect("imported JS should be readable");
    assert!(app_js.contains("//# sourceMappingURL=app.mjs.map"));
    assert!(dep_js.contains("//# sourceMappingURL=dep.mjs.map"));

    let app_map = fs::read_to_string(&app_map).expect("entry source map should be readable");
    let dep_map = fs::read_to_string(&dep_map).expect("imported source map should be readable");
    assert!(app_map.contains("\"version\": 3"));
    assert!(app_map.contains("\"file\": \"app.mjs\""));
    assert!(app_map.contains("app.clsk"));
    assert!(app_map.contains("(def answer (+ value 1))"));
    assert!(app_map.contains("\"mappings\":"));
    assert!(dep_map.contains("\"file\": \"dep.mjs\""));
    assert!(dep_map.contains("dep.clsk"));
    assert!(dep_map.contains("(def value 41)"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn cli_build_json_reports_written_artifacts() {
    let temp_dir = temp_dir("closkell-cli-build-json-artifacts");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let dep = temp_dir.join("dep.clsk");
    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("dist").join("app.mjs");
    fs::write(&dep, "(ann value Number)\n(def value 41)\n").expect("dependency should be written");
    fs::write(
        &app,
        "(import \"./dep.clsk\" [value])\n(ann answer Number)\n(def answer (+ value 1))\n",
    )
    .expect("app should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--sourcemap")
        .arg("--json")
        .output()
        .expect("closkell build --json should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        run.status.success(),
        "closkell build --json failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.trim_start().starts_with("{\"ok\":true")
            && stdout.contains("\"artifacts\":[")
            && stdout.contains("\"kind\":\"import\"")
            && stdout.contains("dep.clsk")
            && stdout.contains("dep.mjs")
            && stdout.contains("dep.mjs.map")
            && stdout.contains("\"kind\":\"entry\"")
            && stdout.contains("app.clsk")
            && stdout.contains("app.mjs")
            && stdout.contains("app.mjs.map")
            && stdout.contains("\"diagnostics\":[]"),
        "build --json did not report written artifacts:\n{}",
        stdout
    );
}

#[test]
fn cli_build_json_reports_check_diagnostics() {
    let temp_dir = temp_dir("closkell-cli-build-json-diagnostics");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("bad.clsk");
    let output = temp_dir.join("dist").join("bad.mjs");
    fs::write(&source, "(ann bad Number)\n(def bad \"nope\")\n")
        .expect("bad module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&source)
        .arg("--out")
        .arg(&output)
        .arg("--json")
        .output()
        .expect("closkell build --json should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell build --json unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.trim_start().starts_with("{\"ok\":false")
            && stdout.contains("\"diagnostics\":[")
            && stdout.contains("\"code\":\"clsk/type-mismatch\"")
            && stdout.contains("\"expected\":\"Number\"")
            && stdout.contains("\"actual\":\"String\"")
            && stdout.contains("\"artifacts\":[]")
            && !stdout.contains("Error at"),
        "build --json did not report structured diagnostics:\n{}",
        stdout
    );
}

#[test]
fn cli_build_expands_imported_macros_and_erases_macro_imports() {
    if !node_available() {
        eprintln!("skipping imported macro build integration test because node is unavailable");
        return;
    }

    let temp_dir = temp_dir("closkell-cli-build-imported-macros");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let macros = temp_dir.join("macros.clsk");
    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("dist").join("app.mjs");
    let macro_output = temp_dir.join("dist").join("macros.mjs");

    fs::write(
        &macros,
        "(defmacro cmd-none [] `{:kind :none})\n\
         (defmacro with-reading-temp [value]\n  (let [tmp (gensym \"reading\")]\n    `(let [~tmp ~value]\n       {:bpm ~tmp\n        :label (str \"HR \" ~tmp)})))\n",
    )
    .expect("macro module should be written");
    fs::write(
        &app,
        "(import \"./macros.clsk\" [cmd-none with-reading-temp])\n\
         (def reading-summary (with-reading-temp 142))\n\
         (defn update [state msg]\n  [state (cmd-none)])\n",
    )
    .expect("app module should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");

    assert!(
        run.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let app_js = fs::read_to_string(&output).expect("app JS should be readable");
    assert!(
        !app_js.contains("macros.mjs"),
        "macro-only import survived in emitted JS:\n{}",
        app_js
    );
    assert!(
        !macro_output.exists(),
        "macro-only module output should not be written"
    );
    assert!(app_js.contains("reading__gensym0"));

    let script = format!(
        r#"
const mod = await import(fileUrl({module_path}));
if (mod.reading_summary.bpm !== 142) throw new Error("imported gensym macro did not expand reading bpm");
if (mod.reading_summary.label !== "HR 142") throw new Error("imported gensym macro did not reuse the fresh binding");
const [, command] = mod.update({{}}, Symbol.for("noop"));
if (command.kind !== Symbol.for("none")) throw new Error("imported cmd-none macro did not expand");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        module_path = json_string_for_test(&output.display().to_string())
    );
    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "generated imported-macro module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn cli_build_erases_type_only_imports_across_modules() {
    let temp_dir = temp_dir("closkell-cli-build-type-only-imports");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let model = temp_dir.join("model.clsk");
    let helper = temp_dir.join("helper.clsk");
    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("dist").join("app.mjs");
    let helper_output = temp_dir.join("dist").join("helper.mjs");
    let model_output = temp_dir.join("dist").join("model.mjs");
    let model_map = temp_dir.join("dist").join("model.mjs.map");

    fs::write(
        &model,
        "(type Reading\n  {:bpm Number\n   :label String})\n\
         \n\
         (type Readout\n  {:latest Reading\n   :summary String})\n",
    )
    .expect("model module should be written");
    fs::write(
        &helper,
        "(import \"./model.clsk\" [Reading])\n\
         \n\
         (ann format-reading (Fn [Reading] String))\n\
         (defn format-reading [reading]\n  (str reading.label \" \" reading.bpm))\n",
    )
    .expect("helper module should be written");
    fs::write(
        &app,
        "(import \"./model.clsk\" [Reading Readout])\n\
         (import \"./helper.clsk\" [format-reading])\n\
         \n\
         (ann sample-reading Reading)\n\
         (def sample-reading {:bpm 142 :label \"Tempo\"})\n\
         \n\
         (ann sample-readout Readout)\n\
         (def sample-readout\n  {:latest sample-reading\n   :summary (format-reading sample-reading)})\n",
    )
    .expect("app module should be written");
    fs::create_dir_all(temp_dir.join("dist")).expect("dist dir should be created");
    fs::write(&model_output, "stale model js").expect("stale model output should be written");
    fs::write(&model_map, "stale model map").expect("stale model map should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--sourcemap")
        .output()
        .expect("closkell build should run");

    assert!(
        run.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let app_js = fs::read_to_string(&output).expect("entry JS should be readable");
    let helper_js = fs::read_to_string(&helper_output).expect("helper JS should be readable");
    assert!(
        app_js.contains("import { format_reading } from \"./helper.mjs\";"),
        "entry JS did not keep the runtime helper import:\n{}",
        app_js
    );
    assert!(
        !app_js.contains("model.mjs"),
        "entry JS kept a type-only model import:\n{}",
        app_js
    );
    assert!(
        !helper_js.contains("model.mjs"),
        "helper JS kept a type-only model import:\n{}",
        helper_js
    );
    assert!(
        !model_output.exists(),
        "type-only model output was not removed"
    );
    assert!(
        !model_map.exists(),
        "type-only model source map was not removed"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn cli_build_app_writes_vite_entry_bootstrap() {
    let temp_dir = temp_dir("closkell-cli-build-app");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let app = temp_dir.join("app.clsk");
    let package_json = temp_dir.join("package.json");
    let output = temp_dir.join("dist").join("main.mjs");
    let app_map = temp_dir.join("dist").join("main.mjs.map");
    let runtime_package = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("package.json");
    let runtime_entry = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");

    fs::write(&package_json, "{\"type\":\"module\"}\n").expect("package.json should be written");
    fs::write(
        &app,
        "(def init {:count 0})\n\
         (defn update [state msg] [state {:kind :none}])\n\
         (def card-class \"rounded\")\n\
         (defn view [state]\n  #html <button class={card-class} on:click={{:kind :tick}}>{state.count}</button>)\n",
    )
    .expect("app source should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--app")
        .arg("--root")
        .arg("app")
        .arg("--css")
        .arg("./styles.css")
        .arg("--vendor-runtime")
        .arg("--sourcemap")
        .output()
        .expect("closkell build --app should run");

    assert!(
        run.status.success(),
        "closkell build --app failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let app_js = fs::read_to_string(&output).expect("app entry JS should be readable");
    assert!(
        app_js.starts_with(
            "import { createBrowserBootInput as __closkellCreateBrowserBootInput, createCommandHandlers as __closkellCreateCommandHandlers, createSubscriptionHandlers as __closkellCreateSubscriptionHandlers, createDevtoolsOverlay as __closkellCreateDevtoolsOverlay, startApp as __closkellStartApp } from \"@closkell/runtime\";"
        ),
        "app bootstrap did not import the runtime first:\n{}",
        app_js
    );
    assert!(app_js.contains("import \"./styles.css\";"));
    assert!(app_js.contains("document.getElementById(\"app\")"));
    assert!(app_js.contains("export const __closkellApp = __closkellStartApp({"));
    assert!(app_js.contains("const __closkellHandlers = __closkellCreateCommandHandlers();"));
    assert!(app_js.contains("boot: __closkellCreateBrowserBootInput()"));
    assert!(app_js.contains("handlers: __closkellHandlers"));
    assert!(app_js.contains(
        "subscriptions: typeof subscriptions === \"function\" ? subscriptions : undefined"
    ));
    assert!(app_js.contains(
        "subscriptionHandlers: __closkellCreateSubscriptionHandlers({ commandHandlers: __closkellHandlers })"
    ));
    assert!(
        app_js.contains("__closkellCreateDevtoolsOverlay(globalThis.__closkellDevtoolsOverlay)")
    );
    assert!(app_js.contains("globalThis.__closkellDevtoolsOverlayInstance = __closkellDevtools"));
    assert!(app_js.contains("devtools: __closkellDevtools"));
    assert!(app_js.contains("//# sourceMappingURL=main.mjs.map"));
    let card_class_index = app_js
        .find("export const card_class")
        .expect("class constant should be emitted");
    let app_start_index = app_js
        .find("export const __closkellApp")
        .expect("app start should be emitted");
    assert!(
        card_class_index < app_start_index,
        "app startup should run after generated constants:\n{}",
        app_js
    );

    let app_map = fs::read_to_string(&app_map).expect("app source map should be readable");
    assert!(app_map.contains("\"file\": \"main.mjs\""));
    assert!(app_map.contains("app.clsk"));
    assert!(app_map.contains("(defn view [state]"));
    assert!(
        runtime_package.is_file(),
        "runtime package was not vendored"
    );
    let runtime_package =
        fs::read_to_string(&runtime_package).expect("runtime package should be readable");
    assert!(runtime_package.contains("\"name\": \"@closkell/runtime\""));
    assert!(runtime_entry.is_file(), "runtime entry was not vendored");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn cli_build_app_requires_app_exports() {
    let temp_dir = temp_dir("closkell-cli-build-app-missing-exports");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("dist").join("main.mjs");
    fs::write(&app, "(def init {:count 0})\n").expect("app source should be written");

    let run = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .arg("--app")
        .output()
        .expect("closkell build --app should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !run.status.success(),
        "closkell build --app unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let output = combined_output(&run.stdout, &run.stderr);
    assert!(
        output.contains("build --app expects") && output.contains("missing update, view"),
        "missing app exports were not reported clearly:\n{}",
        output
    );
}

#[test]
fn cli_inspect_reports_api_type_declarations() {
    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(
            workspace_root()
                .join("fixtures")
                .join("hrweb")
                .join("hrweb_api_types.clsk"),
        )
        .output()
        .expect("closkell inspect should run");

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"types\":"),
        "inspect did not include type declarations:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"annotations\":"),
        "inspect did not include type annotations:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"WorkoutMsg\""),
        "inspect did not report WorkoutMsg:\n{}",
        stdout
    );
    assert!(
        stdout.contains("(Union {:kind :start} {:kind :pause}"),
        "inspect did not render the tagged union schema:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"UpdateResult\"")
            && stdout.contains("[WorkoutState (Cmd WorkoutMsg)]"),
        "inspect did not report the update result type:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"reading-label\"")
            && stdout.contains("(Fn [HeartReading] String)"),
        "inspect did not report the reading-label annotation:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"api-type-count\"") && stdout.contains("\"schema\":\"Number\""),
        "inspect did not report the api-type-count annotation:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"api-update\"")
            && stdout.contains("(Fn [WorkoutState WorkoutMsg] UpdateResult)"),
        "inspect did not report the api-update Cmd annotation:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_reports_parametric_type_declaration_params() {
    let temp_dir = temp_dir("closkell-cli-inspect-parametric-types");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("api.clsk");
    fs::write(
        &source,
        "(type RemoteData a (Union {:kind :idle} {:kind :ready :value a}))\n\
         (ann loaded (RemoteData String))\n\
         (def loaded {:kind :ready :value \"ok\"})",
    )
    .expect("api module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&source)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"name\":\"RemoteData\"")
            && stdout.contains("\"params\":[\"a\"]")
            && stdout.contains("\"schema\":\"(Union {:kind :idle} {:kind :ready :value a})\""),
        "inspect did not include parametric type params:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_reports_module_boundaries_and_review_facts() {
    let temp_dir = temp_dir("closkell-cli-inspect-review-facts");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let api = temp_dir.join("api.clsk");
    let app = temp_dir.join("app.clsk");
    fs::write(
        &api,
        "(type Payload {:value Number})\n\
         (ann answer Number)\n\
         (def answer 41)\n",
    )
    .expect("api module should be written");
    fs::write(
        &app,
        "(import \"./api.clsk\" [(answer as importedAnswer) (Payload as ImportedPayload)])\n\
         (import \"closkell/test\" [describe test expect=])\n\
         (import \"marked\" [(parse as markedParse)])\n\
         (foreign pure markedParse (Fn [String {:async Bool}] String))\n\
         (type Local {:payload ImportedPayload :html TrustedHtml})\n\
         (ann html TrustedHtml)\n\
         (def html (unsafe-cast TrustedHtml (markedParse \"**ok**\" {:async false})))\n\
         (ann payload ImportedPayload)\n\
         (def payload {:value (+ importedAnswer 1)})\n\
         (ann local Local)\n\
         (def local {:payload payload :html html})\n\
         (def tests [(describe \"inspect facts\"\n\
                      (test \"sees aliases\"\n\
                        (expect= local.payload.value 42)))])\n",
    )
    .expect("app module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"imports\":")
            && stdout.contains("\"path\":\"./api.clsk\"")
            && stdout.contains("\"imported\":\"answer\",\"local\":\"importedAnswer\"")
            && stdout.contains("\"imported\":\"Payload\",\"local\":\"ImportedPayload\"")
            && stdout.contains("\"path\":\"marked\"")
            && stdout.contains("\"imported\":\"parse\",\"local\":\"markedParse\""),
        "inspect did not include imported/local module boundary facts:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"publicSignatures\":")
            && stdout.contains("\"name\":\"html\",\"schema\":\"TrustedHtml\",\"annotated\":true")
            && stdout.contains("\"name\":\"local\",\"schema\":\"{:html TrustedHtml :payload {:value Number}}\",\"annotated\":true"),
        "inspect did not include public signatures:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"jsInterop\":")
            && stdout.contains("\"mode\":\"pure\",\"name\":\"markedParse\""),
        "inspect did not include foreign interop boundaries:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"unsafeCasts\":")
            && stdout.contains("\"target\":\"TrustedHtml\"")
            && stdout.contains("markedParse"),
        "inspect did not include unsafe-cast review facts:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"tests\":")
            && stdout.contains("\"name\":\"sees aliases\"")
            && stdout.contains("\"path\":[\"inspect facts\",\"sees aliases\"]"),
        "inspect did not include test case facts:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_reports_changed_path_summaries_for_update_forms() {
    let temp_dir = temp_dir("closkell-cli-inspect-changed-paths");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let app = temp_dir.join("app.clsk");
    fs::write(
        &app,
        "(type ChildState {:count Number})\n\
         (type ChildMsg (Union {:kind :child-inc}))\n\
         (type AppState {:count Number :status String :settings (Map String Bool) :profile {:name String} :log ChildState :route String})\n\
         (type AppMsg (Union {:kind :inc} {:kind :reset} {:kind :settings} {:kind :rename :name String} {:kind :log :msg ChildMsg}))\n\
         (type ChildResult [ChildState (Cmd ChildMsg)])\n\
         (type AppResult [AppState (Cmd AppMsg)])\n\
         (ann child-update (Fn [ChildState ChildMsg] ChildResult))\n\
         (defn child-update [state msg]\n  [(assoc state :count (+ state.count 1)) {:kind :none}])\n\
         (defn clear-status [state]\n  (dissoc state :status))\n\
         (ann update (Fn [AppState AppMsg] AppResult))\n\
         (defn update [state msg]\n  (match msg\n    {:kind :inc}\n      [(assoc state :count (+ state.count 1) :status \"Counting\") {:kind :none}]\n    {:kind :reset}\n      [(merge state {:count 0 :status \"Ready\"}) {:kind :none}]\n    {:kind :settings}\n      [(assoc state :settings (map-assoc state.settings \"compact\" true)) {:kind :none}]\n    {:kind :rename :name name}\n      [(update-in state [:profile :name] (fn [current] name)) {:kind :none}]\n    {:kind :log :msg child-msg}\n      (scope-update state :log child-msg child-update :log)))\n",
    )
    .expect("app module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"changedPathSummaries\":")
            && stdout
                .contains("\"source\":\"update\",\"operation\":\"assoc\",\"path\":\"state.count\"")
            && stdout.contains(
                "\"source\":\"update\",\"operation\":\"assoc\",\"path\":\"state.status\""
            )
            && stdout
                .contains("\"source\":\"update\",\"operation\":\"merge\",\"path\":\"state.count\"")
            && stdout.contains(
                "\"source\":\"update\",\"operation\":\"merge\",\"path\":\"state.status\""
            )
            && stdout.contains(
                "\"source\":\"update\",\"operation\":\"map-assoc\",\"path\":\"state.settings\""
            )
            && stdout.contains(
                "\"source\":\"update\",\"operation\":\"update-in\",\"path\":\"state.profile.name\""
            )
            && stdout.contains(
                "\"source\":\"update\",\"operation\":\"scope-update\",\"path\":\"state.log\""
            )
            && stdout.contains(
                "\"source\":\"clear-status\",\"operation\":\"dissoc\",\"path\":\"state.status\""
            ),
        "inspect did not include compiler-derived changed path summaries:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_uses_persistent_cache_for_unchanged_module_graph() {
    let temp_dir = temp_dir("closkell-cli-inspect-cache-hit");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let source = temp_dir.join("app.clsk");
    fs::write(
        &source,
        "(type Box a {:value a})\n\
         (ann value (Box String))\n\
         (def value {:value \"ok\"})\n",
    )
    .expect("source module should be written");

    let first = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg("--cache-debug")
        .arg(&source)
        .output()
        .expect("closkell inspect should run");
    assert!(
        first.status.success(),
        "first cached inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        first_stderr.contains("closkell cache miss")
            && first_stderr.contains("closkell cache write"),
        "first inspect did not report miss/write:\n{}",
        first_stderr
    );
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    assert!(
        first_stdout.contains("\"name\":\"Box\"")
            && first_stdout.contains("\"params\":[\"a\"]")
            && first_stdout.contains("\"schema\":\"{:value a}\""),
        "inspect did not include cached type details:\n{}",
        first_stdout
    );

    let second = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg("--cache-debug")
        .arg(&source)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        second.status.success(),
        "second cached inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    let second_stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        second_stderr.contains("closkell cache hit"),
        "second inspect did not report cache hit:\n{}",
        second_stderr
    );
}

#[test]
fn cli_inspect_invalidates_cache_when_import_changes() {
    let temp_dir = temp_dir("closkell-cli-inspect-cache-invalidation");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let api = temp_dir.join("api.clsk");
    let app = temp_dir.join("app.clsk");
    fs::write(&api, "(def answer 1)\n").expect("api module should be written");
    fs::write(
        &app,
        "(import \"./api.clsk\" [answer])\n\
         (def local answer)\n",
    )
    .expect("app module should be written");

    let first = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg("--cache-debug")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");
    assert!(
        first.status.success(),
        "initial cached inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let first_stderr = String::from_utf8_lossy(&first.stderr);
    assert!(
        first_stderr.contains("closkell cache miss")
            && first_stderr.contains("closkell cache write"),
        "initial inspect did not write cache:\n{}",
        first_stderr
    );

    fs::write(&api, "(def answer 2)\n").expect("api module should be changed");

    let second = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg("--cache-debug")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        second.status.success(),
        "changed dependency inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    let second_stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        second_stderr.contains("closkell cache miss")
            && !second_stderr.contains("closkell cache hit"),
        "dependency change did not invalidate inspect cache:\n{}",
        second_stderr
    );
}

#[test]
fn cli_inspect_reports_subscription_schema() {
    let temp_dir = temp_dir("closkell-cli-inspect-subscriptions");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let app = temp_dir.join("app.clsk");
    fs::write(
        &app,
        "(type State {:running? Bool})\n\
         (type Msg (Union {:kind :tick} {:kind :media-changed :id String :media String :matches Bool}))\n\
         (ann subscriptions (Fn [State] (Sub Msg)))\n\
         (defn subscriptions [state]\n  (Sub.batch [(if state.running?\n                   (Sub.timer/every \"clock\" 250 {:kind :tick})\n                   Sub.none)\n              (Sub.media-query \"mobile\" \"(max-width: 700px)\" :media-changed)]))\n",
    )
    .expect("app module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"subscriptionSchema\":")
            && stdout.contains("\"kind\":\"sub/timer/every\"")
            && stdout.contains("\"kind\":\"sub/media-query\""),
        "inspect did not include subscription schema:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_reports_task_perform_command_schema() {
    let temp_dir = temp_dir("closkell-cli-inspect-task-perform");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let app = temp_dir.join("app.clsk");
    fs::write(
        &app,
        "(type Msg (Union {:kind :loaded :value String} {:kind :failed :error String}))\n\
         (ann load-command (Fn [String] (Cmd Msg)))\n\
         (defn load-command [url]\n  (Task.perform (Http.get-text url)\n                (fn [text] {:kind :loaded :value text})\n                (fn [error] {:kind :failed :error error})))\n",
    )
    .expect("app module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"commandLogSchema\":")
            && stdout.contains("\"kind\":\"task/perform\"")
            && stdout.contains("\"fields\":[\"kind\",\"onError\",\"onSuccess\",\"task\"]"),
        "inspect did not include task/perform command schema:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_reports_scoped_view_reads_and_uses() {
    let temp_dir = temp_dir("closkell-cli-inspect-scope-view");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let app = temp_dir.join("app.clsk");
    fs::write(
        &app,
        "(defn child-view [state]\n  #html <button>{state.count}</button>)\n\
         (defn view [state]\n  #html <main>{(scope-view :log child-view state.log)}</main>)\n",
    )
    .expect("app module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"component\":\"view\",\"uses\":[\"child-view\"]"),
        "inspect did not report scoped child component use:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"path\":\"state.log.count\""),
        "inspect did not project scoped child reads through state.log:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_includes_imported_command_helper_schema() {
    let temp_dir = temp_dir("closkell-cli-inspect-imported-commands");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let commands = temp_dir.join("commands.clsk");
    fs::write(
        &commands,
        "(type ChartMsg (Union {:kind :drawn}))\n\
         (defn canvas-command []\n  {:kind :canvas/draw\n   :ref \"chart\"\n   :ops [{:op :clear}]\n   :msg {:kind :drawn}})\n\
         (ann chart-command (Fn [] (Cmd ChartMsg)))\n\
         (defn chart-command []\n  {:kind :batch\n   :commands [(canvas-command)]})\n",
    )
    .expect("command module should be written");

    let app = temp_dir.join("app.clsk");
    fs::write(
        &app,
        "(import \"./commands.clsk\" [chart-command])\n\
         (def init {:ready true})\n\
         (defn update [state msg]\n  [state (chart-command)])\n\
         (defn view [state]\n  #html <button>{state.ready}</button>)\n",
    )
    .expect("app module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"kind\":\"batch\",\"fields\":[\"commands\",\"kind\"]"),
        "inspect did not include the imported batch command shape:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"kind\":\"canvas/draw\",\"fields\":[\"kind\",\"msg\",\"ops\",\"ref\"]"),
        "inspect did not follow the imported command helper to canvas/draw:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_includes_imported_match_command_helper_schema() {
    let temp_dir = temp_dir("closkell-cli-inspect-imported-match-commands");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let commands = temp_dir.join("commands.clsk");
    fs::write(
        &commands,
        "(type ChartMsg (Union {:kind :drawn}))\n\
         (type ChartState {:live Bool})\n\
         (ann chart-command (Fn [ChartState] (Cmd ChartMsg)))\n\
         (defn chart-command [state]\n  (match state.live\n    true {:kind :canvas/draw\n          :ref \"chart\"\n          :ops [{:op :clear}]\n          :msg {:kind :drawn}}\n    _ {:kind :timer/cancel\n       :id \"chart-redraw\"}))\n",
    )
    .expect("command module should be written");

    let app = temp_dir.join("app.clsk");
    fs::write(
        &app,
        "(import \"./commands.clsk\" [chart-command])\n\
         (def init {:live true})\n\
         (defn update [state msg]\n  [state {:kind :batch\n          :commands [(chart-command state)]}])\n\
         (defn view [state]\n  #html <button>{state.live}</button>)\n",
    )
    .expect("app module should be written");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"kind\":\"batch\",\"fields\":[\"commands\",\"kind\"]"),
        "inspect did not include the update batch command shape:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"kind\":\"canvas/draw\",\"fields\":[\"kind\",\"msg\",\"ops\",\"ref\"],\"sources\":[\"chart-command\"]"),
        "inspect did not follow the imported match helper to canvas/draw:\n{}",
        stdout
    );
    assert!(
        stdout.contains(
            "\"kind\":\"timer/cancel\",\"fields\":[\"id\",\"kind\"],\"sources\":[\"chart-command\"]"
        ),
        "inspect did not include the alternate match branch command:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_reports_hrweb_wrapped_panes_and_source_reads() {
    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(
            workspace_root()
                .join("projects")
                .join("hrweb")
                .join("src")
                .join("app.clsk"),
        )
        .output()
        .expect("closkell inspect should run");

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    for component in ["live-pane", "log-pane", "metrics-pane"] {
        assert!(
            stdout.contains(&format!("\"name\":{}", json_string_for_test(component))),
            "inspect did not include wrapped HRWeb pane `{}`:\n{}",
            component,
            stdout
        );
    }
    assert!(
        stdout.contains(
            "\"component\":\"detail-pane\",\"uses\":[\"live-pane\",\"log-pane\",\"metrics-pane\"]"
        ),
        "inspect did not report detail-pane branch component dependencies:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"path\":\"state.readings\"")
            && stdout.contains("\"template\":\"summary-stat-grid\""),
        "inspect did not map summary stat derived reads back to state.readings:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"path\":\"state.entries\"")
            && stdout.contains("\"template\":\"log-pane\""),
        "inspect did not map log-pane derived reads back to state.entries:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"expr\":\"(latest-bpm-label state)\",\"reads\":[\"state.latestBpm\"]"),
        "inspect did not project the live-pane helper read to state.latestBpm:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"expr\":\"(monitor-status-label state)\",\"reads\":[\"state.appStatus\",\"state.connected?\",\"state.simulated?\",\"state.statusMode\"]"),
        "inspect did not project the connection helper reads to precise state paths:\n{}",
        stdout
    );
    assert!(
        stdout.contains(
            "\"expr\":\"(tab-class state view)\",\"reads\":[\"state.detailView\",\"view\"]"
        ),
        "inspect did not project tab component reads through parameters:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("\"summary.avg\"")
            && !stdout.contains("\"summary.zones\"")
            && !stdout.contains("\"summary.trimp\"")
            && !stdout.contains("\"summary.hrr\""),
        "inspect leaked local summary bindings into HRWeb state reads:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"AppMsg\"") && stdout.contains(":monitor-connected"),
        "inspect did not report the HRWeb message union:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"UpdateResult\"") && stdout.contains("[AppState (Cmd AppMsg)]"),
        "inspect did not report the HRWeb update result type:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"update\"")
            && stdout.contains("(Fn [AppState AppMsg] UpdateResult)"),
        "inspect did not report the HRWeb update annotation:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"name\":\"startup-command\"")
            && stdout.contains("(Fn [Bool] (Cmd AppMsg))"),
        "inspect did not report the HRWeb startup command annotation:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"kind\":\"storage/get\",\"fields\":[\"format\",\"key\",\"kind\",\"onError\",\"toMessage\"],\"sources\":[\"startup-command\"]"),
        "inspect did not report command schema source for startup storage load:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"kind\":\"canvas/draw\"")
            && stdout.contains(
                "\"sources\":[\"heart-chart-batch\",\"heart-chart-command\",\"metric-chart-batch\",\"metric-chart-command\"]"
            ),
        "inspect did not preserve imported chart command sources:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"kind\":\"dom-ref/resize-watch\"")
            && stdout.contains("\"sources\":[\"heart-chart-batch\",\"metric-chart-batch\"]"),
        "inspect did not preserve imported chart resize sources:\n{}",
        stdout
    );
}

#[test]
fn cli_inspect_reports_detail_tabs_alias_reads_without_broad_state() {
    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(
            workspace_root()
                .join("fixtures")
                .join("hrweb")
                .join("hrweb_detail_tabs_app.clsk"),
        )
        .output()
        .expect("closkell inspect should run");

    assert!(
        inspect.status.success(),
        "closkell inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        !stdout.contains("\"path\":\"state\""),
        "inspect should not collapse detail-tabs aliases to broad state reads:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"expr\":\"(count readings)\",\"reads\":[\"state.detailView\",\"state.entries\",\"state.readings\",\"state.selectedLogId\"]"),
        "inspect did not project readings alias through display-readings:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"path\":\"state.entries\",\"slots\"")
            && stdout.contains("\"template\":\"view\",\"slot\":2,\"node\":0,\"kind\":{\"attr\":\"data-readings\"},\"expr\":\"(count readings)\",\"reads\":[\"state.detailView\",\"state.entries\",\"state.readings\",\"state.selectedLogId\"]"),
        "statePathToSlots did not include expression/read details for the readings alias:\n{}",
        stdout
    );
    assert!(
        stdout.contains(
            "\"expr\":\"selected-label\",\"reads\":[\"state.entries\",\"state.selectedLogId\"]"
        ),
        "inspect did not project selected-label alias through selected-log-label:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"expr\":\"(stat-tile \\\"Time\\\" stats.duration)\",\"reads\":[\"state.detailView\",\"state.entries\",\"state.liveStats\",\"state.selectedLogId\"]"),
        "inspect did not project stats alias through display-stats:\n{}",
        stdout
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

fn combined_output(stdout: &[u8], stderr: &[u8]) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    )
}

fn json_string_for_test(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
