use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn runtime_cmd_helpers_cover_error_capable_effect_records() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));
const {{ Cmd }} = runtime;

const http = Cmd.httpRequest({{ url: "/api/log", method: "GET" }}, Symbol.for("loaded"), Symbol.for("load-failed"), Symbol.for("text"));
expectCommand(http, "http/request");
expectSymbol(http.onSuccess, "loaded", "http success");
expectSymbol(http.onError, "load-failed", "http error");
expectSymbol(http.response, "text", "http response");
if (http.request.url !== "/api/log" || http.request.method !== "GET") {{
  throw new Error("http helper did not preserve nested request init");
}}

const httpUrl = Cmd.httpRequest("/api/log", Symbol.for("loaded"), Symbol.for("load-failed"), Symbol.for("text"));
expectCommand(httpUrl, "http/request");
if (httpUrl.url !== "/api/log" || httpUrl.request !== undefined) {{
  throw new Error("http URL helper did not emit top-level URL shape");
}}
expectSymbol(httpUrl.onSuccess, "loaded", "http URL success");
expectSymbol(httpUrl.onError, "load-failed", "http URL error");
expectSymbol(httpUrl.response, "text", "http URL response");

const httpUrlOptions = Cmd.httpRequest("/api/log", {{
  method: "POST",
  headers: {{ "content-type": "application/json" }},
  body: "{{}}",
  response: Symbol.for("json"),
  onSuccess: Symbol.for("posted"),
  onError: Symbol.for("post-failed")
}});
expectCommand(httpUrlOptions, "http/request");
if (httpUrlOptions.url !== "/api/log" || httpUrlOptions.method !== "POST" || httpUrlOptions.body !== "{{}}") {{
  throw new Error("http URL options helper did not preserve fetch init");
}}
if (httpUrlOptions.headers["content-type"] !== "application/json") {{
  throw new Error("http URL options helper did not preserve headers");
}}
expectSymbol(httpUrlOptions.response, "json", "http URL options response");
expectSymbol(httpUrlOptions.onSuccess, "posted", "http URL options success");
expectSymbol(httpUrlOptions.onError, "post-failed", "http URL options error");

const now = Cmd.timeNow(Symbol.for("now"), Symbol.for("clock-failed"));
expectCommand(now, "time/now");
expectSymbol(now.onSuccess, "now", "time success");
expectSymbol(now.onError, "clock-failed", "time error");

const loadState = Cmd.storageGet("hrweb.state", Symbol.for("state-loaded"), Symbol.for("load-failed"), Symbol.for("json"));
expectCommand(loadState, "storage/get");
expectSymbol(loadState.onSuccess, "state-loaded", "storage get success");
expectSymbol(loadState.onError, "load-failed", "storage get error");
expectSymbol(loadState.format, "json", "storage get format");
if (loadState.key !== "hrweb.state") {{
  throw new Error("storage get helper did not preserve key");
}}

const roll = Cmd.randomNumber(1, 5, Symbol.for("rolled"), Symbol.for("random-failed"));
expectCommand(roll, "random/number");
expectSymbol(roll.onSuccess, "rolled", "random success");
expectSymbol(roll.onError, "random-failed", "random error");
if (roll.min !== 1 || roll.max !== 5) {{
  throw new Error("random number helper did not preserve bounds");
}}

const animation = Cmd.animationFrame(Symbol.for("frame"), {{
  id: "hold-frame",
  onSuccess: Symbol.for("frame-ready"),
  onError: Symbol.for("frame-failed")
}});
expectCommand(animation, "animation/frame");
expectSymbol(animation.onFrame, "frame", "animation frame");
expectSymbol(animation.onSuccess, "frame-ready", "animation setup success");
expectSymbol(animation.onError, "frame-failed", "animation setup error");
if (animation.id !== "hold-frame") {{
  throw new Error("animation frame helper did not preserve id");
}}

const animationCancel = Cmd.animationCancel("hold-frame", {{
  onSuccess: Symbol.for("frame-cancelled"),
  onError: Symbol.for("frame-cancel-failed")
}});
expectCommand(animationCancel, "animation/cancel");
expectSymbol(animationCancel.onSuccess, "frame-cancelled", "animation cancel success");
expectSymbol(animationCancel.onError, "frame-cancel-failed", "animation cancel error");
if (animationCancel.id !== "hold-frame") {{
  throw new Error("animation cancel helper did not preserve id");
}}

const simulator = Cmd.simulationHeartRate("sim", {{ ms: 500, min: 120, max: 150, jitter: 4 }}, Symbol.for("sim-connected"), Symbol.for("rate"), Symbol.for("sim-disconnected"), Symbol.for("sim-failed"));
expectCommand(simulator, "simulation/heart-rate");
expectSymbol(simulator.onSuccess, "sim-connected", "simulation success");
expectSymbol(simulator.onReading, "rate", "simulation reading");
expectSymbol(simulator.onDisconnected, "sim-disconnected", "simulation disconnected");
expectSymbol(simulator.onError, "sim-failed", "simulation error");
if (simulator.id !== "sim" || simulator.ms !== 500 || simulator.min !== 120 || simulator.max !== 150 || simulator.jitter !== 4) {{
  throw new Error("simulation helper did not preserve options");
}}

const simulatorStop = Cmd.simulationStop("sim", Symbol.for("sim-stopped"), Symbol.for("sim-stop"), Symbol.for("sim-stop-failed"));
expectCommand(simulatorStop, "simulation/stop");
expectSymbol(simulatorStop.msg, "sim-stopped", "simulation stop msg");
expectSymbol(simulatorStop.onSuccess, "sim-stop", "simulation stop success");
expectSymbol(simulatorStop.onError, "sim-stop-failed", "simulation stop error");

const draw = Cmd.canvasDraw("chart", [{{ op: Symbol.for("clear") }}], Symbol.for("drawn"), Symbol.for("chart-error"), {{
  cssWidth: 320,
  cssHeight: 180,
  devicePixelRatio: true
}});
expectCommand(draw, "canvas/draw");
expectSymbol(draw.msg, "drawn", "canvas success");
expectSymbol(draw.onError, "chart-error", "canvas error");
if (draw.cssWidth !== 320 || draw.cssHeight !== 180 || draw.devicePixelRatio !== true) {{
  throw new Error("canvas draw helper did not preserve sizing options");
}}

const focus = Cmd.domRefFocus("type-picker-input", Symbol.for("focused"), Symbol.for("focus-failed"));
expectCommand(focus, "dom-ref/focus");
expectSymbol(focus.msg, "focused", "focus success");
expectSymbol(focus.onError, "focus-failed", "focus error");

const focusTyped = Cmd.domRefFocus("type-picker-input", {{
  onSuccess: Symbol.for("focused-with-ref"),
  onError: Symbol.for("focus-failed-with-ref")
}});
expectCommand(focusTyped, "dom-ref/focus");
expectSymbol(focusTyped.onSuccess, "focused-with-ref", "focus typed success");
expectSymbol(focusTyped.onError, "focus-failed-with-ref", "focus typed error");
if (focusTyped.ref !== "type-picker-input") {{
  throw new Error("typed focus helper did not preserve ref");
}}

const focusObjectMessage = Cmd.domRefFocus("type-picker-input", {{ kind: Symbol.for("focused-object") }}, Symbol.for("focus-failed"));
expectCommand(focusObjectMessage, "dom-ref/focus");
if (focusObjectMessage.msg.kind !== Symbol.for("focused-object")) {{
  throw new Error("focus helper should keep object-shaped messages as :msg");
}}

const click = Cmd.domRefClick("import-file", Symbol.for("clicked"), Symbol.for("click-failed"));
expectCommand(click, "dom-ref/click");
expectSymbol(click.msg, "clicked", "click success");
expectSymbol(click.onError, "click-failed", "click error");

const clickTyped = Cmd.domRefClick("import-file", {{
  onSuccess: Symbol.for("clicked-with-ref"),
  onError: Symbol.for("click-failed-with-ref")
}});
expectCommand(clickTyped, "dom-ref/click");
expectSymbol(clickTyped.onSuccess, "clicked-with-ref", "click typed success");
expectSymbol(clickTyped.onError, "click-failed-with-ref", "click typed error");

const windowWatch = Cmd.windowEventWatch("keydown", Symbol.for("dev-key"), "dev-hotkey", {{ passive: true }}, Symbol.for("window-failed"));
expectCommand(windowWatch, "window/event-watch");
expectSymbol(windowWatch.onEvent, "dev-key", "window event");
expectSymbol(windowWatch.onError, "window-failed", "window error");
if (windowWatch.id !== "dev-hotkey" || windowWatch.options.passive !== true) {{
  throw new Error("window event helper did not preserve id/options");
}}

const windowUnwatch = Cmd.windowEventUnwatch("dev-hotkey", {{
  onSuccess: Symbol.for("window-stopped"),
  onError: Symbol.for("window-stop-failed")
}});
expectCommand(windowUnwatch, "window/event-unwatch");
expectSymbol(windowUnwatch.onSuccess, "window-stopped", "window unwatch success");
expectSymbol(windowUnwatch.onError, "window-stop-failed", "window unwatch error");

const media = Cmd.mediaQueryWatch("(max-width: 820px)", Symbol.for("media-changed"), "mobile", Symbol.for("media-failed"));
expectCommand(media, "media-query/watch");
expectSymbol(media.onChange, "media-changed", "media change");
expectSymbol(media.onError, "media-failed", "media error");
if (media.id !== "mobile" || media.query !== "(max-width: 820px)") {{
  throw new Error("media query helper did not preserve id/query");
}}

const mediaUnwatch = Cmd.mediaQueryUnwatch("mobile", {{
  onSuccess: Symbol.for("media-stopped"),
  onError: Symbol.for("media-stop-failed")
}});
expectCommand(mediaUnwatch, "media-query/unwatch");
expectSymbol(mediaUnwatch.onSuccess, "media-stopped", "media unwatch success");
expectSymbol(mediaUnwatch.onError, "media-stop-failed", "media unwatch error");

const resizeUnwatch = Cmd.domRefResizeUnwatch("chart", {{
  onSuccess: Symbol.for("resize-stopped"),
  onError: Symbol.for("resize-stop-failed")
}});
expectCommand(resizeUnwatch, "dom-ref/resize-unwatch");
expectSymbol(resizeUnwatch.onSuccess, "resize-stopped", "resize unwatch success");
expectSymbol(resizeUnwatch.onError, "resize-stop-failed", "resize unwatch error");

function expectCommand(command, kind) {{
  if (command.kind !== Symbol.for(kind)) throw new Error(`expected ${{kind}}, found ${{String(command.kind)}}`);
}}

function expectSymbol(value, name, label) {{
  if (value !== Symbol.for(name)) throw new Error(`${{label}} was not :${{name}}`);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    assert!(
        node.status.success(),
        "runtime Cmd helper contract failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn runtime_task_helpers_perform_http_tasks() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));
const {{ Task, Http }} = runtime;

const calls = [];
const handlers = runtime.createCommandHandlers({{
  async fetch(url) {{
    calls.push(url);
    if (url === "/fail") {{
      return {{ ok: false, status: 503, statusText: "Offline", text: async () => "", json: async () => ({{}}) }};
    }}
    return {{ ok: true, status: 200, statusText: "OK", text: async () => `spec:${{url}}`, json: async () => ({{ title: `spec:${{url}}` }}) }};
  }}
}});

const loaded = await handlers["task/perform"](
  Task.perform(
    Task.andThen(
      Http.getText("/spec"),
      (text) => Task.succeed({{ title: text.toUpperCase() }})
    ),
    (spec) => ({{ kind: Symbol.for("loaded"), value: spec }}),
    (error) => ({{ kind: Symbol.for("failed"), error }})
  ),
  () => {{}}
);

if (loaded.kind !== Symbol.for("loaded")) throw new Error("task success did not map to loaded message");
if (loaded.value.title !== "SPEC:/SPEC") throw new Error(`task success payload was wrong: ${{JSON.stringify(loaded.value)}}`);
if (calls[0] !== "/spec") throw new Error("HTTP text task did not call fetch with the URL");

const failed = await handlers["task/perform"](
  Task.perform(
    Task.mapError(Http.getText("/fail"), (error) => `wrapped:${{error}}`),
    (text) => ({{ kind: Symbol.for("loaded"), value: text }}),
    (error) => ({{ kind: Symbol.for("failed"), error }})
  ),
  () => {{}}
);

if (failed.kind !== Symbol.for("failed")) throw new Error("task error did not map to failed message");
if (failed.error !== "wrapped:HTTP 503 Offline") throw new Error(`task error payload was wrong: ${{failed.error}}`);

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    assert!(
        node.status.success(),
        "runtime Task helpers failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn runtime_browser_boot_and_load_commands_use_explicit_host_state() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

const classOps = [];
const host = {{
  location: {{ href: "https://docs.example.test/?url=%2Fopenapi.json&op=get%3A%2Fpets" }},
  document: {{
    documentElement: {{
      classList: {{
        toggle(name, enabled) {{
          classOps.push([name, enabled]);
        }}
      }}
    }}
  }}
}};
const storage = new Map([
  ["better-swagger-theme", "light"],
  ["better-swagger-auth:https://docs.example.test/openapi.json", JSON.stringify([{{ schemeId: "BearerAuth", type: "bearer", token: "persisted-token" }}])]
]);
const storageApi = {{
  getItem(key) {{ return storage.has(String(key)) ? storage.get(String(key)) : null; }},
  setItem(key, value) {{ storage.set(String(key), String(value)); }},
  removeItem(key) {{ storage.delete(String(key)); }}
}};
const handlers = runtime.createCommandHandlers({{ host, storage: storageApi, sessionStorage: storageApi }});

const boot = runtime.createBrowserBootInput({{ host }});
if (boot.currentUrl !== host.location.href) throw new Error("boot input did not capture current URL");

const themeMessage = handlers["browser/theme-load"]({{
  kind: Symbol.for("browser/theme-load"),
  key: "better-swagger-theme",
  toMessage: (theme) => ({{ kind: Symbol.for("theme-loaded"), theme }})
}}, () => {{}});
if (themeMessage.kind !== Symbol.for("theme-loaded") || themeMessage.theme !== "light") {{
  throw new Error(`theme load returned wrong message: ${{JSON.stringify(themeMessage)}}`);
}}
if (classOps.length !== 1 || classOps[0][0] !== "dark" || classOps[0][1] !== false) {{
  throw new Error(`theme load did not apply the stored light theme: ${{JSON.stringify(classOps)}}`);
}}

const authMessage = handlers["auth-storage/load"]({{
  kind: Symbol.for("auth-storage/load"),
  sourceUrl: "https://docs.example.test/openapi.json",
  toMessage: (entries) => ({{ kind: Symbol.for("auth-loaded"), entries }})
}}, () => {{}});
if (authMessage.kind !== Symbol.for("auth-loaded")) throw new Error("auth load did not map through toMessage");
if (authMessage.entries.BearerAuth.token !== "persisted-token") {{
  throw new Error(`auth load returned wrong entries: ${{JSON.stringify(authMessage.entries)}}`);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    assert!(
        node.status.success(),
        "runtime browser boot/load commands failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn runtime_scoped_helpers_wrap_child_effects_and_view_messages() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

const childUpdate = (state, msg) => [
  {{ ...state, count: state.count + 1, last: msg.kind }},
  {{ kind: Symbol.for("time/now"), onSuccess: Symbol.for("child-time") }}
];
const [nextState, scopedCommand] = runtime.scopeUpdate(
  {{ log: {{ count: 1 }}, route: "/logs" }},
  Symbol.for("log"),
  {{ kind: Symbol.for("inc") }},
  childUpdate,
  Symbol.for("log")
);
if (nextState.log.count !== 2 || nextState.route !== "/logs") throw new Error("scopeUpdate did not replace the child state");
if (typeof scopedCommand.onSuccess !== "function") throw new Error("scopeUpdate did not map command continuations");

const handlers = runtime.createCommandHandlers({{ now: () => 42 }});
const parentTime = handlers["time/now"](scopedCommand, () => {{}});
if (parentTime.kind !== Symbol.for("log")) throw new Error("scoped command did not wrap parent kind");
if (parentTime.msg.kind !== Symbol.for("child-time") || parentTime.msg.value !== 42) throw new Error("scoped command did not preserve child success payload");

const scopedSub = runtime.scopeSubscriptions(
  {{ count: 2 }},
  () => runtime.Sub.timerEvery("child-clock", 100, {{ kind: Symbol.for("tick") }}),
  Symbol.for("log")
);
if (scopedSub.kind !== Symbol.for("sub/timer/every")) throw new Error("scopeSubscriptions changed the subscription kind");
if (scopedSub.msg.kind !== Symbol.for("log") || scopedSub.msg.msg.kind !== Symbol.for("tick")) {{
  throw new Error("scopeSubscriptions did not wrap timer messages");
}}

const childMessages = [];
const childView = (state) => ({{
  definition: {{ name: "child-view", params: ["state"] }},
  root: {{ tagName: "BUTTON" }},
  mount(_parent, dispatch) {{
    childMessages.push(["mount", state.count]);
    dispatch({{ kind: Symbol.for("clicked"), count: state.count }});
  }},
  update(nextState, dispatch, updateContext) {{
    childMessages.push(["update", nextState.count, updateContext?.localChangedPaths?.includes("state.count")]);
    dispatch({{ kind: Symbol.for("updated"), count: nextState.count }});
  }},
  dispose() {{
    childMessages.push(["dispose"]);
  }}
}});

const parentMessages = [];
const parentDispatch = (message) => parentMessages.push(message);
parentDispatch.__closkellRefs = new Map();
const scopedView = runtime.scopeView(Symbol.for("log"), childView, {{ count: 2 }});
scopedView.mount({{}}, parentDispatch);
scopedView.update(Symbol.for("log"), childView, {{ count: 3 }}, parentDispatch, {{ changedPaths: ["state.log.count"], frames: [] }});

if (parentMessages.length !== 2) throw new Error(`expected two scoped view messages, saw ${{parentMessages.length}}`);
if (parentMessages[0].kind !== Symbol.for("log") || parentMessages[0].msg.kind !== Symbol.for("clicked")) throw new Error("scopeView did not wrap mount dispatch");
if (parentMessages[1].kind !== Symbol.for("log") || parentMessages[1].msg.kind !== Symbol.for("updated")) throw new Error("scopeView did not wrap update dispatch");
if (childMessages[1][2] !== true) throw new Error("scopeView did not project child local changed paths");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    assert!(
        node.status.success(),
        "runtime scoped helpers failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn runtime_simulation_command_handlers_dispatch_readings_and_cleanup() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

const intervals = new Map();
const cleared = [];
let nextHandle = 0;
const timers = {{
  setInterval(callback, ms) {{
    const handle = `sim-${{++nextHandle}}`;
    intervals.set(handle, {{ callback, ms, active: true }});
    return handle;
  }},
  clearInterval(handle) {{
    cleared.push(handle);
    const interval = intervals.get(handle);
    if (interval) interval.active = false;
  }}
}};

const randoms = [1, 0];
const handlers = runtime.createCommandHandlers({{
  timers,
  random() {{
    if (!randoms.length) throw new Error("unexpected simulation random request");
    return randoms.shift();
  }}
}});
const dispatched = [];
const dispatch = (message) => dispatched.push(message);

const start = {{
  kind: Symbol.for("simulation/heart-rate"),
  id: "sim",
  ms: 250,
  min: 130,
  max: 150,
  jitter: 5,
  start: 140,
  onSuccess: Symbol.for("connected"),
  onReading: Symbol.for("rate"),
  onDisconnected: Symbol.for("disconnected"),
  onError: Symbol.for("failed")
}};

const connected = handlers["simulation/heart-rate"](start, dispatch);
if (connected.kind !== Symbol.for("connected")) throw new Error("simulation start did not return the success message");
if (connected.value.id !== "sim" || connected.value.deviceName !== "Simulated monitor" || connected.value.connected !== true) {{
  throw new Error(`simulation success payload was wrong: ${{JSON.stringify(connected.value)}}`);
}}

const handle = [...intervals.keys()][0];
if (!handle || intervals.get(handle).ms !== 250) throw new Error("simulation interval was not registered");
intervals.get(handle).callback();
intervals.get(handle).callback();
if (dispatched.length !== 2) throw new Error(`expected two readings, saw ${{dispatched.length}}`);
if (dispatched[0].kind !== Symbol.for("rate") || dispatched[0].bpm !== 145) throw new Error("first simulated reading was wrong");
if (dispatched[1].kind !== Symbol.for("rate") || dispatched[1].bpm !== 140) throw new Error("second simulated reading was wrong");

const stopped = handlers["simulation/stop"]({{
  kind: Symbol.for("simulation/stop"),
  id: "sim",
  onSuccess: Symbol.for("stopped")
}}, dispatch);
if (stopped.kind !== Symbol.for("stopped") || stopped.value.id !== "sim") throw new Error("simulation stop success was wrong");
if (cleared[0] !== handle) throw new Error("simulation stop did not clear the interval");
if (dispatched[2]?.kind !== Symbol.for("disconnected")) throw new Error("simulation stop did not dispatch the original disconnected message");
if (intervals.get(handle).active !== false) throw new Error("simulation interval remained active after stop");

handlers.dispose();
if (cleared.length !== 1) throw new Error("dispose should not clear an already stopped simulation twice");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    assert!(
        node.status.success(),
        "runtime simulation command handlers failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn runtime_start_app_diffs_and_disposes_subscriptions() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

const intervals = new Map();
const cleared = [];
let nextHandle = 0;
const timers = {{
  setInterval(callback, ms) {{
    const handle = `timer-${{++nextHandle}}`;
    intervals.set(handle, {{ callback, ms }});
    return handle;
  }},
  clearInterval(handle) {{
    cleared.push(handle);
    intervals.delete(handle);
  }}
}};

const commandHandlers = runtime.createCommandHandlers({{ timers }});
const devEvents = [];
let tickCount = 0;
const app = runtime.startApp({{
  root: {{}},
  init() {{
    return [{{ running: false, ms: 250 }}, {{ kind: Symbol.for("none") }}];
  }},
  update(state, msg) {{
    if (msg.kind === Symbol.for("toggle")) return [{{ ...state, running: !state.running }}, {{ kind: Symbol.for("none") }}];
    if (msg.kind === Symbol.for("set-ms")) return [{{ ...state, ms: msg.ms }}, {{ kind: Symbol.for("none") }}];
    if (msg.kind === Symbol.for("tick")) {{
      tickCount += 1;
      return [state, {{ kind: Symbol.for("none") }}];
    }}
    return [state, {{ kind: Symbol.for("none") }}];
  }},
  view() {{
    return {{
      root: {{}},
      mount() {{}},
      update() {{}},
      dispose() {{}}
    }};
  }},
  subscriptions(state) {{
    return state.running
      ? runtime.Sub.batch([runtime.Sub.timerEvery("clock", state.ms, {{ kind: Symbol.for("tick") }})])
      : runtime.Sub.none;
  }},
  handlers: commandHandlers,
  subscriptionHandlers: runtime.createSubscriptionHandlers({{ commandHandlers }}),
  devtools: (event) => devEvents.push(event)
}});

if (app.subscriptions.length !== 0) throw new Error("inactive app should start with no subscriptions");

app.dispatch({{ kind: Symbol.for("toggle") }});
if (app.subscriptions.length !== 1) throw new Error("running app should start one subscription");
if (intervals.size !== 1) throw new Error("timer subscription did not create an interval");
const firstHandle = [...intervals.keys()][0];
if (intervals.get(firstHandle).ms !== 250) throw new Error("timer subscription used the wrong interval");
intervals.get(firstHandle).callback();
if (tickCount !== 1) throw new Error("timer subscription did not dispatch its message");

app.dispatch({{ kind: Symbol.for("set-ms"), ms: 500 }});
if (cleared[0] !== firstHandle) throw new Error("changed subscription did not stop the old interval");
const secondHandle = [...intervals.keys()][0];
if (secondHandle === firstHandle || intervals.get(secondHandle).ms !== 500) {{
  throw new Error("changed subscription did not start a replacement interval");
}}

app.dispatch({{ kind: Symbol.for("toggle") }});
if (app.subscriptions.length !== 0) throw new Error("stopped app should have no active subscriptions");
if (cleared[1] !== secondHandle) throw new Error("removed subscription did not clear the replacement interval");

const subscriptionEvents = devEvents.filter((event) => event.type?.startsWith("subscription/"));
if (subscriptionEvents.map((event) => event.type).join(",") !== "subscription/start,subscription/stop,subscription/start,subscription/stop") {{
  throw new Error(`unexpected subscription event sequence: ${{subscriptionEvents.map((event) => event.type).join(",")}}`);
}}

app.dispose();
if (intervals.size !== 0) throw new Error("dispose left subscription intervals active");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    assert!(
        node.status.success(),
        "runtime subscription diffing failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn runtime_devtools_overlay_records_renders_and_disposes() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

class Element {{
  constructor(tagName, ownerDocument = null) {{
    this.tagName = tagName;
    this.ownerDocument = ownerDocument;
    this.children = [];
    this.parentNode = null;
    this.attributes = {{}};
    this.style = {{}};
    this.textContent = "";
  }}
  appendChild(child) {{
    child.parentNode = this;
    this.children.push(child);
    return child;
  }}
  removeChild(child) {{
    const index = this.children.indexOf(child);
    if (index >= 0) this.children.splice(index, 1);
    child.parentNode = null;
    return child;
  }}
  replaceChildren(...children) {{
    for (const child of this.children) child.parentNode = null;
    this.children = [];
    for (const child of children) this.appendChild(child);
  }}
  setAttribute(name, value = "") {{
    this.attributes[name] = String(value);
  }}
  get firstChild() {{
    return this.children[0] || null;
  }}
}}

const document = {{
  body: null,
  createElement(tagName) {{
    return new Element(tagName, document);
  }}
}};
document.body = new Element("body", document);

const seen = [];
const overlay = runtime.createDevtoolsOverlay({{
  document,
  maxEvents: 2,
  maxVisible: 2,
  onEvent: (event) => seen.push(event.type)
}});

if (!overlay.root || document.body.children[0] !== overlay.root) {{
  throw new Error("overlay did not mount into document.body");
}}

overlay.emit({{ type: "state/update", message: Symbol.for("start"), changedPaths: ["state.label"] }});
overlay.emit({{ type: "command/run", kind: "storage/set" }});
overlay.emit({{ type: "template/update", name: "view", changedPaths: ["state.saved?"], updatedSlots: [{{ id: 1 }}], skippedSlots: [{{ id: 2 }}, {{ id: 3 }}] }});

if (overlay.events.length !== 2) {{
  throw new Error(`overlay kept ${{overlay.events.length}} events instead of the bounded history`);
}}
if (seen.join(",") !== "state/update,command/run,template/update") {{
  throw new Error(`overlay did not forward events to onEvent: ${{seen.join(",")}}`);
}}

const header = overlay.root.children[0];
const summary = header.children[1];
if (summary.textContent !== "2 events") {{
  throw new Error(`overlay summary was not updated: ${{summary.textContent}}`);
}}

const list = overlay.root.children[1];
if (list.children.length !== 2) {{
  throw new Error(`overlay rendered ${{list.children.length}} rows instead of two`);
}}
if (!list.children[0].textContent.includes("template/update view +1 -2 state.saved?")) {{
  throw new Error(`latest overlay row was not summarized: ${{list.children[0].textContent}}`);
}}
if (!list.children[1].textContent.includes("command/run storage/set")) {{
  throw new Error(`older overlay row was not summarized: ${{list.children[1].textContent}}`);
}}

overlay.dispose();
if (document.body.children.length !== 0 || overlay.root.parentNode !== null) {{
  throw new Error("overlay dispose did not remove the root");
}}
overlay.emit({{ type: "app/dispose" }});
if (overlay.events.length !== 2) {{
  throw new Error("disposed overlay should ignore later events");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    assert!(
        node.status.success(),
        "runtime devtools overlay contract failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_metrics_execute_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_metrics.clsk");
    let output = env::temp_dir().join(format!("closkell-hrweb-metrics-{}.mjs", std::process::id()));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));
if (!mod.in_zone_({{ min: 111, max: 130 }}, 120)) {{
  throw new Error("expected bpm to be inside zone");
}}
if (mod.in_zone_({{ min: 111, max: 130 }}, 150)) {{
  throw new Error("expected bpm to be outside zone");
}}
const adherence = mod.zone2_adherence({{ durationMs: 60000, readings: [{{ bpm: 120, time: 0 }}] }}, 30000);
if (adherence !== 50) {{
  throw new Error(`expected adherence 50, found ${{adherence}}`);
}}
if (mod.zone2_adherence({{ durationMs: 0, readings: [] }}, 0) !== null) {{
  throw new Error("expected empty workout adherence to be null");
}}
if (mod.calculate_zone2_adherence(mod.sample_entry) !== 50) {{
  throw new Error(`expected sample Zone 2 adherence 50, found ${{mod.calculate_zone2_adherence(mod.sample_entry)}}`);
}}
if (mod.zone_duration_ms(mod.sample_entry, 2) !== 30000) {{
  throw new Error(`expected sample Zone 2 duration 30000, found ${{mod.zone_duration_ms(mod.sample_entry, 2)}}`);
}}
if (mod.calculate_trimp(mod.sample_entry) !== 2.5) {{
  throw new Error(`expected sample TRIMP 2.5, found ${{mod.calculate_trimp(mod.sample_entry)}}`);
}}
if (!mod.matches_liss_type_("  LISS steady ride ")) {{
  throw new Error("expected LISS/steady workout to match low-intensity metrics");
}}
if (!mod.matches_hrr_type_("weighted strength intervals")) {{
  throw new Error("expected strength intervals to match HRR metrics");
}}
if (mod.matches_liss_type_(null) || mod.matches_hrr_type_(null)) {{
  throw new Error("expected missing exercise type not to match specialized metrics");
}}
if (metricNames(mod.get_metrics_for_type("steady aerobic")).join(",") !== "zone2,trimp") {{
  throw new Error("expected LISS metrics to include Zone 2 and TRIMP");
}}
if (metricNames(mod.get_metrics_for_type("HIIT tabata")).join(",") !== "hrr,trimp") {{
  throw new Error("expected HIIT metrics to include HRR and TRIMP");
}}
if (metricNames(mod.get_metrics_for_type(null)).join(",") !== "trimp") {{
  throw new Error("expected untyped workout metrics to fall back to TRIMP");
}}
if (mod.reading_at_or_after(mod.sample_hrr_entry.readings, 65000).time !== 70000) {{
  throw new Error("expected recovery reader to find the first reading after the target time");
}}
if (mod.calculate_workout_hrr(mod.sample_hrr_entry) !== 30) {{
  throw new Error(`expected sample HRR 30, found ${{mod.calculate_workout_hrr(mod.sample_hrr_entry)}}`);
}}
if (mod.calculate_workout_hrr(mod.sample_flat_entry) !== null) {{
  throw new Error("expected flat workout HRR to be null");
}}

function metricNames(values) {{
  return values.map((value) => Symbol.keyFor(value));
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_metrics_consumer_imports_local_module() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-hrweb-metrics-import-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_metrics_consumer.clsk");
    let output = temp_dir.join("hrweb_metrics_consumer.mjs");
    let imported_output = temp_dir.join("hrweb_metrics.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(
        imported_output.exists(),
        "recursive build did not emit imported metrics module"
    );
    let emitted = fs::read_to_string(&output).expect("consumer output should be readable");
    assert!(
        emitted.contains("from \"./hrweb_metrics.mjs\""),
        "consumer output did not point at the generated metrics module:\n{}",
        emitted
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({modulePath}));
if (mod.summary.trimp !== 2.5) {{
  throw new Error(`expected imported TRIMP 2.5, found ${{mod.summary.trimp}}`);
}}
if (mod.summary["hrr?"] !== true) {{
  throw new Error("expected imported HRR type matcher to return true");
}}
if (metricNames(mod.summary.metrics).join(",") !== "hrr,trimp") {{
  throw new Error("expected imported metrics list to come from metrics module");
}}

function metricNames(values) {{
  return values.map((value) => Symbol.keyFor(value));
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output)
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
        "generated imported metrics module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_log_transforms_execute_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_log_transforms.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-log-transforms-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));
if (mod.summary.visibleCount !== 2) throw new Error(`expected 2 visible entries, found ${{mod.summary.visibleCount}}`);
if (mod.summary.firstPage.length !== 1 || mod.summary.firstPage[0].id !== "intervals") throw new Error("slice/newest-first did not pick newest entry");
if (mod.summary.bars.map((bar) => bar.label).join(",") !== "warmup,intervals") throw new Error("recent bars were not sorted ascending before take-last");
if (mod.summary.ranked.map((entry) => `${{entry.id}}:${{entry.rank}}`).join(",") !== "intervals:1,warmup:2") throw new Error("ranked options were not indexed in newest-first order");

const before = mod.sample_log.map((entry) => entry.id).join(",");
const sorted = mod.newest_first(mod.sample_log);
const after = mod.sample_log.map((entry) => entry.id).join(",");
if (sorted.map((entry) => entry.id).join(",") !== "intervals,warmup") throw new Error("newest-first sort result was wrong");
if (before !== after || after !== "warmup,intervals,deleted") throw new Error("sort-by-desc mutated the original log vector");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated log transform module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_state_updates_execute_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_state_updates.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-state-updates-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));

const typed = mod.set_exercise_type(mod.initial_state.entries, "warmup", "Strength");
if (typed[0].exerciseType !== "Strength") throw new Error("assoc did not update exercise type");
if (mod.initial_state.entries[0].exerciseType !== "LISS") throw new Error("assoc mutated original entry");

const hidden = mod.hide_entry(typed, "intervals", 4242);
if (hidden[1].hiddenAt !== 4242) throw new Error("assoc did not set hiddenAt");
if (typed[1].hiddenAt !== null) throw new Error("second assoc mutated previous vector");

const zones = mod.set_zone_boundary(mod.initial_state.zones, 0, 125);
if (zones[0].max !== 125 || zones[1].min !== 126) throw new Error("zone boundary update was wrong");
if (mod.initial_state.zones[0].max !== 110 || mod.initial_state.zones[1].min !== 111) throw new Error("zone boundary update mutated original zones");

const imported = mod.import_complete(mod.initial_state, hidden);
if (imported.message !== "Imported" || imported.selectedLogId !== "warmup") throw new Error("merge did not update import state");
if (imported.entries !== hidden) throw new Error("merge did not keep imported entries");

const cleared = mod.clear_message(imported);
if (Object.prototype.hasOwnProperty.call(cleared, "message")) throw new Error("dissoc did not remove message");
if (imported.message !== "Imported") throw new Error("dissoc mutated original state");

if (mod.updated_state.visibleCount !== 1) throw new Error(`expected one visible entry, found ${{mod.updated_state.visibleCount}}`);
if (mod.updated_state.zones[0].max !== 125 || mod.updated_state.zones[1].min !== 126) throw new Error("updated state zones were wrong");
if (Object.prototype.hasOwnProperty.call(mod.updated_state, "message")) throw new Error("updated state should not keep message");

if (mod.bumped_summary.summary.value !== 2) throw new Error("update-in did not update nested value");
if (mod.nested_state.summary.value !== 1) throw new Error("update-in mutated the original nested state");
if (mod.relabeled_summary.summary.label !== "Warmup!") throw new Error("update-in did not pass extra updater args");
if (mod.bumped_summary.summary.label !== "Warmup") throw new Error("second update-in mutated prior nested state");
if (mod.nested_summary_value !== 2) throw new Error("get-in did not read nested value");

if (mod.equal_records !== true) throw new Error("value equality did not compare nested Closkell data structurally");
if (mod.unequal_records !== false) throw new Error("value equality accepted ordered vectors with different values");
if (mod.identical_shared !== true) throw new Error("identical? did not accept the same record reference");
if (mod.identical_distinct !== false) throw new Error("identical? treated distinct equal records as identical");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated state update module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_duration_format_executes_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_duration_format.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-duration-format-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));

if (mod.pad2(4) !== "04") throw new Error("pad2 did not left-pad single digit values");
if (mod.format_duration(0) !== "00:00") throw new Error("zero duration label was wrong");
if (mod.format_duration(61_000) !== "01:01") throw new Error("minute duration label was wrong");
if (mod.format_duration(3_601_000) !== "60:01") throw new Error("exercise timer should stay in minute format");
if (mod.format_zone_duration(0) !== "0:00") throw new Error("zero zone duration label was wrong");
if (mod.format_zone_duration(59_000) !== "0:59") throw new Error("sub-minute zone duration label was wrong");
if (mod.format_zone_duration(61_000) !== "1:01") throw new Error("minute zone duration label was wrong");
if (mod.format_zone_duration(3_661_000) !== "1:01:01") throw new Error("hour zone duration label was wrong");
if (mod.sample_duration_labels.join(",") !== "00:00,01:01,60:01") throw new Error("sample duration labels were wrong");
if (mod.sample_zone_duration_labels.join(",") !== "0:00,0:59,1:01,1:01:01") throw new Error("sample zone duration labels were wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated duration format module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_log_timestamp_formats_date_and_time() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_log_timestamp.clsk");
    let output = env::temp_dir().join(format!("closkell-log-timestamp-{}.mjs", std::process::id()));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));
const formatter = new Intl.DateTimeFormat(undefined, {{
  month: "short",
  day: "numeric",
  hour: "2-digit",
  minute: "2-digit",
}});

const first = mod.sample_log[0];
const expected = formatter.format(new Date(first.stoppedAt));
if (mod.log_date_label(first) !== expected) throw new Error("log timestamp label did not match Intl month/day/time formatting");

const rows = mod.sample_log_rows;
if (rows.length !== 2) throw new Error("sample log rows were not mapped");
if (rows[0].dateLabel !== expected) throw new Error("row date label was wrong");
if (rows[1].label !== "Strength") throw new Error("row label did not preserve exercise type");

const untyped = mod.log_row({{ id: "blank", exerciseType: "", stoppedAt: 1704672000000, durationMs: 60000 }});
if (untyped.label !== "Untyped") throw new Error("empty exercise type should render as Untyped");
if (untyped.dateLabel !== formatter.format(new Date(1704672000000))) throw new Error("dynamic row date label was wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated log timestamp module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_zone_state_executes_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_zone_state.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-zone-state-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));

const loaded = mod.loaded_zone_state;
if (loaded.targetZoneId !== 4) throw new Error("stored string target zone should coerce to 4");
if (loaded.zones.length !== 5) throw new Error(`expected five zones, found ${{loaded.zones.length}}`);
if (loaded.zones.map((zone) => `${{zone.min}}-${{zone.max}}`).join(",") !== "95-106,107-120,121-121,122-170,171-190") {{
  throw new Error("loaded zones were not coerced, clamped, and normalized from stored ranges");
}}
if (loaded.zones[2].id !== 3 || loaded.zones[2].name !== "Zone 3" || loaded.zones[2].color !== "#f0b429") {{
  throw new Error("default zone identity fields were not preserved");
}}

const fallback = mod.fallback_zone_state;
if (fallback.targetZoneId !== 3) throw new Error("fallback target zone was wrong");
if (fallback.zones.map((zone) => `${{zone.min}}-${{zone.max}}`).join(",") !== "90-110,111-130,131-150,151-170,171-190") {{
  throw new Error("fallback zones were not normalized defaults");
}}

const custom = mod.normalize_zones([
  {{ id: 1, name: "A", min: 10, max: 5, color: "#111" }},
  {{ id: 2, name: "B", min: 500, max: 100, color: "#222" }},
]);
if (custom.map((zone) => `${{zone.min}}-${{zone.max}}`).join(",") !== "30-30,31-100") {{
  throw new Error("normalize-zones did not clamp lower bounds and enforce contiguity");
}}

const saved = JSON.parse(mod.saved_zone_json);
if (saved.targetZoneId !== 4) throw new Error("saved payload target zone was wrong");
if (saved.zones.length !== 5) throw new Error("saved payload zone count was wrong");
if (Object.prototype.hasOwnProperty.call(saved.zones[0], "name")) throw new Error("saved payload should omit display-only zone names");
if (Object.prototype.hasOwnProperty.call(saved.zones[0], "color")) throw new Error("saved payload should omit display-only zone colors");
if (saved.zones[0].id !== 1 || saved.zones[0].min !== 95 || saved.zones[0].max !== 106) {{
  throw new Error("saved payload first zone was wrong");
}}

const command = mod.persist_zone_command;
if (command.kind !== Symbol.for("storage/set")) throw new Error("persist command kind was wrong");
if (command.key !== "heartRateExercise.zones.v1") throw new Error("persist command key was wrong");
if (command.msg !== Symbol.for("zones-saved")) throw new Error("persist command msg was wrong");
if (command.value.zones[1].min !== 107 || command.value.targetZoneId !== 4) {{
  throw new Error("persist command value was not the zone-state payload");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated zone state module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_zone_boot_app_loads_stored_zones_and_persists_target() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-zone-boot-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_zone_boot_app.clsk");
    let output = temp_dir.join("zone-boot.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = {{}};
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) {{
      this.children.push(node);
    }} else {{
      this.children.splice(index, 0, node);
    }}
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(type, listener) {{
    this.listeners[type] ||= [];
    this.listeners[type].push(listener);
  }}
  removeEventListener(type, listener) {{
    this.listeners[type] = (this.listeners[type] || []).filter((entry) => entry !== listener);
  }}
  click() {{
    for (const listener of [...(this.listeners.click || [])]) {{
      listener({{ type: "click", currentTarget: this, target: this }});
    }}
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function descendants(node, tagName) {{
  const matches = [];
  for (const child of node.children || []) {{
    if (child.tagName === tagName) matches.push(child);
    matches.push(...descendants(child, tagName));
  }}
  return matches;
}}

function textOf(node) {{
  return (node.children || []).map((child) => "nodeValue" in child ? child.nodeValue : textOf(child)).join("");
}}

function storageWith(value) {{
  return {{
    values: new Map([["heartRateExercise.zones.v1", value]]),
    setItem(key, nextValue) {{
      this.values.set(key, String(nextValue));
    }},
    getItem(key) {{
      return this.values.has(key) ? this.values.get(key) : null;
    }},
    removeItem(key) {{
      this.values.delete(key);
    }}
  }};
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const storedZones = JSON.stringify({{
  zones: [
    {{ min: "95", max: "106" }},
    {{ min: "106", max: "120" }},
    {{ min: 500, max: 100 }}
  ],
  targetZoneId: "4"
}});

const storage = storageWith(storedZones);
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ storage }})
}});

const section = host.children[0];
const buttons = descendants(section, "button");
const status = descendants(section, "p")[0];
if (app.commands.length !== 1 || app.commands[0].kind !== "storage/get") throw new Error("zone boot should log an initial storage/get");
if (app.commands[0].command.key !== "heartRateExercise.zones.v1") throw new Error("zone boot storage key was wrong");
if (app.commands[0].command.format !== Symbol.for("json")) throw new Error("zone boot storage format should be json");
if (app.state.targetZoneId !== 4 || section.attributes["data-target"] !== "4") throw new Error("stored target zone was not loaded");
if (app.state.zones.map((zone) => `${{zone.min}}-${{zone.max}}`).join(",") !== "95-106,107-120,121-121,122-170,171-190") {{
  throw new Error("stored string zones were not coerced and normalized");
}}
if (buttons.length !== 5) throw new Error(`expected five zone buttons, found ${{buttons.length}}`);
if (buttons[0].attributes["data-range"] !== "95-106") throw new Error("first zone range attr was wrong");
if (!buttons[3].hasAttribute("data-selected")) throw new Error("stored target zone button was not selected");
if (textOf(status) !== "Zones loaded") throw new Error("loaded status text was wrong");

const initialFirstButton = buttons[0];
initialFirstButton.click();
if (app.commands.length !== 2 || app.commands[1].kind !== "storage/set") throw new Error("target click should log storage/set");
if (app.state.targetZoneId !== 1 || section.attributes["data-target"] !== "1") throw new Error("target click did not update state");
if (textOf(status) !== "Zones saved") throw new Error("zones-saved completion did not update status");
if (descendants(section, "button")[0] !== initialFirstButton) throw new Error("zone button was replaced after target update");
const saved = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (saved.targetZoneId !== 1 || saved.zones[0].min !== 95 || saved.zones[1].min !== 107) {{
  throw new Error("storage/set did not persist the loaded normalized zone payload");
}}

const badStorage = storageWith("{{");
const badHost = new Element("main");
const badApp = runtime.startApp({{
  root: badHost,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ storage: badStorage }})
}});
const badSection = badHost.children[0];
const badStatus = descendants(badSection, "p")[0];
if (badApp.commands.length !== 1 || badApp.commands[0].kind !== "storage/get") throw new Error("bad zone boot should still log storage/get");
if (badApp.state.targetZoneId !== 3 || badSection.attributes["data-target"] !== "3") throw new Error("bad storage should fall back to default target");
if (!badApp.state.error.includes("JSON") || !badSection.attributes["data-error"].includes("JSON")) {{
  throw new Error("bad storage JSON error was not routed into state");
}}
if (textOf(badStatus) !== "Default zones") throw new Error("bad storage fallback status was wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated zone boot app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_zone_edges_uses_vector_edge_helpers() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_zone_edges.clsk");
    let output = env::temp_dir().join(format!("closkell-zone-edges-{}.mjs", std::process::id()));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));

if (mod.sample_boundaries.length !== 3) throw new Error("drop-last should leave three boundary handles");
if (mod.sample_boundaries.map((edge) => `${{edge.leftId}}-${{edge.rightId}}:${{edge.value}}`).join(",") !== "1-2:119,2-3:139,3-4:159") {{
  throw new Error("zone boundaries did not pair adjacent zones");
}}

const summary = mod.sample_edge_summary;
if (summary.firstMin !== 90 || summary.secondMin !== 120 || summary.lastMax !== 179 || summary.boundaryCount !== 3) {{
  throw new Error(`edge summary was wrong: ${{JSON.stringify(summary)}}`);
}}

const next = mod.zone_boundaries([
  {{ id: 10, name: "A", min: 100, max: 120 }},
  {{ id: 20, name: "B", min: 121, max: 140 }},
  {{ id: 30, name: "C", min: 141, max: 160 }}
]);
if (next.map((edge) => `${{edge.leftId}}-${{edge.rightId}}`).join(",") !== "10-20,20-30") {{
  throw new Error("dynamic zone boundaries were wrong");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated zone edges module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_chart_axis_ops_execute_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_chart_axis_ops.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-chart-axis-ops-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));

if (mod.sample_tick_indices.join(",") !== "0,1,2,3,4") throw new Error("forward range was wrong");
if (mod.reverse_tick_indices.join(",") !== "4,2,0") throw new Error("descending range was wrong");

const ops = mod.sample_axis_ops;
if (ops.length !== 29) throw new Error(`expected 29 axis ops, found ${{ops.length}}`);
if (ops[0].op !== Symbol.for("begin-path")) throw new Error("axis ops should begin with a path");
if (ops[1].op !== Symbol.for("move-to") || ops[1].x !== 48 || ops[1].y !== 214) throw new Error("axis baseline move-to was wrong");
if (ops[2].op !== Symbol.for("line-to") || ops[2].x !== 582 || ops[2].y !== 214) throw new Error("axis baseline line-to was wrong");

const labels = ops.filter((op) => op.op === Symbol.for("fill-text"));
if (labels.length !== 5) throw new Error(`expected five tick labels, found ${{labels.length}}`);
if (labels.map((op) => op.text).join(",") !== "0m,15m,30m,45m,60m") throw new Error("tick labels were wrong");
if (labels.map((op) => op.textAlign).join(",") !== "left,center,center,center,right") throw new Error("edge-aware tick alignment was wrong");
if (!close(labels[1].x, 181.5) || !close(labels[2].x, 315) || !close(labels[3].x, 448.5)) {{
  throw new Error("interior tick x positions were wrong");
}}
if (labels.some((op) => op.fillStyle !== "#617066" || op.font !== "600 12px system-ui" || op.textBaseline !== "top")) {{
  throw new Error("tick label style fields were wrong");
}}

const command = mod.chart_axis_command;
if (command.kind !== Symbol.for("canvas/draw")) throw new Error("axis command kind was wrong");
if (command.ref !== "heart-chart" || command.width !== 600 || command.height !== 260) throw new Error("axis command target or dimensions were wrong");
if (command.msg !== Symbol.for("axis-drawn")) throw new Error("axis command msg was wrong");
if (command.ops !== ops) throw new Error("axis command should reuse generated ops");

function close(left, right) {{
  return Math.abs(left - right) < 0.000001;
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated chart axis ops module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_chart_module_uses_precise_axis_labels() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let source = workspace_root()
        .join("projects")
        .join("hrweb")
        .join("src")
        .join("chart.clsk");
    let temp_dir = env::temp_dir().join(format!(
        "closkell-hrweb-chart-module-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    let output = temp_dir.join("chart.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&source)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));

if (mod.minute_label(120000, 0.5) !== "1.0m") throw new Error("two-minute midpoint label was not fixed precision");
if (mod.minute_label(120000, 0.75) !== "1.5m") throw new Error("sub-ten-minute fractional label was wrong");
if (mod.minute_label(3600000, 0.5) !== "30m") throw new Error("long chart label should be rounded");

const state = {{
  detailView: "live",
  "mobile?": false,
  chartWidth: 600,
  chartHeight: 260,
  displayElapsedMs: 120000,
  readings: [
    {{ bpm: 118, time: 0 }},
    {{ bpm: 136, time: 30000 }},
    {{ bpm: 148, time: 60000 }},
    {{ bpm: 158, time: 90000 }},
    {{ bpm: 142, time: 120000 }}
  ],
  entries: [],
  selectedLogId: null,
  targetZoneId: 3,
  zones: [
    {{ id: 2, name: "Zone 2", min: 111, max: 130, color: "#2a9d8f" }},
    {{ id: 3, name: "Zone 3", min: 131, max: 150, color: "#d9184b" }},
    {{ id: 4, name: "Zone 4", min: 151, max: 170, color: "#f77f00" }}
  ]
}};

const labels = mod.heart_chart_ops(state)
  .filter((op) => op.op === Symbol.for("fill-text"))
  .map((op) => String(op.text))
  .filter((text) => text.endsWith("m"));
if (labels.join(",") !== "0m,0m,1m,1.0m,1.3m,1.7m,2.0m") {{
  throw new Error(`production chart time labels were wrong: ${{labels.join(",")}}`);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
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
        "generated HRWeb chart module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_chart_detail_ops_draw_rich_canvas_state() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_chart_detail_ops.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-chart-detail-ops-{}.mjs",
        std::process::id()
    ));
    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

if (mod.chart_detail_ops.length !== 15) throw new Error(`expected 15 detail ops, found ${{mod.chart_detail_ops.length}}`);
if (mod.chart_detail_command.ops !== mod.chart_detail_ops) throw new Error("chart detail command should reuse generated ops");
if (mod.chart_detail_command.kind !== Symbol.for("canvas/draw")) throw new Error("chart detail command kind was wrong");
if (mod.chart_detail_command.ref !== "trend-chart") throw new Error("chart detail command ref was wrong");

const lineStroke = mod.chart_detail_ops.find((op) => op.op === Symbol.for("stroke") && op.lineCap === "round");
if (!lineStroke || lineStroke.lineJoin !== "round" || lineStroke.globalAlpha !== 0.72) {{
  throw new Error("line stroke op did not preserve rounded state fields");
}}
const latestPoint = mod.chart_detail_ops.find((op) => op.op === Symbol.for("arc"));
if (!close(latestPoint.x, 616) || !close(latestPoint.y, 75.52) || latestPoint.radius !== 7) {{
  throw new Error("latest point arc geometry was wrong");
}}

class Canvas {{
  constructor() {{
    this.width = 0;
    this.height = 0;
    this.context = new Context();
  }}
  getContext(kind) {{
    if (kind !== "2d") return null;
    return this.context;
  }}
}}

class Context {{
  constructor() {{
    this.calls = [];
    this.fillStyle = "";
    this.strokeStyle = "";
    this.lineWidth = 1;
    this.lineCap = "butt";
    this.lineJoin = "miter";
    this.globalAlpha = 1;
    this.font = "";
    this.textAlign = "";
    this.textBaseline = "";
  }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, this.globalAlpha, ...args]); }}
  strokeRect(...args) {{ this.calls.push(["strokeRect", this.strokeStyle, this.lineWidth, this.globalAlpha, ...args]); }}
  beginPath() {{ this.calls.push(["beginPath"]); }}
  moveTo(...args) {{ this.calls.push(["moveTo", ...args]); }}
  lineTo(...args) {{ this.calls.push(["lineTo", ...args]); }}
  arc(...args) {{ this.calls.push(["arc", ...args]); }}
  stroke() {{ this.calls.push(["stroke", this.strokeStyle, this.lineWidth, this.lineCap, this.lineJoin, this.globalAlpha]); }}
  fill() {{ this.calls.push(["fill", this.fillStyle, this.globalAlpha]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, this.textAlign, this.textBaseline, this.globalAlpha, ...args]); }}
}}

const canvas = new Canvas();
const dispatch = () => {{}};
dispatch.__closkellRefs = new Map([["trend-chart", canvas]]);
const message = runtime.createCommandHandlers()["canvas/draw"](mod.chart_detail_command, dispatch);

if (message !== Symbol.for("chart-drawn")) throw new Error("canvas handler did not return draw completion message");
if (canvas.width !== 640 || canvas.height !== 300) throw new Error("canvas dimensions were not applied");

const calls = canvas.context.calls;
const targetFill = calls.find((call) => call[0] === "fillRect" && call[1] === "#d9184b");
if (!targetFill || !close(targetFill[2], 0.14) || !close(targetFill[4], 92.8)) {{
  throw new Error("target zone fill did not apply color, alpha, and y position");
}}
const targetStroke = calls.find((call) => call[0] === "strokeRect");
if (!targetStroke || targetStroke[1] !== "#d9184b" || targetStroke[2] !== 2 || !close(targetStroke[3], 0.34)) {{
  throw new Error("target zone stroke did not apply stroke state");
}}
const stroke = calls.find((call) => call[0] === "stroke");
if (!stroke || stroke[1] !== "#d9184b" || stroke[2] !== 4 || stroke[3] !== "round" || stroke[4] !== "round" || !close(stroke[5], 0.72)) {{
  throw new Error("line stroke did not apply rounded line state");
}}
const fill = calls.find((call) => call[0] === "fill");
if (!fill || fill[1] !== "#d9184b" || fill[2] !== 1) throw new Error("latest point fill did not restore alpha");
const label = calls.find((call) => call[0] === "fillText");
if (!label || label[1] !== "#172019" || label[2] !== "700 12px system-ui" || label[3] !== "right" || label[4] !== "bottom" || label[6] !== "158 bpm") {{
  throw new Error("latest label text state was wrong");
}}

function close(left, right) {{
  return Math.abs(left - right) < 0.000001;
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated chart detail ops module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_heart_chart_app_draws_zones_axis_and_readings() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-heart-chart-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_heart_chart_app.clsk");
    let output = temp_dir.join("heart-chart-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
    this.width = 0;
    this.height = 0;
    this.calls = [];
    this.context = new CanvasContext(this.calls);
  }}
  getContext(name) {{
    if (name !== "2d") return null;
    return this.context;
  }}
}}

class CanvasContext {{
  constructor(calls) {{
    this.calls = calls;
    this.fillStyle = "#000";
    this.strokeStyle = "#000";
    this.lineWidth = 1;
    this.lineCap = "butt";
    this.lineJoin = "miter";
    this.font = "";
    this.textAlign = "start";
    this.textBaseline = "alphabetic";
    this.globalAlpha = 1;
  }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, this.globalAlpha, ...args]); }}
  strokeRect(...args) {{ this.calls.push(["strokeRect", this.strokeStyle, this.lineWidth, this.globalAlpha, ...args]); }}
  beginPath() {{ this.calls.push(["beginPath"]); }}
  moveTo(...args) {{ this.calls.push(["moveTo", ...args]); }}
  lineTo(...args) {{ this.calls.push(["lineTo", ...args]); }}
  arc(...args) {{ this.calls.push(["arc", ...args]); }}
  stroke() {{ this.calls.push(["stroke", this.strokeStyle, this.lineWidth, this.lineCap, this.lineJoin, this.globalAlpha]); }}
  fill() {{ this.calls.push(["fill", this.fillStyle, this.globalAlpha]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, this.textAlign, this.textBaseline, this.globalAlpha, ...args]); }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function key(value) {{
  return typeof value === "symbol" ? Symbol.keyFor(value) : value;
}}

function close(actual, expected) {{
  return Math.abs(actual - expected) < 0.000001;
}}

function descendants(node, tagName) {{
  const matches = [];
  for (const child of node.children || []) {{
    if (child.tagName === tagName) matches.push(child);
    matches.push(...descendants(child, tagName));
  }}
  return matches;
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const ops = mod.sample_heart_chart_ops;

if (ops.length !== 62) throw new Error(`expected 62 heart chart ops, found ${{ops.length}}`);
if (mod.sample_heart_chart_command.kind !== Symbol.for("canvas/draw") || mod.sample_heart_chart_command.ref !== "heart-chart") {{
  throw new Error("heart chart sample command was wrong");
}}
if (mod.sample_heart_chart_command.width !== 600 || mod.sample_heart_chart_command.height !== 260) {{
  throw new Error("heart chart command dimensions were wrong");
}}

const zoneFills = ops.filter((op) => key(op.op) === "fill-rect").slice(1);
if (zoneFills.map((op) => op.fillStyle).join(",") !== "#2a9d8f14,#d9184b34,#f77f0014") {{
  throw new Error("zone band fill colors were wrong");
}}
if (!close(zoneFills[1].y, 78) || !close(zoneFills[1].height, 58) || zoneFills[1].width !== 532) {{
  throw new Error("target zone band geometry was wrong");
}}
const targetStroke = ops.find((op) => key(op.op) === "stroke-rect");
if (!targetStroke || targetStroke.strokeStyle !== "#d9184b" || targetStroke.lineWidth !== 2 || targetStroke.y !== 78 || targetStroke.height !== 57) {{
  throw new Error("target zone stroke op was wrong");
}}

const labels = ops.filter((op) => key(op.op) === "fill-text");
if (labels.slice(0, 4).map((op) => `${{op.text}}:${{op.x}}`).join(",") !== "111:42,130:42,150:42,170:42") {{
  throw new Error("boundary labels were wrong");
}}
if (labels.slice(4).map((op) => `${{op.text}}:${{op.textAlign}}:${{op.x}}`).join("|") !== "0m:left:50|1m:center:183|1.0m:center:316|1.5m:center:449|2.0m:right:582") {{
  throw new Error("time axis labels were wrong");
}}

const readingStrokes = ops.filter((op) => key(op.op) === "stroke" && op.strokeStyle !== "#cbd5ce");
if (readingStrokes.map((op) => `${{op.strokeStyle}}:${{op.globalAlpha}}`).join(",") !== "#d9184b:1,#d9184b:1,#f77f00:0.72,#d9184b:1") {{
  throw new Error("reading segment stroke colors or alpha were wrong");
}}
const latestPoint = ops.find((op) => key(op.op) === "arc");
if (!latestPoint || !close(latestPoint.x, 582) || !close(latestPoint.y, 102.4) || latestPoint.radius !== 5) {{
  throw new Error("latest point geometry was wrong");
}}

const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
const canvas = descendants(section, "canvas")[0];
const button = descendants(section, "button")[0];
const paragraph = descendants(section, "p")[0];
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.getRef("heart-chart") !== canvas) throw new Error("heart chart ref was not registered");
if (app.commands.length !== 1 || app.commands[0].kind !== "canvas/draw") throw new Error("initial heart chart draw was not logged");
if (app.state.draws !== 1 || app.state.status !== "Drawn 1") throw new Error("initial draw completion state was wrong");
if (section.attributes["data-draws"] !== "1" || section.attributes["data-status"] !== "Drawn 1") {{
  throw new Error("initial heart chart attrs did not render");
}}
if (statusText.nodeValue !== "Drawn 1") throw new Error("initial heart chart status text was wrong");
if (canvas.width !== 600 || canvas.height !== 260) throw new Error("heart chart canvas dimensions were not applied");

const calls = canvas.calls;
const runtimeBand = calls.find((call) => call[0] === "fillRect" && call[1] === "#d9184b34");
if (!runtimeBand || !close(runtimeBand[3], 50) || !close(runtimeBand[4], 78) || !close(runtimeBand[6], 58)) {{
  throw new Error("runtime target zone band was wrong");
}}
const runtimeTargetStroke = calls.find((call) => call[0] === "strokeRect");
if (!runtimeTargetStroke || runtimeTargetStroke[1] !== "#d9184b" || runtimeTargetStroke[2] !== 2 || !close(runtimeTargetStroke[5], 78)) {{
  throw new Error("runtime target zone stroke was wrong");
}}
if (!calls.some((call) => call[0] === "fillText" && call[6] === "150" && call[3] === "right" && close(call[8], 78.6))) {{
  throw new Error("runtime boundary label was wrong");
}}
if (!calls.some((call) => call[0] === "fillText" && call[6] === "2.0m" && call[3] === "right" && call[7] === 582)) {{
  throw new Error("runtime time axis label was wrong");
}}
if (!calls.some((call) => call[0] === "stroke" && call[1] === "#f77f00" && call[2] === 4 && call[3] === "round" && call[4] === "round" && close(call[5], 0.72))) {{
  throw new Error("runtime non-target reading stroke was wrong");
}}
if (!calls.some((call) => call[0] === "arc" && close(call[1], 582) && close(call[2], 102.4) && call[3] === 5)) {{
  throw new Error("runtime latest point arc was wrong");
}}
if (!calls.some((call) => call[0] === "fill" && call[1] === "#d9184b" && call[2] === 1)) {{
  throw new Error("runtime latest point fill was wrong");
}}

const initialCanvas = canvas;
const initialText = statusText;
button.click();
if (descendants(section, "canvas")[0] !== initialCanvas) throw new Error("heart chart canvas was replaced after redraw");
if (paragraph.children.find((node) => "nodeValue" in node) !== initialText) throw new Error("heart chart status text was replaced");
if (app.commands.length !== 2 || app.commands[1].kind !== "canvas/draw") throw new Error("redraw command was not logged");
if (app.state.draws !== 2 || section.attributes["data-draws"] !== "2" || initialText.nodeValue !== "Drawn 2") {{
  throw new Error("redraw completion state did not render");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated heart chart app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_metrics_chart_app_draws_trend_and_bar_canvases() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-metrics-chart-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_metrics_chart_app.clsk");
    let output = temp_dir.join("metrics-chart-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
    this.width = 0;
    this.height = 0;
    this.calls = [];
    this.context = new CanvasContext(this.calls);
  }}
  getContext(name) {{
    if (name !== "2d") return null;
    return this.context;
  }}
}}

class CanvasContext {{
  constructor(calls) {{
    this.calls = calls;
    this.fillStyle = "#000";
    this.strokeStyle = "#000";
    this.lineWidth = 1;
    this.lineCap = "butt";
    this.lineJoin = "miter";
    this.font = "";
    this.textAlign = "start";
    this.textBaseline = "alphabetic";
  }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, ...args]); }}
  strokeRect(...args) {{ this.calls.push(["strokeRect", this.strokeStyle, this.lineWidth, ...args]); }}
  beginPath() {{ this.calls.push(["beginPath"]); }}
  moveTo(...args) {{ this.calls.push(["moveTo", ...args]); }}
  lineTo(...args) {{ this.calls.push(["lineTo", ...args]); }}
  arc(...args) {{ this.calls.push(["arc", ...args]); }}
  stroke() {{ this.calls.push(["stroke", this.strokeStyle, this.lineWidth, this.lineCap, this.lineJoin]); }}
  fill() {{ this.calls.push(["fill", this.fillStyle]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, this.textAlign, this.textBaseline, ...args]); }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function key(value) {{
  return typeof value === "symbol" ? Symbol.keyFor(value) : value;
}}

function close(actual, expected) {{
  return Math.abs(actual - expected) < 0.000001;
}}

function descendants(node, tagName) {{
  const matches = [];
  for (const child of node.children || []) {{
    if (child.tagName === tagName) matches.push(child);
    matches.push(...descendants(child, tagName));
  }}
  return matches;
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

if (mod.sample_trend_ops.length !== 44) throw new Error(`expected 44 trend ops, found ${{mod.sample_trend_ops.length}}`);
if (mod.sample_empty_trend_ops.length !== 3) throw new Error("empty trend should draw background plus centered text");
const emptyText = mod.sample_empty_trend_ops[2];
if (key(emptyText.op) !== "fill-text" || emptyText.text !== "Not enough data yet" || emptyText.x !== 180 || emptyText.y !== 90) {{
  throw new Error("empty trend text op was wrong");
}}

const trendLabels = mod.sample_trend_ops.filter((op) => key(op.op) === "fill-text");
if (trendLabels.slice(0, 5).map((op) => op.text).join(",") !== "100%,75%,50%,25%,0%") {{
  throw new Error("trend y-axis labels were wrong");
}}
if (trendLabels.slice(5).map((op) => `${{op.text}}:${{op.textAlign}}:${{op.x}}`).join("|") !== "Jan 1-7:left:34|Jan 8-14:center:192|Jan 15-21:right:350") {{
  throw new Error("trend edge-aware x labels were wrong");
}}
const trendArcs = mod.sample_trend_ops.filter((op) => key(op.op) === "arc");
if (trendArcs.length !== 3 || !close(trendArcs[1].x, 192) || !close(trendArcs[1].y, 53.2) || trendArcs[1].radius !== 4) {{
  throw new Error("trend point arc geometry was wrong");
}}
const trendStroke = mod.sample_trend_ops.find((op) => key(op.op) === "stroke" && op.strokeStyle === "#2a9d8f");
if (!trendStroke || trendStroke.lineWidth !== 3 || trendStroke.lineCap !== "round" || trendStroke.lineJoin !== "round") {{
  throw new Error("trend line stroke did not preserve rounded state");
}}

const barRects = mod.sample_bar_ops.filter((op) => key(op.op) === "fill-rect" && op.fillStyle === "#d9184b");
if (barRects.length !== 3 || !close(barRects[0].x, 36) || !close(barRects[0].width, 90.66666666666667) || !close(barRects[1].height, 124)) {{
  throw new Error("bar geometry was wrong");
}}
const barLabels = mod.sample_bar_ops.filter((op) => key(op.op) === "fill-text");
if (barLabels.map((op) => op.text).join(",") !== "Jan 3,Jan 10,Jan 17") throw new Error("bar labels were wrong");
if (!close(barLabels[0].x, 81.33333333333334) || !close(barLabels[2].x, 290.6666666666667)) {{
  throw new Error("centered bar label positions were wrong");
}}

const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
const canvases = descendants(section, "canvas");
const trendCanvas = canvases[0];
const barCanvas = canvases[1];
const button = descendants(section, "button")[0];
const paragraph = descendants(section, "p")[0];
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.getRef("zone2-trend") !== trendCanvas || app.getRef("trimp-bars") !== barCanvas) throw new Error("chart refs were not registered");
if (app.commands.length !== 2 || app.commands.map((entry) => entry.kind).join(",") !== "canvas/draw,canvas/draw") {{
  throw new Error("initial batched draw commands were not logged");
}}
if (app.commands[0].command.ref !== "zone2-trend" || app.commands[1].command.ref !== "trimp-bars") {{
  throw new Error("initial draw command refs were wrong");
}}
if (app.state.draws !== 2 || app.state.status !== "Drawn bars") throw new Error("initial draw completion state was wrong");
if (section.attributes["data-draws"] !== "2" || section.attributes["data-status"] !== "Drawn bars") {{
  throw new Error("initial chart attrs did not update after draw completions");
}}
if (statusText.nodeValue !== "Drawn bars") throw new Error("initial chart status text was wrong");
if (trendCanvas.width !== 360 || trendCanvas.height !== 180 || barCanvas.width !== 360 || barCanvas.height !== 180) {{
  throw new Error("canvas dimensions were not applied");
}}
if (!trendCanvas.calls.some((call) => call[0] === "stroke" && call[1] === "#2a9d8f" && call[2] === 3 && call[3] === "round" && call[4] === "round")) {{
  throw new Error("trend runtime stroke state was wrong");
}}
if (!trendCanvas.calls.some((call) => call[0] === "arc" && close(call[1], 192) && close(call[2], 53.2) && call[3] === 4)) {{
  throw new Error("trend runtime arc call was wrong");
}}
if (!trendCanvas.calls.some((call) => call[0] === "fillText" && call[5] === "Jan 15-21" && call[3] === "right" && call[6] === 350)) {{
  throw new Error("trend runtime edge label was wrong");
}}
if (!barCanvas.calls.some((call) => call[0] === "fillRect" && call[1] === "#d9184b" && close(call[2], 140.66666666666669) && close(call[4], 90.66666666666667))) {{
  throw new Error("bar runtime fill geometry was wrong");
}}
if (!barCanvas.calls.some((call) => call[0] === "fillText" && call[5] === "Jan 10" && close(call[6], 186.00000000000003))) {{
  throw new Error("bar runtime label position was wrong");
}}

const initialTrendCanvas = trendCanvas;
const initialBarCanvas = barCanvas;
const initialStatusText = statusText;
button.click();
if (descendants(section, "canvas")[0] !== initialTrendCanvas || descendants(section, "canvas")[1] !== initialBarCanvas) {{
  throw new Error("chart canvases were replaced after redraw");
}}
if (paragraph.children.find((node) => "nodeValue" in node) !== initialStatusText) throw new Error("chart status text node was replaced");
if (app.commands.length !== 4) throw new Error("redraw should log two more draw commands");
if (app.state.draws !== 4 || section.attributes["data-draws"] !== "4" || initialStatusText.nodeValue !== "Drawn bars") {{
  throw new Error("redraw completion state did not render");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated metrics chart app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_json_log_codec_roundtrips_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_json_log_codec.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-json-log-codec-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r##"
const mod = await import(fileUrl({path}));
const parsed = JSON.parse(mod.payload);

if (!mod.payload.includes('"version": 2')) throw new Error("payload did not include version 2");
if (!mod.payload.includes('\n  "entries"')) throw new Error("payload was not pretty printed");
if (mod.payload !== mod.export_log(mod.sample_log)) throw new Error("export-log did not reproduce payload");
if (parsed.entries.length !== 2) throw new Error("payload entry count was wrong");
if (mod.roundtrip_count !== 2) throw new Error("roundtrip count was wrong");
if (mod.roundtrip_entries[1].id !== "intervals") throw new Error("roundtrip entries were wrong");
if (mod.roundtrip_entries[0].hiddenAt !== null) throw new Error("nil hiddenAt did not survive as JSON null");
if (mod.export_filename !== "exercise-log-2024-01-06.json") throw new Error("export filename did not use ISO date format");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated JSON log codec module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_import_sanitize_executes_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_import_sanitize.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-import-sanitize-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({path}));
const entries = mod.sanitized_entries;

if (entries.length !== 1) throw new Error(`expected one valid imported entry, found ${{entries.length}}`);
const entry = entries[0];
if (entry.id !== "valid") throw new Error("valid entry id was not preserved");
if (entry.durationMs !== 69000) throw new Error("duration was not preserved");
if (entry.readings.length !== 2) throw new Error(`expected two valid readings, found ${{entry.readings.length}}`);
if (entry.readings.map((reading) => reading.bpm).join(",") !== "122,142") {{
  throw new Error("readings were not filtered and rounded");
}}
if (entry.zones.length !== 2 || entry.zones[0].id !== 2 || entry.zones[0].min !== 111) {{
  throw new Error("invalid zone arrays should fall back to defaults");
}}
if (entry.exerciseType !== "LISS") throw new Error("exercise type was not trimmed");
if (entry.hiddenAt !== null) throw new Error("missing hiddenAt should normalize to null");
if (entry.targetZoneId !== 3) throw new Error("target zone id was not preserved");

const legacy = mod.legacy_entries;
if (legacy.length !== 1) throw new Error(`expected one legacy entry, found ${{legacy.length}}`);
if (legacy[0].targetZoneId !== 3) throw new Error("legacy target zone should default to 3");
if (legacy[0].zones.length !== 2) throw new Error("legacy missing zones should receive defaults");
if (legacy[0].exerciseType !== null) throw new Error("legacy missing exercise type should normalize to null");

const malformed = mod.import_log(JSON.stringify({{ entries: [null, {{ id: "bad", readings: [] }}] }}));
if (malformed.length !== 0) throw new Error("malformed imported entries should be discarded without throwing");
if (mod.missing_entries_error !== "File does not contain an exercise log.") throw new Error("missing entries import error was wrong");
if (mod.malformed_entries_error !== "No valid exercise entries were found.") throw new Error("malformed entries import error was wrong");
if (mod.singular_success_message !== "Replaced log with 1 exercise.") throw new Error("singular import success message was wrong");
if (mod.plural_success_message !== "Replaced log with 2 exercises.") throw new Error("plural import success message was wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated import sanitize module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_exercise_grouping_executes_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_exercise_grouping.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-hrweb-exercise-grouping-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({path}));
const groups = mod.grouped_by_type;

if (groups.map((group) => group.type).join(",") !== "HIIT,LISS,Strength,Untyped") {{
  throw new Error("groups were not sorted by normalized type with Untyped last");
}}
if (groups[1].entries.map((entry) => entry.id).join(",") !== "jog,warmup") {{
  throw new Error("LISS group was not trimmed/merged and sorted newest first");
}}
if (groups[3].entries[0].id !== "untagged") throw new Error("untyped entry was not grouped");
if (mod.sample_group_log.length !== 5) throw new Error("grouping mutated the source log length");
if (mod.sample_group_log[4].exerciseType !== " LISS ") throw new Error("grouping mutated source entries");

const extended = mod.upsert_entry_group(groups, {{ id: "ride", exerciseType: "LISS", stoppedAt: 6000 }});
if (extended === groups) throw new Error("upsert-entry-group returned the original groups vector");
if (extended[1].entries.map((entry) => entry.id).join(",") !== "jog,warmup,ride") {{
  throw new Error("conj did not append to the matching group");
}}
if (groups[1].entries.length !== 2) throw new Error("conj mutated the existing group entries");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        path = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated exercise grouping module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_metric_trends_execute_in_node() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-hrweb-metric-trends-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_metric_trends.clsk");
    let output = temp_dir.join("hrweb_metric_trends.mjs");
    let imported_output = temp_dir.join("hrweb_metrics.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(
        imported_output.exists(),
        "recursive build did not emit imported metrics module"
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));

const weekly = mod.weekly_trimp_trend;
if (weekly.length !== 2) throw new Error(`expected 2 weekly buckets, found ${{weekly.length}}`);
if (weekly[0].timestamp !== startOfWeek(1704326400000)) throw new Error("first weekly timestamp was wrong");
if (weekly[0].value !== 2.5) throw new Error(`first weekly average was wrong: ${{weekly[0].value}}`);
if (weekly[0].label !== groupLabel(weekly[0].timestamp, "week")) throw new Error("first weekly label was wrong");
if (weekly[1].value !== 2) throw new Error(`second weekly average was wrong: ${{weekly[1].value}}`);

const monthly = mod.monthly_trimp_trend;
if (monthly.length !== 2) throw new Error(`expected 2 monthly buckets, found ${{monthly.length}}`);
if (monthly[0].timestamp !== startOfMonth(1704326400000)) throw new Error("first monthly timestamp was wrong");
if (monthly[0].value !== 2.5 || monthly[1].value !== 2) throw new Error("monthly averages were wrong");
if (monthly[0].label !== groupLabel(monthly[0].timestamp, "month")) throw new Error("first monthly label was wrong");

const bars = mod.trimp_bars;
if (bars.length !== 2) throw new Error(`expected 2 TRIMP bars, found ${{bars.length}}`);
if (bars.map((bar) => bar.value).join(",") !== "3,2") throw new Error("TRIMP bars did not keep the latest two entries in order");
if (bars[0].label !== formatDate(1704499200000, "month-day")) throw new Error("first TRIMP bar label was wrong");
if (mod.sample_trend_log.length !== 3) throw new Error("trend build mutated the source log");

function startOfWeek(timestamp) {{
  const date = new Date(timestamp);
  const day = date.getDay();
  const diff = day === 0 ? -6 : 1 - day;
  date.setHours(0, 0, 0, 0);
  date.setDate(date.getDate() + diff);
  return date.getTime();
}}

function startOfMonth(timestamp) {{
  const date = new Date(timestamp);
  date.setHours(0, 0, 0, 0);
  date.setDate(1);
  return date.getTime();
}}

function addDays(timestamp, days) {{
  const date = new Date(timestamp);
  date.setDate(date.getDate() + days);
  return date.getTime();
}}

function formatDate(timestamp, style) {{
  const date = new Date(timestamp);
  if (style === "month-year") return new Intl.DateTimeFormat(undefined, {{ month: "short", year: "2-digit" }}).format(date);
  if (style === "month-day") return new Intl.DateTimeFormat(undefined, {{ month: "short", day: "numeric" }}).format(date);
  if (style === "month") return new Intl.DateTimeFormat(undefined, {{ month: "short" }}).format(date);
  if (style === "day") return new Intl.DateTimeFormat(undefined, {{ day: "numeric" }}).format(date);
  return new Intl.DateTimeFormat(undefined).format(date);
}}

function groupLabel(timestamp, grouping) {{
  if (grouping === "month") return formatDate(timestamp, "month-year");
  const end = addDays(timestamp, 6);
  if (new Date(timestamp).getMonth() === new Date(end).getMonth()) {{
    return `${{formatDate(timestamp, "month")}} ${{new Date(timestamp).getDate()}}-${{new Date(end).getDate()}}`;
  }}
  return `${{formatDate(timestamp, "month-day")}}-${{formatDate(end, "day")}}`;
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
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
        "generated metric trends module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn check_rejects_missing_local_import_export() {
    let temp_dir = env::temp_dir().join(format!("closkell-missing-import-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let lib = temp_dir.join("metrics.clsk");
    let app = temp_dir.join("app.clsk");
    let output = temp_dir.join("app.mjs");
    fs::write(&lib, "(def present 1)").expect("library source should write");
    fs::write(
        &app,
        "(import \"./metrics.clsk\" [missing])\n(def value missing)",
    )
    .expect("app source should write");

    let check = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg(&app)
        .output()
        .expect("closkell check should run");
    assert!(
        !check.status.success(),
        "check unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(
        String::from_utf8_lossy(&check.stdout).contains("import `missing` is not exported"),
        "missing export diagnostic was not printed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&app)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !build.status.success(),
        "build unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
}

#[test]
fn check_validates_imported_modules() {
    let temp_dir =
        env::temp_dir().join(format!("closkell-bad-import-module-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let lib = temp_dir.join("browser.clsk");
    let app = temp_dir.join("app.clsk");
    fs::write(&lib, "(def leaked fetch)").expect("library source should write");
    fs::write(
        &app,
        "(import \"./browser.clsk\" [leaked])\n(def value leaked)",
    )
    .expect("app source should write");

    let check = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("check")
        .arg(&app)
        .output()
        .expect("closkell check should run");
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !check.status.success(),
        "check unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(
        String::from_utf8_lossy(&check.stdout).contains("pure code must return typed command data"),
        "imported module diagnostics were not printed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn inspect_reports_component_graph_slots_and_command_schema() {
    let temp_dir = env::temp_dir().join(format!("closkell-inspect-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let app = temp_dir.join("app.clsk");
    fs::write(
        &app,
        "(def init {:status \"Ready\" :show? true :summary {:label \"Rest\"}})\n\
         (defn empty-card [] #html <p>Empty</p>)\n\
         (defn live-card [state] #html <aside>{state.status}</aside>)\n\
         (defn summary-card [summary] #html <article>{summary.label}</article>)\n\
         (defn update [state msg]\n  [state {:kind :timer/after :id \"hold\" :ms 800 :msg {:kind :done}}])\n\
         (defn view [state]\n  #html <section data-status={state.status}>\n          {(summary-card state.summary)}\n          {(if state.show? (live-card state) #html <div>{(empty-card)}</div>)}\n        </section>)",
    )
    .expect("inspect source should write");

    let inspect = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("inspect")
        .arg(&app)
        .output()
        .expect("closkell inspect should run");
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        inspect.status.success(),
        "inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&inspect.stdout),
        String::from_utf8_lossy(&inspect.stderr)
    );

    let stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(
        stdout.contains("\"commandLogSchema\"")
            && stdout.contains("\"kind\":\"timer/after\"")
            && stdout.contains("\"fields\":[\"id\",\"kind\",\"ms\",\"msg\"]")
            && stdout.contains("\"sources\":[\"update\"]"),
        "inspect did not report command schema\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"componentGraph\"")
            && stdout.contains("\"component\":\"view\"")
            && stdout.contains("\"uses\":[\"empty-card\",\"live-card\",\"summary-card\"]"),
        "inspect did not report component graph\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"statePathToSlots\"")
            && stdout.contains("\"path\":\"state.status\"")
            && stdout.contains("\"path\":\"state.summary.label\"")
            && stdout.contains("\"expr\":\"state.status\",\"reads\":[\"state.status\"]")
            && stdout.contains(
                "\"expr\":\"(summary-card state.summary)\",\"reads\":[\"state.summary.label\"]"
            ),
        "inspect did not report state path slots\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"templates\"")
            && stdout.contains("\"name\":\"summary-card\"")
            && stdout.contains("\"name\":\"view\"")
            && stdout.contains("\"component\":\"summary-card\""),
        "inspect did not report template metadata\n{}",
        stdout
    );
}

#[test]
fn compiled_hrweb_status_view_reuses_dom_nodes() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-status-view-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_status_view.clsk");
    let output = temp_dir.join("status-view.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.disabled = false;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({path}));
const root = new Element("root");
const messages = [];
const firstState = {{ buttonClass: "primary", "connected?": false, label: "Start" }};
const component = mod.status_view(firstState);
component.mount(root, (message) => messages.push(message));

const button = root.children[0];
const text = button.children[0];
if (button.tagName !== "button") throw new Error("expected button root");
if (button.attributes.class !== "primary") throw new Error("initial class was not set");
if (!button.hasAttribute("disabled") || button.disabled !== true) throw new Error("disabled attr was not set");
if (text.nodeValue !== "Start") throw new Error("initial text was not set");

component.update({{ buttonClass: "secondary", "connected?": true, label: "Pause" }});
if (root.children[0] !== button) throw new Error("button node was replaced");
if (button.children[0] !== text) throw new Error("text node was replaced");
if (button.attributes.class !== "secondary") throw new Error("class was not updated");
if (button.hasAttribute("disabled") || button.disabled !== false) throw new Error("disabled attr was not removed");
if (text.nodeValue !== "Pause") throw new Error("text was not updated");

button.click();
if (messages[0] !== Symbol.for("start")) throw new Error("event dispatch did not produce the expected message");

const slotReads = component.definition.slots.map((slot) => slot.reads.join(".")).join("|");
if (!slotReads.includes("state.buttonClass") || !slotReads.includes("state.connected?") || !slotReads.includes("state.label")) {{
  throw new Error(`missing slot read metadata: ${{slotReads}}`);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        path = js_string(&output)
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
        "generated status view failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_mini_app_dispatches_update_and_reuses_dom() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-mini-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_mini_app.clsk");
    let output = temp_dir.join("mini-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.disabled = false;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: (event) => devEvents.push(event)
}});

const button = host.children[0];
const text = button.children[0];
if (app.state["connected?"] !== false) throw new Error("initial state was wrong");
if (button.attributes.class !== "primary") throw new Error("initial class was wrong");
if (!button.hasAttribute("disabled")) throw new Error("initial disabled attr missing");
if (text.nodeValue !== "Start") throw new Error("initial label was wrong");

button.click();
if (app.state["connected?"] !== true) throw new Error("click did not update state");
if (host.children[0] !== button) throw new Error("button was replaced after app update");
if (button.children[0] !== text) throw new Error("text node was replaced after app update");
if (button.attributes.class !== "secondary") throw new Error("updated class was wrong");
if (button.hasAttribute("disabled")) throw new Error("disabled attr should have been removed");
if (text.nodeValue !== "Pause") throw new Error("updated label was wrong");
if (app.commands.length !== 0) throw new Error("Cmd.none should not be logged as an external command");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated mini app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_collection_state_app_updates_set_and_map_slots() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-collection-state-app-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_collection_state_app.clsk");
    let output = temp_dir.join("collection-state-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

const helperSet = runtime.Cmd.storageSet("hrweb.helper", "value", Symbol.for("stored"), Symbol.for("store-failed"));
if (helperSet.kind !== Symbol.for("storage/set") || helperSet.key !== "hrweb.helper" || helperSet.value !== "value") throw new Error("Cmd.storageSet helper emitted the wrong command shape");
if (helperSet.msg !== Symbol.for("stored") || helperSet.onError !== Symbol.for("store-failed")) throw new Error("Cmd.storageSet helper did not preserve success and error continuations");
const helperRemove = runtime.Cmd.storageRemove("hrweb.helper", Symbol.for("removed"), Symbol.for("remove-failed"));
if (helperRemove.kind !== Symbol.for("storage/remove") || helperRemove.key !== "hrweb.helper") throw new Error("Cmd.storageRemove helper emitted the wrong command shape");
if (helperRemove.msg !== Symbol.for("removed") || helperRemove.onError !== Symbol.for("remove-failed")) throw new Error("Cmd.storageRemove helper did not preserve success and error continuations");
const helperRemoveOnSuccess = runtime.Cmd.storageRemove("hrweb.helper", {{
  onSuccess: Symbol.for("removed-with-payload"),
  onError: Symbol.for("remove-failed-with-payload")
}});
if (helperRemoveOnSuccess.kind !== Symbol.for("storage/remove") || helperRemoveOnSuccess.key !== "hrweb.helper") throw new Error("Cmd.storageRemove onSuccess helper emitted the wrong command shape");
if (helperRemoveOnSuccess.msg !== undefined) throw new Error("Cmd.storageRemove onSuccess helper should not emit a legacy msg field");
if (helperRemoveOnSuccess.onSuccess !== Symbol.for("removed-with-payload") || helperRemoveOnSuccess.onError !== Symbol.for("remove-failed-with-payload")) throw new Error("Cmd.storageRemove helper did not preserve typed success continuations");

const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: {{ events: devEvents }}
}});

const section = host.children.find((node) => node.tagName === "section");
const tagButton = byTestId(section, "tag-button");
const mapButton = byTestId(section, "map-button");
const tickButton = byTestId(section, "tick-button");
const tagText = firstText(tagButton);
const mapText = firstText(mapButton);
const tickText = firstText(tickButton);

if (tagText.nodeValue.trim() !== "Tags 1 steady") throw new Error(`initial tag text was wrong: ${{tagText.nodeValue}}`);
if (mapText.nodeValue.trim() !== "Zone2 1") throw new Error(`initial map text was wrong: ${{mapText.nodeValue}}`);
if (tickText.nodeValue.trim() !== "Tick 0") throw new Error(`initial tick text was wrong: ${{tickText.nodeValue}}`);

tagButton.click();
if (tagText.nodeValue.trim() !== "Tags 2 tempo") throw new Error(`Set-backed slot did not update: ${{tagText.nodeValue}}`);
const tagUpdate = devEvents.find((event) => event.type === "state/update" && event.changedPaths.includes("state.tags"));
if (!tagUpdate) throw new Error("Set update did not report state.tags as changed");

mapButton.click();
if (mapText.nodeValue.trim() !== "Zone2 2") throw new Error(`Map-backed slot did not update: ${{mapText.nodeValue}}`);
const mapUpdate = devEvents.find((event) => event.type === "state/update" && event.changedPaths.includes("state.durations"));
if (!mapUpdate) throw new Error("Map update did not report state.durations as changed");

tickButton.click();
if (tickText.nodeValue.trim() !== "Tick 1") throw new Error(`scalar slot did not update: ${{tickText.nodeValue}}`);
if (tagText.nodeValue.trim() !== "Tags 2 tempo" || mapText.nodeValue.trim() !== "Zone2 2") {{
  throw new Error("unrelated scalar update disturbed collection-backed slots");
}}

function byTestId(parent, id) {{
  return parent.children.find((node) => node.attributes?.["data-testid"] === id);
}}

function firstText(node) {{
  return node.children.find((child) => "nodeValue" in child && child.nodeValue.trim() !== "");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated collection state app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_command_app_routes_storage_command() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-command-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_command_app.clsk");
    let output = temp_dir.join("command-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.disabled = false;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const storage = {{
  values: new Map(),
  getItem(key) {{
    return this.values.has(key) ? this.values.get(key) : null;
  }},
  setItem(key, value) {{
    this.values.set(key, value);
  }},
  removeItem(key) {{
    this.values.delete(key);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ storage }}),
  devtools: (event) => devEvents.push(event)
}});

const button = host.children[0];
const text = button.children[0];
button.click();

if (host.children[0] !== button) throw new Error("button was replaced after command update");
if (button.children[0] !== text) throw new Error("text node was replaced after command update");
if (storage.getItem("hrweb.last-action") !== "start") throw new Error("storage command did not persist value");
if (app.commands.length !== 1 || app.commands[0].kind !== "storage/set") throw new Error("storage command was not logged");
if (app.state["saved?"] !== true) throw new Error("stored completion message did not update state");
if (text.nodeValue !== "Saved") throw new Error("completion message did not update label");
if (button.attributes["data-saved"] !== "") throw new Error("saved attr was not updated");

const eventTypes = devEvents.map((event) => event.type);
if (!eventTypes.includes("app/init") || !eventTypes.includes("app/mount")) {{
  throw new Error("devtools did not report app lifecycle events");
}}
const templateEvent = devEvents.find((event) => event.type === "template/mount");
if (!templateEvent || templateEvent.name !== "template0") {{
  throw new Error("devtools did not report the mounted template");
}}
const slotReads = templateEvent.slots.flatMap((slot) => slot.reads || []).join("|");
if (!slotReads.includes("state.buttonClass") || !slotReads.includes("state.saved?") || !slotReads.includes("state.label")) {{
  throw new Error("devtools template metadata did not include state reads");
}}
const startStateEvent = devEvents.find((event) => event.type === "state/update" && event.message === Symbol.for("start"));
if (!startStateEvent || startStateEvent.previousState.label !== "Start" || startStateEvent.state.label !== "Saving") {{
  throw new Error("devtools did not report the start state transition");
}}
if (!startStateEvent.changedPaths.includes("state.connected?") || !startStateEvent.changedPaths.includes("state.label")) {{
  throw new Error(`devtools did not report start changed paths: ${{startStateEvent.changedPaths.join(",")}}`);
}}
const commandEvent = devEvents.find((event) => event.type === "command/run" && event.kind === "storage/set");
if (!commandEvent || commandEvent.command.key !== "hrweb.last-action") {{
  throw new Error("devtools did not report the storage command");
}}
const storedStateEvent = devEvents.find((event) => event.type === "state/update" && event.message === Symbol.for("stored"));
if (!storedStateEvent || storedStateEvent.previousState.label !== "Saving" || storedStateEvent.state.label !== "Saved") {{
  throw new Error("devtools did not report the command completion transition");
}}
if (storedStateEvent.changedPaths.join(",") !== "state.label,state.saved?") {{
  throw new Error(`devtools reported wrong completion changed paths: ${{storedStateEvent.changedPaths.join(",")}}`);
}}
const templateUpdates = devEvents.filter((event) => event.type === "template/update" && event.name === "template0");
if (templateUpdates.length < 2) {{
  throw new Error("devtools did not report template update decisions");
}}
const startTemplateUpdate = templateUpdates.find((event) => event.changedPaths.includes("state.connected?"));
if (!startTemplateUpdate || !slotKinds(startTemplateUpdate.updatedSlots).includes("attr:class") || !slotKinds(startTemplateUpdate.updatedSlots).includes("attr:disabled")) {{
  throw new Error("start update did not update class and disabled slots");
}}
if (!slotKinds(startTemplateUpdate.skippedSlots).includes("attr:data-saved")) {{
  throw new Error("start update did not skip unchanged data-saved slot");
}}
const storedTemplateUpdate = templateUpdates.find((event) => event.changedPaths.includes("state.saved?"));
const storedUpdatedKinds = slotKinds(storedTemplateUpdate?.updatedSlots || []);
const storedSkippedKinds = slotKinds(storedTemplateUpdate?.skippedSlots || []);
if (!storedUpdatedKinds.includes("attr:data-saved") || !storedUpdatedKinds.includes("text")) {{
  throw new Error(`completion update did not update saved/text slots: ${{storedUpdatedKinds.join(",")}}`);
}}
if (!storedSkippedKinds.includes("attr:class") || !storedSkippedKinds.includes("attr:disabled")) {{
  throw new Error(`completion update did not skip class/disabled slots: ${{storedSkippedKinds.join(",")}}`);
}}
app.dispose();
if (!devEvents.some((event) => event.type === "template/dispose" && event.name === "template0")) {{
  throw new Error("devtools did not report template disposal");
}}
if (!devEvents.some((event) => event.type === "app/dispose")) {{
  throw new Error("devtools did not report app disposal");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}

function slotKinds(slots) {{
  return slots.map((slot) => {{
    if (slot.kind === "text") return "text";
    if (slot.kind?.attr) return `attr:${{slot.kind.attr}}`;
    if (slot.kind?.event) return `event:${{slot.kind.event}}`;
    return JSON.stringify(slot.kind);
  }});
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated command app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_boot_app_runs_initial_storage_command() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-boot-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_boot_app.clsk");
    let output = temp_dir.join("boot-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener() {{}}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const storage = {{
  values: new Map([[
    "heartRateExercise.log.v1",
    JSON.stringify({{ version: 2, entries: [{{ id: "a", label: "Walk" }}] }})
  ]]),
  getItem(key) {{
    return this.values.has(key) ? this.values.get(key) : null;
  }},
  setItem(key, value) {{
    this.values.set(key, value);
  }},
  removeItem(key) {{
    this.values.delete(key);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ storage }})
}});

const section = host.children[0];
const span = section.children.find((node) => node.tagName === "span");
const text = span.children.find((node) => "nodeValue" in node);

if (app.commands.length !== 1 || app.commands[0].kind !== "storage/get") throw new Error("initial storage/get command was not logged");
if (app.commands[0].command.key !== "heartRateExercise.log.v1") throw new Error("initial storage/get key was wrong");
if (app.commands[0].command.format !== Symbol.for("json")) throw new Error("initial storage/get format was wrong");
if (app.commands[0].command.onError !== Symbol.for("log-load-failed")) throw new Error("initial storage/get error tag was wrong");
if (app.state["loaded?"] !== true) throw new Error("storage load did not update loaded flag");
if (app.state.entries.length !== 1 || app.state.entries[0].id !== "a") throw new Error("storage load did not parse entries");
if (text.nodeValue !== "Loaded 1") throw new Error(`storage load did not update label: ${{text.nodeValue}}`);
if (section.attributes["data-loaded"] !== "") throw new Error("loaded attr was not updated");
if (section.attributes["data-error"] !== "") throw new Error("successful storage load should not set error attr");

app.dispatch({{ kind: Symbol.for("unknown") }});
if (host.children[0] !== section) throw new Error("section node was replaced after no-op dispatch");
if (span.children.find((node) => "nodeValue" in node) !== text) throw new Error("text node was replaced after no-op dispatch");

const badStorage = {{
  getItem(key) {{
    if (key !== "heartRateExercise.log.v1") throw new Error("unexpected storage key");
    return "{{";
  }},
  setItem() {{}},
  removeItem() {{}}
}};
const badHost = new Element("main");
const badApp = runtime.startApp({{
  root: badHost,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ storage: badStorage }})
}});
const badSection = badHost.children[0];
const badText = badSection.children.find((node) => node.tagName === "span").children.find((node) => "nodeValue" in node);
if (badApp.commands.length !== 1 || badApp.commands[0].kind !== "storage/get") throw new Error("bad storage load command was not logged");
if (badApp.state["loaded?"] !== true) throw new Error("bad storage should still finish loading");
if (badApp.state.entries.length !== 0) throw new Error("bad storage should fall back to empty entries");
if (!badApp.state.error.includes("JSON")) throw new Error(`bad storage error was not routed: ${{badApp.state.error}}`);
if (badText.nodeValue !== "Loaded 0") throw new Error("bad storage fallback label was wrong");
if (badSection.attributes["data-loaded"] !== "") throw new Error("bad storage loaded attr was not set");
if (!badSection.attributes["data-error"].includes("JSON")) throw new Error("bad storage error attr was not set");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated boot app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_timer_app_starts_ticks_and_cancels_interval() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-timer-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_timer_app.clsk");
    let output = temp_dir.join("timer-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

globalThis.__CLOSKELL_ENV__ = {{ DEV: false }};

const intervals = new Map();
const cleared = [];
let nextHandle = 0;
const timers = {{
  setInterval(callback, ms) {{
    const handle = `interval-${{++nextHandle}}`;
    intervals.set(handle, {{ callback, ms, active: true }});
    return handle;
  }},
  clearInterval(handle) {{
    cleared.push(handle);
    const interval = intervals.get(handle);
    if (interval) interval.active = false;
  }},
  setTimeout(callback) {{
    callback();
    return "timeout";
  }}
}};

function runInterval(handle) {{
  const interval = intervals.get(handle);
  if (interval?.active) interval.callback();
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ timers }})
}});

const section = host.children[0];
const buttons = section.children.filter((node) => node.tagName === "button");
const startButton = buttons[0];
const stopButton = buttons[1];
const span = section.children.find((node) => node.tagName === "span");
const text = span.children.find((node) => "nodeValue" in node);

if (text.nodeValue !== "Idle") throw new Error("initial timer label was wrong");
if (section.hasAttribute("data-running")) throw new Error("running attr should start absent");
if (section.attributes["data-ticks"] !== "0") throw new Error("initial tick attr was wrong");

startButton.click();
if (host.children[0] !== section) throw new Error("section was replaced after timer start");
if (span.children.find((node) => "nodeValue" in node) !== text) throw new Error("text node was replaced after timer start");
if (app.commands.length !== 1 || app.commands[0].kind !== "timer/every") throw new Error("timer/every command was not logged");
if (app.commands[0].command.id !== "exercise-clock") throw new Error("timer id was wrong");
if (app.commands[0].command.ms !== 250) throw new Error("timer interval was wrong");
const handle = [...intervals.keys()][0];
if (!handle || intervals.get(handle).ms !== 250) throw new Error("fake interval was not registered");
if (app.state["running?"] !== true) throw new Error("start did not set running state");
if (text.nodeValue !== "Running") throw new Error("start did not update label");
if (section.attributes["data-running"] !== "") throw new Error("running attr was not set");

startButton.click();
if (app.commands.length !== 2 || app.commands[1].kind !== "timer/every") throw new Error("replacement timer/every command was not logged");
if (cleared[0] !== handle) throw new Error("replacement timer did not clear the old interval");
const replacementHandle = [...intervals.keys()].find((candidate) => candidate !== handle);
if (!replacementHandle || intervals.get(replacementHandle).ms !== 250) throw new Error("replacement interval was not registered");

runInterval(handle);
if (app.state.ticks !== 0) throw new Error("old replacement timer still dispatched ticks");
runInterval(replacementHandle);
if (app.state.ticks !== 1) throw new Error("timer tick did not update state");
if (text.nodeValue !== "Ticks 1") throw new Error("timer tick did not update label");
if (section.attributes["data-ticks"] !== "1") throw new Error("timer tick attr was not updated");

stopButton.click();
if (app.commands.length !== 3 || app.commands[2].kind !== "timer/cancel") throw new Error("timer/cancel command was not logged");
if (cleared[1] !== replacementHandle) throw new Error("timer cancel did not clear the replacement handle");
if (app.state["running?"] !== false) throw new Error("stop did not clear running state");
if (text.nodeValue !== "Stopped") throw new Error("stop did not update label");
if (section.hasAttribute("data-running")) throw new Error("running attr was not removed");

runInterval(replacementHandle);
if (app.state.ticks !== 1) throw new Error("cleared timer still dispatched ticks");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated timer app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_hold_control_app_cancels_pending_timeouts() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-hold-control-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_hold_control_app.clsk");
    let output = temp_dir.join("hold-control-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = {{}};
    this.disabled = false;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "disabled") this.disabled = true;
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "disabled") this.disabled = false;
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  pointerdown() {{
    this.emit("pointerdown", {{ type: "pointerdown", currentTarget: this, target: this }});
  }}
  pointerup() {{
    this.emit("pointerup", {{ type: "pointerup", currentTarget: this, target: this }});
  }}
  pointercancel() {{
    this.emit("pointercancel", {{ type: "pointercancel", currentTarget: this, target: this }});
  }}
  keydown(key) {{
    const event = {{
      type: "keydown",
      key,
      currentTarget: this,
      target: this,
      defaultPrevented: false,
      preventDefault() {{
        this.defaultPrevented = true;
      }}
    }};
    this.emit("keydown", event);
    return event;
  }}
  keyup(key) {{
    this.emit("keyup", {{ type: "keyup", key, currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const timeouts = new Map();
const clearedTimeouts = [];
let nextHandle = 0;
const timers = {{
  setTimeout(callback, ms) {{
    const handle = `timeout-${{++nextHandle}}`;
    timeouts.set(handle, {{ callback, ms, active: true }});
    return handle;
  }},
  clearTimeout(handle) {{
    clearedTimeouts.push(handle);
    const timeout = timeouts.get(handle);
    if (timeout) timeout.active = false;
  }},
  setInterval() {{
    throw new Error("hold control app should not start intervals");
  }},
  clearInterval() {{}}
}};

async function runTimeout(handle) {{
  const timeout = timeouts.get(handle);
  if (timeout?.active) {{
    timeout.active = false;
    timeout.callback();
    await Promise.resolve();
  }}
}}

function textOf(element) {{
  const text = element.children.find((node) => "nodeValue" in node);
  if (text) return text.nodeValue;
  const child = element.children.find((node) => node.children);
  return child ? textOf(child) : "";
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ timers }})
}});

const section = host.children[0];
const buttons = section.children.filter((node) => node.tagName === "button");
const [stopButton, deleteButton] = buttons;
const stopFill = stopButton.children.find((node) => node.tagName === "span");
const stopLabel = stopButton.children.find((node) => node.tagName === "strong");
const deleteFill = deleteButton.children.find((node) => node.tagName === "span");
const deleteLabel = deleteButton.children.find((node) => node.tagName === "strong");
const paragraph = section.children.find((node) => node.tagName === "p");
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (statusText.nodeValue !== "Recording") throw new Error("initial hold status was wrong");
if (textOf(stopLabel) !== "Stop" || textOf(deleteLabel) !== "Delete") throw new Error("initial hold labels were wrong");
if (stopFill.style.width !== "0%" || deleteFill.style.width !== "0%") throw new Error("initial hold fills were wrong");
if (stopButton.hasAttribute("disabled") || deleteButton.hasAttribute("disabled")) throw new Error("hold buttons should start enabled");

stopButton.pointerdown();
if (app.commands.length !== 1 || app.commands[0].kind !== "timer/after") throw new Error("stop hold did not start timer/after");
if (app.commands[0].command.id !== "hold-stop" || app.commands[0].command.ms !== 800) throw new Error("stop hold timer payload was wrong");
if (!stopButton.hasAttribute("data-holding")) throw new Error("stop hold attr was not set");
if (stopFill.style.width !== "100%" || textOf(stopLabel) !== "Hold...") throw new Error("stop hold UI did not start");
if (statusText.nodeValue !== "Hold to stop") throw new Error("stop hold status was wrong");
const cancelledHandle = [...timeouts.keys()][0];

stopButton.pointerup();
if (app.commands.length !== 2 || app.commands[1].kind !== "timer/cancel") throw new Error("stop hold cancel command was not logged");
if (clearedTimeouts[0] !== cancelledHandle) throw new Error("timer/cancel did not clear stop timeout");
if (app.state.stopHold !== undefined) throw new Error("generated state should use stopHold? field");
if (app.state["stopHold?"] !== false) throw new Error("stop hold state was not cancelled");
if (stopButton.hasAttribute("data-holding")) throw new Error("stop hold attr was not removed");
if (stopFill.style.width !== "0%" || textOf(stopLabel) !== "Stop") throw new Error("stop hold UI did not cancel");
await runTimeout(cancelledHandle);
if (Symbol.keyFor(app.state.exerciseState) !== "running") throw new Error("cancelled stop timeout still stopped exercise");

const enter = stopButton.keydown("Enter");
if (!enter.defaultPrevented) throw new Error("Enter should prevent default when starting hold");
if (app.commands.length !== 3 || app.commands[2].kind !== "timer/after") throw new Error("keyboard stop hold did not start timer");
const stopHandle = [...timeouts.keys()].find((handle) => handle !== cancelledHandle);
await runTimeout(stopHandle);
if (Symbol.keyFor(app.state.exerciseState) !== "idle") throw new Error("completed stop hold did not stop exercise");
if (app.state["stopHold?"] !== false) throw new Error("completed stop hold did not clear active flag");
if (!section.hasAttribute("data-stopped")) throw new Error("stopped attr was not set");
if (!stopButton.hasAttribute("disabled") || stopButton.disabled !== true) throw new Error("stop button was not disabled after stopping");
if (statusText.nodeValue !== "Stopped") throw new Error("stop completion status was wrong");

deleteButton.pointerdown();
if (app.commands.length !== 4 || app.commands[3].kind !== "timer/after") throw new Error("delete hold did not start timer/after");
if (app.commands[3].command.id !== "hold-delete") throw new Error("delete hold timer id was wrong");
if (deleteFill.style.width !== "100%" || textOf(deleteLabel) !== "Hold") throw new Error("delete hold UI did not start");
const deleteCancelHandle = [...timeouts.keys()].find((handle) => ![cancelledHandle, stopHandle].includes(handle));
deleteButton.pointercancel();
if (app.commands.length !== 5 || app.commands[4].kind !== "timer/cancel") throw new Error("delete pointercancel did not cancel timer");
if (clearedTimeouts[1] !== deleteCancelHandle) throw new Error("timer/cancel did not clear delete timeout");
await runTimeout(deleteCancelHandle);
if (app.state["deleted?"] !== false) throw new Error("cancelled delete timeout still deleted log");

const space = deleteButton.keydown(" ");
if (!space.defaultPrevented) throw new Error("Space should prevent default when starting delete hold");
if (app.commands.length !== 6 || app.commands[5].kind !== "timer/after") throw new Error("keyboard delete hold did not start timer");
const deleteHandle = [...timeouts.keys()].find((handle) => ![cancelledHandle, stopHandle, deleteCancelHandle].includes(handle));
await runTimeout(deleteHandle);
if (app.state["deleted?"] !== true) throw new Error("completed delete hold did not delete log");
if (app.state.selectedLogId !== null) throw new Error("delete hold did not clear selected log");
if (!section.hasAttribute("data-deleted")) throw new Error("deleted attr was not set");
if (!deleteButton.hasAttribute("disabled") || deleteButton.disabled !== true) throw new Error("delete button was not disabled after deleting");
if (statusText.nodeValue !== "Deleted") throw new Error("delete completion status was wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated hold control app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_log_delete_hold_app_hides_and_persists_selected_log() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-log-delete-hold-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_log_delete_hold_app.clsk");
    let output = temp_dir.join("log-delete-hold-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = {{}};
    this.disabled = false;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "disabled") this.disabled = true;
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "disabled") this.disabled = false;
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
  pointerdown() {{
    const event = preventableEvent("pointerdown", this);
    this.emit("pointerdown", event);
    return event;
  }}
  pointercancel() {{
    this.emit("pointercancel", {{ type: "pointercancel", currentTarget: this, target: this }});
  }}
  keydown(key) {{
    const event = preventableEvent("keydown", this);
    event.key = key;
    this.emit("keydown", event);
    return event;
  }}
  keyup(key) {{
    this.emit("keyup", {{ type: "keyup", key, currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function preventableEvent(type, target) {{
  return {{
    type,
    currentTarget: target,
    target,
    defaultPrevented: false,
    preventDefault() {{
      this.defaultPrevented = true;
    }}
  }};
}}

const timeouts = new Map();
const clearedTimeouts = [];
let nextTimeout = 0;
const timers = {{
  setTimeout(callback, ms) {{
    const handle = `timeout-${{++nextTimeout}}`;
    timeouts.set(handle, {{ callback, ms, active: true }});
    return handle;
  }},
  clearTimeout(handle) {{
    clearedTimeouts.push(handle);
    const timeout = timeouts.get(handle);
    if (timeout) timeout.active = false;
  }},
  setInterval() {{
    throw new Error("log delete hold should not start intervals");
  }},
  clearInterval() {{}}
}};

const frames = new Map();
const cancelledFrames = [];
let nextFrame = 0;
function requestAnimationFrame(callback) {{
  const handle = `frame-${{++nextFrame}}`;
  frames.set(handle, {{ callback, active: true }});
  return handle;
}}
function cancelAnimationFrame(handle) {{
  cancelledFrames.push(handle);
  const frame = frames.get(handle);
  if (frame) frame.active = false;
}}

async function runTimeout(handle) {{
  const timeout = timeouts.get(handle);
  if (timeout?.active) {{
    timeout.active = false;
    timeout.callback();
    await Promise.resolve();
  }}
}}

function runFrame(handle, timestamp) {{
  const frame = frames.get(handle);
  if (!frame?.active) return;
  frame.active = false;
  frame.callback(timestamp);
}}

class MemoryStorage {{
  constructor() {{
    this.values = new Map();
  }}
  getItem(key) {{
    return this.values.has(key) ? this.values.get(key) : null;
  }}
  setItem(key, value) {{
    this.values.set(key, value);
  }}
  removeItem(key) {{
    this.values.delete(key);
  }}
}}

let nowValue = 1000;
const storage = new MemoryStorage();
const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{
    timers,
    requestAnimationFrame,
    cancelAnimationFrame,
    now: () => nowValue,
    storage
  }})
}});

const section = host.children[0];
const list = childByAttr(section, "div", "data-list", "exercise-log");
const deleteButton = childByAttr(section, "button", "data-action", "delete");
const fill = deleteButton.children.find((node) => node.tagName === "span");
const label = textOf(deleteButton.children.find((node) => node.tagName === "strong"));
const statusText = textOf(section.children.filter((node) => node.tagName === "p").at(-1));
let rows = logRows();
const warmupRow = rows[0];
const intervalRow = rows[1];
const detailTitle = textOf(childByAttr(section, "article", "data-detail", "warmup").children.find((node) => node.tagName === "h2"));

if (section.attributes["data-visible-count"] !== "2" || section.attributes["data-hidden-count"] !== "1") {{
  throw new Error("initial visible/hidden counts were wrong");
}}
if (section.attributes["data-selected"] !== "warmup") throw new Error("initial selected log was wrong");
if (rows.length !== 2 || rows[0].attributes["data-id"] !== "warmup" || rows[1].attributes["data-id"] !== "intervals") {{
  throw new Error("initial log rows were wrong");
}}
if (!warmupRow.hasAttribute("data-selected") || intervalRow.hasAttribute("data-selected")) throw new Error("initial row selection attrs were wrong");
if (deleteButton.hasAttribute("disabled") || deleteButton.disabled) throw new Error("delete button should start enabled");
if (fill.style.width !== "0%" || label.nodeValue !== "Delete" || statusText.nodeValue !== "Ready") {{
  throw new Error("initial delete hold UI was wrong");
}}

const down = deleteButton.pointerdown();
if (!down.defaultPrevented) throw new Error("pointerdown should prevent default");
if (commandKinds().join(",") !== "time/now,timer/after,animation/frame") throw new Error(`start commands were wrong: ${{commandKinds().join(",")}}`);
if (app.commands[1].command.id !== "delete-log-hold" || app.commands[1].command.ms !== 1000) throw new Error("delete timer command payload was wrong");
if (app.commands[2].command.id !== "delete-log-progress") throw new Error("delete frame command payload was wrong");
if (!section.hasAttribute("data-holding") || !deleteButton.hasAttribute("data-holding")) throw new Error("holding attrs were not set");
if (app.state["deleteHoldStartedAt"] !== 1000 || app.state["deleteHold?"] !== true) throw new Error("hold start state was wrong");
if (label.nodeValue !== "Hold") throw new Error("delete label did not enter hold mode");
const firstTimer = [...timeouts.keys()][0];
const firstFrame = [...frames.keys()][0];

runFrame(firstFrame, 1400);
if (app.state.deleteHoldProgress !== 0.4) throw new Error(`expected 40% progress, found ${{app.state.deleteHoldProgress}}`);
if (fill.style.width !== "40%" || section.attributes["data-progress"] !== "0.4") throw new Error("progress UI did not update");
if (statusText.nodeValue !== "Deleting 40%") throw new Error("progress status was wrong");
if (commandKinds().at(-1) !== "animation/frame") throw new Error("progress frame did not schedule the next frame");
const secondFrame = [...frames.keys()].find((handle) => handle !== firstFrame);

deleteButton.pointercancel();
if (commandKinds().slice(-2).join(",") !== "timer/cancel,animation/cancel") throw new Error("cancel commands were wrong");
if (clearedTimeouts[0] !== firstTimer) throw new Error("timer/cancel did not clear the active timeout");
if (cancelledFrames[0] !== secondFrame) throw new Error("animation/cancel did not cancel the active frame");
if (section.hasAttribute("data-holding") || deleteButton.hasAttribute("data-holding")) throw new Error("holding attrs were not removed after cancel");
if (app.state["deleteHold?"] !== false || app.state.deleteHoldProgress !== 0) throw new Error("cancel did not reset hold state");
if (fill.style.width !== "0%" || label.nodeValue !== "Delete" || statusText.nodeValue !== "Delete cancelled") {{
  throw new Error("cancel did not reset delete UI");
}}
await runTimeout(firstTimer);
if (app.state.entries.find((entry) => entry.id === "warmup").hiddenAt !== null) throw new Error("cancelled timeout still hid the log");
if (section.attributes["data-visible-count"] !== "2" || section.attributes["data-hidden-count"] !== "1") {{
  throw new Error("cancel should not change visible/hidden counts");
}}

intervalRow.click();
if (app.state.selectedLogId !== "intervals") throw new Error("selecting the second row did not update state");
if (section.attributes["data-selected"] !== "intervals") throw new Error("selected attr did not switch to intervals");
if (detailTitle.nodeValue !== "Short intervals") throw new Error("detail title text did not update in place");
if (logRows()[0] !== warmupRow || logRows()[1] !== intervalRow) throw new Error("keyed rows were replaced on selection");

nowValue = 2000;
const space = deleteButton.keydown(" ");
if (!space.defaultPrevented) throw new Error("Space should prevent default when starting delete hold");
if (commandKinds().slice(-3).join(",") !== "time/now,timer/after,animation/frame") throw new Error("keyboard start commands were wrong");
const completeTimer = [...timeouts.keys()].find((handle) => handle !== firstTimer);
const progressFrame = latestActiveFrame();
runFrame(progressFrame, 2500);
if (fill.style.width !== "50%" || app.state.deleteHoldProgress !== 0.5) throw new Error("second hold progress was wrong");

nowValue = 3200;
await runTimeout(completeTimer);
if (commandKinds().slice(-2).join(",") !== "time/now,storage/set") throw new Error(`completion commands were wrong: ${{commandKinds().slice(-4).join(",")}}`);
if (app.state.selectedLogId !== "warmup") throw new Error("delete completion did not select the next visible log");
if (app.state["deleteHold?"] !== false || app.state.deleteHoldProgress !== 0) throw new Error("delete completion did not reset hold state");
if (app.state.entries.find((entry) => entry.id === "intervals").hiddenAt !== 3200) throw new Error("selected log was not hidden at the completion time");
if (app.state.entries.find((entry) => entry.id === "warmup").hiddenAt !== null) throw new Error("unselected visible log was hidden");
if (section.attributes["data-visible-count"] !== "1" || section.attributes["data-hidden-count"] !== "2") {{
  throw new Error("delete completion counts were wrong");
}}
if (logRows().length !== 1 || logRows()[0] !== warmupRow || logRows()[0].attributes["data-id"] !== "warmup") {{
  throw new Error("delete completion should remove only the deleted row and reuse warmup");
}}
if (intervalRow.parentNode !== null) throw new Error("deleted row was not detached");
if (section.attributes["data-selected"] !== "warmup" || !warmupRow.hasAttribute("data-selected")) throw new Error("warmup was not selected after delete");
if (detailTitle.nodeValue !== "Warmup walk") throw new Error("detail title did not return to warmup");
if (deleteButton.hasAttribute("disabled") || deleteButton.disabled) throw new Error("delete button should stay enabled with a remaining selected log");
if (fill.style.width !== "0%" || label.nodeValue !== "Delete") throw new Error("delete UI did not reset after completion");
if (statusText.nodeValue !== "Deleted, selected warmup and saved") throw new Error("saved status was wrong");

const saved = JSON.parse(storage.getItem("heartRateExercise.log.v1"));
if (saved.version !== 2 || saved.entries.length !== 3) throw new Error("saved log payload shape was wrong");
const savedIntervals = saved.entries.find((entry) => entry.id === "intervals");
if (savedIntervals.hiddenAt !== 3200) throw new Error("saved payload did not persist hiddenAt");
if (saved.entries.find((entry) => entry.id === "warmup").hiddenAt !== null) throw new Error("saved payload mutated warmup");

function logRows() {{
  return list.children.filter((node) => node.tagName === "button");
}}

function latestActiveFrame() {{
  return [...frames.entries()].filter(([, frame]) => frame.active).at(-1)?.[0];
}}

function commandKinds() {{
  return app.commands.map((entry) => entry.kind);
}}

function childByAttr(parent, tagName, name, value) {{
  return parent.children.find((node) => node.tagName === tagName && node.attributes?.[name] === value);
}}

function textOf(parent) {{
  return parent.children.find((node) => "nodeValue" in node);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated log delete hold app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_hold_progress_app_updates_animation_frames() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-hold-progress-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_hold_progress_app.clsk");
    let output = temp_dir.join("hold-progress-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.value = "";
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "value") this.value = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "value") this.value = "";
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  pointerdown() {{
    const event = {{
      type: "pointerdown",
      currentTarget: this,
      target: this,
      defaultPrevented: false,
      preventDefault() {{
        this.defaultPrevented = true;
      }}
    }};
    this.emit("pointerdown", event);
    return event;
  }}
  pointerup() {{
    this.emit("pointerup", {{ type: "pointerup", currentTarget: this, target: this }});
  }}
  pointercancel() {{
    this.emit("pointercancel", {{ type: "pointercancel", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

let currentNow = 1000;
let nextFrameHandle = 0;
const frames = new Map();
const cancelled = [];
const animation = {{
  requestAnimationFrame(callback) {{
    const handle = ++nextFrameHandle;
    frames.set(handle, callback);
    return handle;
  }},
  cancelAnimationFrame(handle) {{
    cancelled.push(handle);
    frames.delete(handle);
  }}
}};

function triggerFrame(handle, timestamp) {{
  const callback = frames.get(handle);
  if (!callback) return false;
  frames.delete(handle);
  callback(timestamp);
  return true;
}}

function elements(node, tagName) {{
  return node.children.filter((child) => child.tagName === tagName);
}}

function textOf(node) {{
  return node.children.find((child) => "nodeValue" in child).nodeValue;
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ animation, now: () => currentNow }})
}});

const section = host.children[0];
const button = elements(section, "button")[0];
const progress = elements(section, "progress")[0];
const paragraph = elements(section, "p")[0];
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.commands.length !== 0) throw new Error("hold progress app should not emit an initial command");
if (section.hasAttribute("data-holding")) throw new Error("initial holding attr should be absent");
if (section.hasAttribute("data-completed")) throw new Error("initial completed attr should be absent");
if (section.attributes["data-progress"] !== "0") throw new Error("initial progress attr was wrong");
if (section.attributes["data-frames"] !== "0") throw new Error("initial frame count attr was wrong");
if (progress.attributes.value !== "0" || progress.attributes.max !== "1") throw new Error("initial progress element attrs were wrong");
if (statusText.nodeValue !== "Idle") throw new Error("initial status was wrong");

const down = button.pointerdown();
if (!down.defaultPrevented) throw new Error("pointerdown did not prevent default");
if (app.commands.length !== 2) throw new Error(`expected time/now and animation/frame, found ${{app.commands.length}}`);
if (app.commands[0].kind !== "time/now" || app.commands[1].kind !== "animation/frame") throw new Error("hold start command sequence was wrong");
if (app.commands[1].command.id !== "hold-progress") throw new Error("animation frame id was wrong");
if (app.state["holding?"] !== true || app.state.startedAt !== 1000) throw new Error("hold start state was wrong");
if (section.attributes["data-holding"] !== "") throw new Error("holding attr was not set");
if (statusText.nodeValue !== "Holding") throw new Error("hold start status was wrong");
if (frames.size !== 1 || !frames.has(1)) throw new Error("first animation frame was not scheduled");

triggerFrame(1, 1200);
if (app.commands.length !== 3 || app.commands[2].kind !== "animation/frame") throw new Error("partial frame should schedule the next frame");
if (app.state.frames !== 1) throw new Error("frame count did not increment");
if (app.state.progress !== 0.25) throw new Error(`partial progress was wrong: ${{app.state.progress}}`);
if (section.attributes["data-progress"] !== "0.25" || progress.attributes.value !== "0.25") throw new Error("partial progress did not render");
if (statusText.nodeValue !== "Progress 25%") throw new Error("partial progress status was wrong");
if (!frames.has(2)) throw new Error("second animation frame was not scheduled");

button.pointerup();
if (app.commands.length !== 4 || app.commands[3].kind !== "animation/cancel") throw new Error("pointerup should cancel the pending frame");
if (cancelled[0] !== 2) throw new Error("animation/cancel did not cancel the pending handle");
if (app.state["holding?"] !== false || app.state.progress !== 0) throw new Error("cancel state was wrong");
if (section.hasAttribute("data-holding")) throw new Error("holding attr was not removed after cancel");
if (statusText.nodeValue !== "Cancelled") throw new Error("cancel status was wrong");
if (triggerFrame(2, 1400)) throw new Error("cancelled frame still fired");
if (app.commands.length !== 4) throw new Error("cancelled frame emitted another command");

currentNow = 2000;
button.pointerdown();
if (app.commands.length !== 6 || app.commands[4].kind !== "time/now" || app.commands[5].kind !== "animation/frame") {{
  throw new Error("restart did not request time and frame");
}}
if (!frames.has(3)) throw new Error("restart frame was not scheduled");
triggerFrame(3, 2400);
if (app.state.progress !== 0.5 || app.state.frames !== 1) throw new Error("restart partial frame was wrong");
if (app.commands.length !== 7 || app.commands[6].kind !== "animation/frame") throw new Error("restart partial frame did not schedule another frame");
if (!frames.has(4)) throw new Error("completion frame was not scheduled");
triggerFrame(4, 2800);
if (app.state["holding?"] !== false || app.state["completed?"] !== true) throw new Error("completion flags were wrong");
if (app.state.progress !== 1 || app.state.frames !== 2) throw new Error("completion progress was wrong");
if (section.hasAttribute("data-holding")) throw new Error("holding attr remained after completion");
if (section.attributes["data-completed"] !== "") throw new Error("completed attr was not set");
if (section.attributes["data-progress"] !== "1" || progress.attributes.value !== "1") throw new Error("completion progress did not render");
if (statusText.nodeValue !== "Complete") throw new Error("completion status was wrong");
if (frames.size !== 0) throw new Error("no animation frames should remain after completion");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated hold progress app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_simulated_monitor_routes_simulation_command() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-simulated-monitor-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_simulated_monitor.clsk");
    let output = temp_dir.join("simulated-monitor.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const intervals = new Map();
const cleared = [];
let nextHandle = 0;
const timers = {{
  setInterval(callback, ms) {{
    const handle = `interval-${{++nextHandle}}`;
    intervals.set(handle, {{ callback, ms, active: true }});
    return handle;
  }},
  clearInterval(handle) {{
    cleared.push(handle);
    const interval = intervals.get(handle);
    if (interval) interval.active = false;
  }},
  setTimeout(callback) {{
    callback();
    return "timeout";
  }}
}};

function runInterval(handle) {{
  const interval = intervals.get(handle);
  if (interval?.active) interval.callback();
}}

const randoms = [0.75, 0.25, 0.9];
function random() {{
  if (!randoms.length) throw new Error("unexpected random request");
  return randoms.shift();
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ random, timers }})
}});

const section = host.children[0];
const buttons = section.children.filter((node) => node.tagName === "button");
const simulateButton = buttons[0];
const stopButton = buttons[1];
const statusSpan = section.children.find((node) => node.tagName === "span");
const bpmStrong = section.children.find((node) => node.tagName === "strong");
const statusText = statusSpan.children.find((node) => "nodeValue" in node);
const bpmText = bpmStrong.children.find((node) => "nodeValue" in node);

if (section.hasAttribute("data-connected")) throw new Error("simulator should start disconnected");
if (section.attributes["data-bpm"] !== "0") throw new Error("initial bpm attr was wrong");
if (statusText.nodeValue !== "Idle") throw new Error("initial simulator status was wrong");

simulateButton.click();
if (app.commands.length !== 0) throw new Error("production simulator click should not emit commands");
if (app.state["connected?"] !== false) throw new Error("production simulator click should leave state disconnected");
if (statusText.nodeValue !== "Idle" || bpmText.nodeValue !== "0") throw new Error("production simulator click should not update DOM text");
if (section.hasAttribute("data-connected")) throw new Error("production simulator click should not set connected attr");

globalThis.__CLOSKELL_ENV__ = {{ DEV: true }};

simulateButton.click();
if (host.children[0] !== section) throw new Error("simulator section was replaced");
if (statusSpan.children.find((node) => "nodeValue" in node) !== statusText) throw new Error("status text node was replaced");
if (bpmStrong.children.find((node) => "nodeValue" in node) !== bpmText) throw new Error("bpm text node was replaced");
if (app.commands.map((entry) => entry.kind).join(",") !== "random/number,random/number,simulation/heart-rate") {{
  throw new Error("simulator did not chain random commands into the simulation effect");
}}
if (app.commands[0].command.min !== 0 || app.commands[0].command.max !== 2) throw new Error("zone random range was wrong");
if (app.commands[1].command.min !== 131 || app.commands[1].command.max !== 150) throw new Error("bpm random range was wrong");
if (app.commands[2].command.id !== "simulated-monitor" || app.commands[2].command.ms !== 1000) throw new Error("simulator command timing was wrong");
if (app.commands[2].command.start !== 136 || app.commands[2].command.min !== 111 || app.commands[2].command.max !== 150 || app.commands[2].command.jitter !== 3.5) {{
  throw new Error("simulator command bounds were wrong");
}}
if (app.commands[2].command.onReading !== Symbol.for("heart-rate")) throw new Error("simulator command did not route readings");
if (app.state["connected?"] !== true) throw new Error("simulator did not connect");
if (app.state.targetZoneId !== 3) throw new Error("simulator picked the wrong zone");
if (app.state.latestBpm !== 136) throw new Error("simulator rounded the first bpm incorrectly");
if (app.state.readings.length !== 1 || app.state.readings[0].bpm !== 136 || app.state.readings[0].time !== 0) {{
  throw new Error("simulator did not append the first reading");
}}
if (statusText.nodeValue !== "Simulated monitor") throw new Error("simulator status text did not update");
if (bpmText.nodeValue !== "136") throw new Error("simulator bpm text did not update");
if (section.attributes["data-connected"] !== "") throw new Error("connected attr was not set");
if (section.attributes["data-bpm"] !== "136") throw new Error("connected bpm attr was wrong");

const handle = [...intervals.keys()][0];
if (!handle || intervals.get(handle).ms !== 1000) throw new Error("simulator interval was not registered");
simulateButton.click();
if (app.commands.length !== 3) throw new Error("connected simulator click should not emit duplicate commands");
if (intervals.size !== 1) throw new Error("connected simulator click should not create a duplicate interval");

runInterval(handle);
if (app.commands.length !== 3) throw new Error("simulation tick should not emit extra command records");
if (app.state.latestBpm !== 139 || app.state.elapsedMs !== 1000) throw new Error("jitter did not update bpm and elapsed time");
if (app.state.readings.length !== 2 || app.state.readings[1].bpm !== 139 || app.state.readings[1].time !== 1000) {{
  throw new Error("jitter did not append the second reading");
}}
if (bpmText.nodeValue !== "139" || section.attributes["data-bpm"] !== "139") throw new Error("jitter did not update the DOM");

stopButton.click();
if (app.commands.length !== 4 || app.commands[3].kind !== "simulation/stop") throw new Error("stop did not cancel simulator");
if (cleared[0] !== handle) throw new Error("timer cancel did not clear the simulator interval");
if (app.state["connected?"] !== false) throw new Error("simulator did not disconnect");
if (statusText.nodeValue !== "Disconnected") throw new Error("disconnect status did not render");
if (section.hasAttribute("data-connected")) throw new Error("connected attr was not removed");

runInterval(handle);
if (app.state.readings.length !== 2) throw new Error("cleared simulator interval still dispatched");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated simulated monitor failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_workout_entry_id_uses_random_suffix() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_workout_entry_id.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-workout-entry-id-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
const expectedSuffix = (0.5).toString(36).slice(2, 9);
const expectedId = `1704499200000-${{expectedSuffix}}`;

if (mod.random_id_suffix(0.5) !== expectedSuffix) throw new Error("random id suffix was wrong");
if (mod.workout_entry_id(1704499200000, 0.5) !== expectedId) throw new Error("workout entry id was wrong");
if (mod.sample_entry_id !== expectedId) throw new Error("sample entry id was wrong");

let [stopping, timeCommand] = mod.update(mod.init, {{ kind: Symbol.for("stop-requested") }});
if (stopping.status !== "Stopping") throw new Error("stop request did not update status");
if (timeCommand.kind !== Symbol.for("time/now") || timeCommand.onSuccess !== Symbol.for("stopped-at")) {{
  throw new Error("stop request did not ask for time/now");
}}

let [pending, randomCommand] = mod.update(stopping, {{ kind: Symbol.for("stopped-at"), value: 1704499200000 }});
if (pending.pendingStoppedAt !== 1704499200000) throw new Error("stopped-at did not store pending timestamp");
if (randomCommand.kind !== Symbol.for("random/number") || randomCommand.min !== 0 || randomCommand.max !== 1 || randomCommand.onSuccess !== Symbol.for("id-roll")) {{
  throw new Error("stopped-at did not request random suffix roll");
}}

let [logged, noneCommand] = mod.update(pending, {{ kind: Symbol.for("id-roll"), value: 0.5 }});
if (noneCommand.kind !== Symbol.for("none")) throw new Error("id-roll should finish with Cmd.none");
if (logged.status !== "Logged" || logged.pendingStoppedAt !== null) throw new Error("id-roll did not clear pending stop state");
if (logged.entries.length !== 1) throw new Error("id-roll did not append a workout entry");
if (logged.entries[0].id !== expectedId || logged.selectedLogId !== expectedId) throw new Error("logged entry id was not selected");
if (logged.entries[0].startedAt !== mod.init.sessionStartedAt || logged.entries[0].durationMs !== mod.init.elapsedMs) {{
  throw new Error("logged entry timing fields were wrong");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated workout entry id module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_workout_lifecycle_uses_time_commands() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-workout-lifecycle-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_workout_lifecycle.clsk");
    let output = temp_dir.join("workout-lifecycle.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const intervals = new Map();
const cleared = [];
let nextHandle = 0;
const timers = {{
  setInterval(callback, ms) {{
    const handle = `interval-${{++nextHandle}}`;
    intervals.set(handle, {{ callback, ms, active: true }});
    return handle;
  }},
  clearInterval(handle) {{
    cleared.push(handle);
    const interval = intervals.get(handle);
    if (interval) interval.active = false;
  }},
  setTimeout(callback) {{
    callback();
    return "timeout";
  }}
}};

function runInterval(handle) {{
  const interval = intervals.get(handle);
  if (interval?.active) interval.callback();
}}

const timestamps = [1000, 1500, 1800, 2200, 3000, 3500];
function now() {{
  if (!timestamps.length) throw new Error("unexpected time/now command");
  return timestamps.shift();
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ timers, now }})
}});

const section = host.children[0];
const buttons = section.children.filter((node) => node.tagName === "button");
const [startButton, pauseButton, resumeButton, stopButton] = buttons;
const span = section.children.find((node) => node.tagName === "span");
const text = span.children.find((node) => "nodeValue" in node);

if (text.nodeValue !== "Idle") throw new Error("initial lifecycle label was wrong");
if (section.hasAttribute("data-running")) throw new Error("running attr should start absent");

startButton.click();
if (app.commands.map((entry) => entry.kind).join(",") !== "time/now,timer/every") {{
  throw new Error("start did not request time and then start the clock");
}}
if (Symbol.keyFor(app.state.exerciseState) !== "running") throw new Error("start did not enter running state");
if (app.state.startedAt !== 1000 || app.state.sessionStartedAt !== 1000) throw new Error("start timestamp was not applied");
if (text.nodeValue !== "Recording") throw new Error("start did not update status text");
if (section.attributes["data-running"] !== "") throw new Error("running attr was not set");

app.dispatch({{ kind: Symbol.for("heart-rate"), bpm: 142, receivedAt: 1250 }});
if (app.state.readings.length !== 1 || app.state.readings[0].time !== 250) throw new Error("heart-rate reading did not use elapsed time");

const firstHandle = [...intervals.keys()][0];
runInterval(firstHandle);
if (app.state.displayElapsedMs !== 500) throw new Error("clock tick did not update display elapsed time from time/now");
if (section.attributes["data-elapsed"] !== "500") throw new Error("elapsed attr was not updated");

pauseButton.click();
if (app.commands.length !== 5 || app.commands[4].kind !== "timer/cancel") throw new Error("pause did not cancel the clock after time/now");
if (cleared[0] !== firstHandle) throw new Error("pause did not clear the first timer handle");
if (app.state.elapsedMs !== 800 || Symbol.keyFor(app.state.exerciseState) !== "paused") throw new Error("pause timestamp was not applied");
if (section.hasAttribute("data-running")) throw new Error("running attr was not removed on pause");

resumeButton.click();
if (app.commands.length !== 7 || app.commands[6].kind !== "timer/every") throw new Error("resume did not restart the clock after time/now");
if (app.state.startedAt !== 2200 || Symbol.keyFor(app.state.exerciseState) !== "running") throw new Error("resume timestamp was not applied");
const secondHandle = [...intervals.keys()].find((handle) => handle !== firstHandle);
if (!secondHandle) throw new Error("resume did not register a second timer handle");

app.dispatch({{ kind: Symbol.for("heart-rate"), bpm: 150, receivedAt: 2500 }});
if (app.state.readings.length !== 2 || app.state.readings[1].time !== 1100) throw new Error("resumed reading did not include paused elapsed time");

stopButton.click();
if (app.commands.length !== 9 || app.commands[8].kind !== "timer/cancel") throw new Error("stop did not request time and cancel the clock");
if (cleared[1] !== secondHandle) throw new Error("stop did not clear the resumed timer handle");
if (Symbol.keyFor(app.state.exerciseState) !== "idle") throw new Error("stop did not return to idle");
if (app.state.entries.length !== 1) throw new Error("stop did not append a log entry");
const entry = app.state.entries[0];
if (entry.id !== "entry-3000") throw new Error("entry id did not use stopped timestamp");
if (entry.startedAt !== 1000 || entry.stoppedAt !== 3000 || entry.durationMs !== 1600) throw new Error("entry timing was wrong");
if (entry.readings.length !== 2 || entry.readings[1].bpm !== 150) throw new Error("entry readings were not preserved");
if (app.state.selectedLogId !== "entry-3000") throw new Error("selected log id was not set");
if (text.nodeValue !== "Done") throw new Error("stop did not update status text");

app.dispatch({{ kind: Symbol.for("delete-selected") }});
if (app.commands.length !== 10 || app.commands[9].kind !== "time/now") throw new Error("delete did not request time");
if (app.state.entries[0].hiddenAt !== 3500) throw new Error("delete timestamp was not applied");
if (app.state.selectedLogId !== null) throw new Error("delete should clear selected log when no visible entries remain");
if (timestamps.length !== 0) throw new Error("not all expected time values were consumed");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated workout lifecycle app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_media_query_app_tracks_breakpoint_changes() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-media-query-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_media_query_app.clsk");
    let output = temp_dir.join("media-query-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

class MediaQueryList {{
  constructor(media, matches) {{
    this.media = media;
    this.matches = matches;
    this.listeners = [];
  }}
  addEventListener(name, listener) {{
    if (name === "change") this.listeners.push(listener);
  }}
  removeEventListener(name, listener) {{
    if (name !== "change") return;
    this.listeners = this.listeners.filter((item) => item !== listener);
  }}
  setMatches(matches) {{
    this.matches = matches;
    for (const listener of [...this.listeners]) listener({{ media: this.media, matches }});
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const queries = new Map();
function matchMedia(query) {{
  if (!queries.has(query)) queries.set(query, new MediaQueryList(query, true));
  return queries.get(query);
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ matchMedia }})
}});

const section = host.children[0];
const label = section.children.find((node) => node.tagName === "strong");
const text = label.children.find((node) => "nodeValue" in node);
const button = section.children.find((node) => node.tagName === "button");
const query = queries.get("(max-width: 700px)");

if (app.commands.length !== 1 || app.commands[0].kind !== "media-query/watch") throw new Error("media query watch command was not logged");
if (query.listeners.length !== 1) throw new Error("media query listener was not registered");
if (app.state["mobile?"] !== true) throw new Error("initial media query state was not dispatched");
if (section.attributes["data-mobile"] !== "") throw new Error("mobile attr was not set");
if (text.nodeValue !== "Mobile") throw new Error("mobile label was wrong");

query.setMatches(false);
if (app.state["mobile?"] !== false) throw new Error("media query change did not update state");
if (section.hasAttribute("data-mobile")) throw new Error("mobile attr was not removed");
if (text.nodeValue !== "Desktop") throw new Error("desktop label was wrong");

button.click();
if (app.commands.length !== 2 || app.commands[1].kind !== "media-query/unwatch") throw new Error("media query unwatch command was not logged");
if (query.listeners.length !== 0) throw new Error("media query listener was not removed");
if (app.state["watching?"] !== false) throw new Error("stop did not update watching state");
if (section.hasAttribute("data-watching")) throw new Error("watching attr was not removed");

query.setMatches(true);
if (app.state["mobile?"] !== false) throw new Error("unwatched media query still dispatched changes");
if (text.nodeValue !== "Desktop") throw new Error("label changed after unwatch");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated media query app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_runtime_dispose_cleans_command_resources() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-runtime-dispose-cleanup-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let source = temp_dir.join("cleanup.clsk");
    let output = temp_dir.join("cleanup.mjs");
    fs::write(
        &source,
        "(defn init []\n\
           [{:status \"Mounted\"}\n\
            {:kind :batch\n\
            :commands [{:kind :timer/every\n\
                        :id \"clock\"\n\
                        :ms 250\n\
                        :msg :tick}\n\
                       {:kind :timer/after\n\
                        :ms 500\n\
                        :msg :late}\n\
                       {:kind :animation/frame\n\
                        :id \"paint\"\n\
                        :onFrame :painted}\n\
                        {:kind :media-query/watch\n\
                         :id \"mobile\"\n\
                         :query \"(max-width: 700px)\"\n\
                         :onChange :media}\n\
                        {:kind :window/event-watch\n\
                         :id \"keyboard\"\n\
                         :type \"keydown\"\n\
                         :onEvent :key}\n\
                        {:kind :dom-ref/resize-watch\n\
                         :id \"panel\"\n\
                         :ref \"panel\"\n\
                         :onChange :resized}]}])\n\
         (defn update [state msg]\n\
           [state {:kind :none}])\n\
         (defn view [state]\n\
           #html <section ref=\"panel\"><span>{state.status}</span></section>)\n",
    )
    .expect("cleanup source should be written");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&source)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

class MediaQueryList {{
  constructor(media) {{
    this.media = media;
    this.matches = true;
    this.listeners = [];
  }}
  addEventListener(name, listener) {{
    if (name === "change") this.listeners.push(listener);
  }}
  removeEventListener(name, listener) {{
    if (name === "change") this.listeners = this.listeners.filter((item) => item !== listener);
  }}
}}

const observers = [];
class FakeResizeObserver {{
  constructor(callback) {{
    this.callback = callback;
    this.nodes = [];
    this.disconnected = false;
    observers.push(this);
  }}
  observe(node) {{
    this.nodes.push(node);
  }}
  disconnect() {{
    this.disconnected = true;
    this.nodes = [];
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const intervals = new Map();
const timeouts = new Map();
const timers = {{
  setInterval(callback, ms) {{
    const handle = {{ kind: "interval", id: intervals.size + 1 }};
    intervals.set(handle, {{ callback, ms }});
    return handle;
  }},
  clearInterval(handle) {{
    intervals.delete(handle);
  }},
  setTimeout(callback, ms) {{
    const handle = {{ kind: "timeout", id: timeouts.size + 1 }};
    timeouts.set(handle, {{ callback, ms }});
    return handle;
  }},
  clearTimeout(handle) {{
    timeouts.delete(handle);
  }}
}};

const frames = new Map();
const animation = {{
  requestAnimationFrame(callback) {{
    const handle = {{ kind: "frame", id: frames.size + 1 }};
    frames.set(handle, callback);
    return handle;
  }},
  cancelAnimationFrame(handle) {{
    frames.delete(handle);
  }}
}};

const queries = new Map();
function matchMedia(query) {{
  if (!queries.has(query)) queries.set(query, new MediaQueryList(query));
  return queries.get(query);
}}

const eventTarget = {{
  listeners: {{}},
  addEventListener(type, listener, options) {{
    this.listeners[type] ||= [];
    this.listeners[type].push({{ listener, options }});
  }},
  removeEventListener(type, listener) {{
    this.listeners[type] = (this.listeners[type] || []).filter((entry) => entry.listener !== listener);
  }},
  count(type) {{
    return (this.listeners[type] || []).length;
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const handlers = runtime.createCommandHandlers({{
  timers,
  animation,
  matchMedia,
  ResizeObserver: FakeResizeObserver,
  eventTarget
}});
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers
}});

const query = queries.get("(max-width: 700px)");
if (!query || query.listeners.length !== 1) throw new Error("media query listener was not registered");
if (intervals.size !== 1) throw new Error("timer interval was not registered");
if (timeouts.size !== 1) throw new Error("timer timeout was not registered");
if (frames.size !== 1) throw new Error("animation frame was not registered");
if (eventTarget.count("keydown") !== 1) throw new Error("window event listener was not registered");
if (observers.length !== 1 || observers[0].disconnected) throw new Error("resize observer was not registered");
if (host.children.length !== 1 || app.getRef("panel") !== host.children[0]) throw new Error("panel ref was not mounted");

app.dispose();

if (intervals.size !== 0) throw new Error("timer interval survived app disposal");
if (timeouts.size !== 0) throw new Error("timer timeout survived app disposal");
if (frames.size !== 0) throw new Error("animation frame survived app disposal");
if (query.listeners.length !== 0) throw new Error("media query listener survived app disposal");
if (eventTarget.count("keydown") !== 0) throw new Error("window event listener survived app disposal");
if (!observers[0].disconnected || observers[0].nodes.length !== 0) throw new Error("resize observer survived app disposal");
if (host.children.length !== 0) throw new Error("component root survived app disposal");
if (app.getRef("panel") !== undefined) throw new Error("panel ref survived app disposal");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated runtime cleanup app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_dev_hotkey_app_gates_window_listener_by_env() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-dev-hotkey-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_dev_hotkey_app.clsk");
    let output = temp_dir.join("dev-hotkey.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.clicks = 0;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(type, listener, options) {{
    this.listeners[type] ||= [];
    this.listeners[type].push({{ listener, options }});
  }}
  removeEventListener(type, listener) {{
    this.listeners[type] = (this.listeners[type] || []).filter((entry) => entry.listener !== listener);
  }}
  emit(type, event = {{}}) {{
    for (const entry of [...(this.listeners[type] || [])]) {{
      entry.listener({{ type, currentTarget: this, target: this, ...event }});
      if (entry.options?.once) this.removeEventListener(type, entry.listener);
    }}
  }}
  click() {{
    this.clicks += 1;
    this.emit("click", {{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function createEventTarget() {{
  return {{
    listeners: {{}},
    addEventListener(type, listener, options) {{
      this.listeners[type] ||= [];
      this.listeners[type].push({{ listener, options }});
    }},
    removeEventListener(type, listener) {{
      this.listeners[type] = (this.listeners[type] || []).filter((entry) => entry.listener !== listener);
    }},
    emit(type, event = {{}}) {{
      const emitted = {{
        type,
        defaultPrevented: false,
        propagationStopped: false,
        preventDefault() {{
          this.defaultPrevented = true;
        }},
        stopPropagation() {{
          this.propagationStopped = true;
        }},
        ...event
      }};
      for (const entry of [...(this.listeners[type] || [])]) {{
        entry.listener(emitted);
        if (entry.options?.once) this.removeEventListener(type, entry.listener);
      }}
      return emitted;
    }},
    count(type) {{
      return (this.listeners[type] || []).length;
    }}
  }};
}}

function descendants(node, tagName) {{
  const matches = [];
  for (const child of node.children || []) {{
    if (child.tagName === tagName) matches.push(child);
    matches.push(...descendants(child, tagName));
  }}
  return matches;
}}

function textOf(node) {{
  return (node.children || []).map((child) => "nodeValue" in child ? child.nodeValue : textOf(child)).join("");
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

function startWithDev(dev, eventTarget) {{
  globalThis.__CLOSKELL_ENV__ = {{ DEV: dev }};
  const host = new Element("main");
  const handlers = runtime.createCommandHandlers({{ eventTarget }});
  const app = runtime.startApp({{
    root: host,
    init: mod.init,
    update: mod.update,
    view: mod.view,
    handlers
  }});
  return {{ app, host }};
}}

const prodEvents = createEventTarget();
const prod = startWithDev(false, prodEvents);
const prodSection = prod.host.children[0];
const prodStatus = descendants(prodSection, "span")[0];
if (prod.app.state["dev?"] !== false) throw new Error("production init did not read env-dev? as false");
if (prod.app.commands.length !== 0) throw new Error("production init should not emit a window watcher");
if (prodEvents.count("keydown") !== 0) throw new Error("production init registered a dev hotkey listener");
if (prodSection.hasAttribute("data-dev")) throw new Error("production data-dev attr should be absent");
if (prodSection.attributes["data-opens"] !== "0" || textOf(prodStatus) !== "Production") {{
  throw new Error("production view did not render initial state");
}}
prod.app.dispose();

const devEvents = createEventTarget();
const dev = startWithDev(true, devEvents);
const devSection = dev.host.children[0];
const devButton = descendants(devSection, "button")[0];
const devStatus = descendants(devSection, "span")[0];
if (dev.app.state["dev?"] !== true) throw new Error("dev init did not read env-dev? as true");
if (dev.app.commands.length !== 1 || dev.app.commands[0].kind !== "window/event-watch") {{
  throw new Error("dev init should log one window/event-watch command");
}}
if (dev.app.commands[0].command.preventDefault.key !== "h" || dev.app.commands[0].command.preventDefault.ctrlKey !== true) {{
  throw new Error("dev hotkey preventDefault guard was not emitted");
}}
if (devEvents.count("keydown") !== 1) throw new Error("dev hotkey listener was not registered");
if (!devSection.hasAttribute("data-dev") || devSection.attributes["data-opens"] !== "0") {{
  throw new Error("dev view did not render initial attrs");
}}
if (textOf(devStatus) !== "Dev tools ready") throw new Error("dev status text was wrong");

const ignoredKey = devEvents.emit("keydown", {{ key: "A", ctrlKey: true, shiftKey: true }});
if (ignoredKey.defaultPrevented) throw new Error("ignored dev key should not prevent default");
if (dev.app.commands.length !== 1) throw new Error("ignored key should not emit a command");
if (dev.app.state.lastKey !== "A" || textOf(devStatus) !== "Ignored key") {{
  throw new Error("ignored key did not update status");
}}

const altKey = devEvents.emit("keydown", {{ key: "h", ctrlKey: true, shiftKey: true, altKey: true }});
if (altKey.defaultPrevented) throw new Error("alt-modified dev key should not prevent default");
if (dev.app.commands.length !== 1) throw new Error("alt-modified key should not emit a command");
if (dev.app.state.lastKey !== "h" || textOf(devStatus) !== "Ignored key") {{
  throw new Error("alt-modified key did not update ignored status");
}}

const hotkey = devEvents.emit("keydown", {{ key: "H", ctrlKey: true, shiftKey: true }});
if (!hotkey.defaultPrevented) throw new Error("matching dev hotkey should prevent default");
if (dev.app.commands.length !== 2 || dev.app.commands[1].kind !== "dom-ref/click") {{
  throw new Error("dev hotkey should emit a dom-ref/click command");
}}
if (devButton.clicks !== 1) throw new Error("dev hotkey did not click the simulator button");
if (dev.app.state.opens !== 1 || devSection.attributes["data-opens"] !== "1") {{
  throw new Error("dev hotkey did not record the simulator open");
}}
if (textOf(devStatus) !== "Simulator opened") throw new Error("dev-opened message did not update status");
if (dev.host.children[0] !== devSection) throw new Error("section root was replaced after dev hotkey");
if (descendants(devSection, "button")[0] !== devButton) throw new Error("simulator button was replaced after dev hotkey");

dev.app.dispose();
if (devEvents.count("keydown") !== 0) throw new Error("dev hotkey listener survived app disposal");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated dev hotkey app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_bluetooth_app_requests_device_and_handles_result() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-bluetooth-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_bluetooth_app.clsk");
    let output = temp_dir.join("bluetooth-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const requests = [];
const bluetooth = {{
  async requestDevice(options) {{
    requests.push(options);
    return {{ name: "Polar H10", id: "polar" }};
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

const unavailableHandlers = runtime.createCommandHandlers({{ bluetooth: null, host: {{}} }});
const unavailableMessage = await unavailableHandlers["bluetooth/request-device"]({{
  kind: Symbol.for("bluetooth/request-device"),
  filters: [{{ services: ["heart_rate"] }}],
  onSuccess: Symbol.for("connected"),
  onError: Symbol.for("bluetooth-error")
}});
if (unavailableMessage.kind !== Symbol.for("bluetooth-error")) throw new Error("unavailable Bluetooth did not route to onError");
if (!unavailableMessage.error.includes("Web Bluetooth unavailable")) throw new Error("Bluetooth unavailable error message was wrong");

const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ bluetooth }})
}});

const button = host.children[0];
const text = button.children[0];
if (text.nodeValue !== "Idle") throw new Error("initial Bluetooth label was wrong");
if (button.hasAttribute("data-connected")) throw new Error("connected attr should start absent");

button.click();
if (app.state.status !== "Pairing") throw new Error("connect click did not set pairing state");
if (text.nodeValue !== "Pairing") throw new Error("pairing label was not rendered");
if (app.commands.length !== 1 || app.commands[0].kind !== "bluetooth/request-device") throw new Error("Bluetooth command was not logged");
if (requests.length !== 1) throw new Error("Bluetooth adapter was not called");
if (requests[0].filters[0].services[0] !== "heart_rate") throw new Error("Bluetooth filters were not passed");
if (requests[0].optionalServices[0] !== "heart_rate") throw new Error("Bluetooth optional services were not passed");

await new Promise((resolve) => setTimeout(resolve, 0));
if (host.children[0] !== button) throw new Error("button was replaced after Bluetooth success");
if (button.children[0] !== text) throw new Error("text node was replaced after Bluetooth success");
if (app.state["connected?"] !== true) throw new Error("Bluetooth success did not set connected state");
if (app.state.deviceName !== "Polar H10") throw new Error("Bluetooth device name was not stored");
if (text.nodeValue !== "Live") throw new Error("Bluetooth success did not update status label");
if (button.attributes["data-connected"] !== "") throw new Error("connected attr was not set");
if (button.attributes["data-device"] !== "Polar H10") throw new Error("device attr was not updated");
if (button.attributes["data-message"] !== "Connected.") throw new Error("message attr was not updated");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated bluetooth app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_heart_rate_app_streams_notifications_and_disconnects() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-heart-rate-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_heart_rate_app.clsk");
    let output = temp_dir.join("heart-rate-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

class Characteristic {{
  constructor() {{
    this.listeners = {{}};
    this.started = false;
    this.stopped = false;
  }}
  async startNotifications() {{
    this.started = true;
    return this;
  }}
  async stopNotifications() {{
    this.stopped = true;
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((item) => item !== listener);
  }}
  emit(bytes) {{
    const view = new DataView(Uint8Array.from(bytes).buffer);
    for (const listener of this.listeners.characteristicvaluechanged || []) {{
      listener({{ target: {{ value: view }} }});
    }}
  }}
}}

class Device {{
  constructor(characteristic) {{
    this.name = "Polar H10";
    this.listeners = {{}};
    this.disconnected = false;
    this.gatt = {{
      connected: false,
      connect: async () => {{
        this.gatt.connected = true;
        return {{
          getPrimaryService: async (service) => {{
            services.push(service);
            return {{
              getCharacteristic: async (name) => {{
                characteristics.push(name);
                return characteristic;
              }}
            }};
          }}
        }};
      }},
      disconnect: () => {{
        this.disconnected = true;
        this.gatt.connected = false;
      }}
    }};
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((item) => item !== listener);
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const requests = [];
const services = [];
const characteristics = [];
const characteristic = new Characteristic();
const device = new Device(characteristic);
const bluetooth = {{
  async requestDevice(options) {{
    requests.push(options);
    return device;
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ bluetooth }})
}});

const section = host.children[0];
const buttons = section.children.filter((node) => node.tagName === "button");
const connectButton = buttons[0];
const disconnectButton = buttons[1];
const span = section.children.find((node) => node.tagName === "span");
const text = span.children.find((node) => "nodeValue" in node);

if (text.nodeValue !== "Idle") throw new Error("initial heart-rate status was wrong");
if (section.hasAttribute("data-connected")) throw new Error("connected attr should start absent");

connectButton.click();
if (app.state.status !== "Pairing") throw new Error("connect click did not enter pairing state");
if (text.nodeValue !== "Pairing") throw new Error("pairing state did not render");
await new Promise((resolve) => setTimeout(resolve, 0));

if (app.commands.length !== 1 || app.commands[0].kind !== "bluetooth/connect-heart-rate") throw new Error("connect-heart-rate command was not logged");
if (requests[0].filters[0].services[0] !== "heart_rate") throw new Error("heart rate filter was not passed");
if (services[0] !== "heart_rate") throw new Error("heart rate service was not requested");
if (characteristics[0] !== "heart_rate_measurement") throw new Error("heart rate characteristic was not requested");
if (!characteristic.started) throw new Error("notifications were not started");
if (app.state["connected?"] !== true) throw new Error("connected message did not update state");
if (app.state.deviceName !== "Polar H10") throw new Error("device name was not stored");
if (text.nodeValue !== "Live") throw new Error("connected state did not render");
if (section.attributes["data-connected"] !== "") throw new Error("connected attr was not set");

characteristic.emit([0, 142]);
if (app.state.latest !== 142) throw new Error("8-bit heart-rate notification did not update state");
if (section.attributes["data-latest"] !== "142") throw new Error("latest attr was not updated for 8-bit bpm");
if (section.attributes["data-message"] !== "142") throw new Error("message attr was not updated for 8-bit bpm");

characteristic.emit([1, 44, 1]);
if (app.state.latest !== 300) throw new Error("16-bit heart-rate notification did not update state");
if (section.attributes["data-latest"] !== "300") throw new Error("latest attr was not updated for 16-bit bpm");

disconnectButton.click();
if (app.commands.length !== 2 || app.commands[1].kind !== "bluetooth/disconnect") throw new Error("disconnect command was not logged");
await new Promise((resolve) => setTimeout(resolve, 0));
if (!characteristic.stopped) throw new Error("notifications were not stopped");
if ((characteristic.listeners.characteristicvaluechanged || []).length !== 0) throw new Error("notification listener was not removed");
if (!device.disconnected) throw new Error("device was not disconnected");
if (app.state["connected?"] !== false) throw new Error("disconnect message did not clear connected state");
if (text.nodeValue !== "Disconnected") throw new Error("disconnect state did not render");
if (section.hasAttribute("data-connected")) throw new Error("connected attr was not removed");

if (host.children[0] !== section) throw new Error("section node was replaced during heart-rate flow");
if (span.children.find((node) => "nodeValue" in node) !== text) throw new Error("text node was replaced during heart-rate flow");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated heart-rate app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_export_app_routes_file_download_command() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-export-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_export_app.clsk");
    let output = temp_dir.join("export-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const downloads = [];
const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

const helperDownload = runtime.Cmd.fileDownload("helper.json", "{{}}", "application/json", Symbol.for("downloaded"), Symbol.for("download-failed"));
if (helperDownload.kind !== Symbol.for("file/download") || helperDownload.name !== "helper.json") throw new Error("Cmd.fileDownload helper emitted the wrong command shape");
if (helperDownload.content !== "{{}}" || helperDownload.mime !== "application/json") throw new Error("Cmd.fileDownload helper did not preserve payload fields");
if (helperDownload.msg !== Symbol.for("downloaded") || helperDownload.onError !== Symbol.for("download-failed")) throw new Error("Cmd.fileDownload helper did not preserve success and error continuations");

const browserDownloads = [];
let revokedHref = null;
class FakeBlob {{
  constructor(parts, options) {{
    this.parts = parts;
    this.type = options.type;
    this.size = parts.join("").length;
  }}
}}
const browserDocument = {{
  body: new Element("body"),
  createElement(tagName) {{
    const node = new Element(tagName);
    if (tagName === "a") {{
      node.click = () => browserDownloads.push({{ href: node.href, download: node.download, parent: node.parentNode }});
    }}
    return node;
  }}
}};
const browserURL = {{
  createObjectURL(blob) {{
    browserDownloads.push({{ blob }});
    return "blob:closkell-export";
  }},
  revokeObjectURL(href) {{
    revokedHref = href;
  }}
}};
const browserHandlers = runtime.createCommandHandlers({{ document: browserDocument, URL: browserURL, Blob: FakeBlob }});
const browserMessage = browserHandlers["file/download"]({{
  kind: Symbol.for("file/download"),
  name: "browser.json",
  content: "{{}}",
  mime: "application/json",
  msg: Symbol.for("downloaded")
}});
if (browserMessage !== Symbol.for("downloaded")) throw new Error("default file/download handler did not return command msg");
if (browserDownloads[0].blob.type !== "application/json") throw new Error("default file/download blob mime was wrong");
if (browserDownloads[1].href !== "blob:closkell-export") throw new Error("default file/download did not click generated link");
if (browserDownloads[1].download !== "browser.json") throw new Error("default file/download name was wrong");
if (revokedHref !== "blob:closkell-export") throw new Error("default file/download did not revoke object URL");

const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{
    download(payload) {{
      downloads.push(payload);
      return {{ accepted: true, name: payload.name }};
    }}
  }})
}});

const button = host.children[0];
const text = button.children[0];
if (text.nodeValue !== "Export") throw new Error("initial export label was wrong");
if (button.hasAttribute("data-downloaded")) throw new Error("downloaded attr should start absent");

button.click();

if (host.children[0] !== button) throw new Error("button was replaced after export update");
if (button.children[0] !== text) throw new Error("text node was replaced after export update");
if (downloads.length !== 1) throw new Error(`expected one download, found ${{downloads.length}}`);
if (downloads[0].name !== "exercise-log.json") throw new Error("download name was wrong");
if (downloads[0].mime !== "application/json") throw new Error("download mime was wrong");
if (downloads[0].content !== '{{"version":2,"entries":[]}}') throw new Error("download content was wrong");
if (app.commands.length !== 1 || app.commands[0].kind !== "file/download") throw new Error("file download command was not logged");
if (app.state["downloaded?"] !== true) throw new Error("export completion message did not update state");
if (text.nodeValue !== "Exported") throw new Error("export completion did not update label");
if (button.attributes["data-downloaded"] !== "") throw new Error("downloaded attr was not updated");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated export app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_import_app_routes_file_import_command() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-import-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_import_app.clsk");
    let output = temp_dir.join("import-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = {{}};
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

const browserDocument = {{
  body: new Element("body"),
  createElement(tagName) {{
    const node = new Element(tagName);
    if (tagName === "input") {{
      node.click = () => {{
        node.files = [{{
          name: "exercise-log.json",
          type: "application/json",
          text: async () => '{{"version":2,"entries":[{{"id":"browser"}}]}}'
        }}];
        for (const listener of node.listeners.change || []) listener({{ target: node }});
      }};
    }}
    return node;
  }}
}};
const browserHandlers = runtime.createCommandHandlers({{ document: browserDocument }});
const browserMessage = await browserHandlers["file/import"]({{
  kind: Symbol.for("file/import"),
  accept: "application/json,.json",
  format: Symbol.for("json"),
  onSuccess: Symbol.for("imported"),
  onError: Symbol.for("failed")
}});
if (browserMessage.kind !== Symbol.for("imported")) throw new Error("default file/import did not return success message");
if (browserMessage.value.entries[0].id !== "browser") throw new Error("default file/import did not parse JSON");
if (browserDocument.body.children.length !== 0) throw new Error("default file/import did not remove hidden input");

const errorHandlers = runtime.createCommandHandlers({{
  importFile() {{
    throw new Error("bad import");
  }}
}});
const errorMessage = await errorHandlers["file/import"]({{
  kind: Symbol.for("file/import"),
  onSuccess: Symbol.for("imported"),
  onError: Symbol.for("import-failed")
}});
if (errorMessage.kind !== Symbol.for("import-failed") || errorMessage.error !== "bad import") {{
  throw new Error("file/import did not route errors through onError");
}}

const imports = [];
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{
    importFile(payload) {{
      imports.push(payload);
      return {{ version: 2, entries: [{{ id: "a" }}, {{ id: "b" }}] }};
    }}
  }})
}});

const button = host.children[0];
const text = button.children[0];
if (text.nodeValue !== "Import") throw new Error("initial import label was wrong");
if (button.hasAttribute("data-importing")) throw new Error("importing attr should start absent");

button.click();
await new Promise((resolve) => setTimeout(resolve, 0));

if (host.children[0] !== button) throw new Error("button was replaced after import");
if (button.children[0] !== text) throw new Error("text node was replaced after import");
if (imports.length !== 1) throw new Error(`expected one file import, found ${{imports.length}}`);
if (imports[0].accept !== "application/json,.json") throw new Error("file import accept was wrong");
if (imports[0].format !== "json") throw new Error("file import format was wrong");
if (app.commands.length !== 1 || app.commands[0].kind !== "file/import") throw new Error("file import command was not logged");
if (app.state.entries.length !== 2 || app.state.entries[1].id !== "b") throw new Error("imported entries did not update state");
if (text.nodeValue !== "Imported 2") throw new Error(`import label did not update: ${{text.nodeValue}}`);
if (button.hasAttribute("data-importing")) throw new Error("importing attr should be removed after import");
if (button.attributes["data-message"] !== "Import complete") throw new Error("import message attr was wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated import app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_import_trigger_clicks_registered_file_ref() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-import-trigger-app-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_import_trigger_app.clsk");
    let output = temp_dir.join("import-trigger-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.value = "";
    this.files = [];
    this.clickCount = 0;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "value") this.value = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "value") this.value = "";
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  click() {{
    this.clickCount += 1;
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
  selectFiles(files) {{
    this.files = files;
    this.value = files[0]?.name || "";
    this.emit("change", {{ type: "change", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children.find((node) => node.tagName === "section");
const button = section.children.find((node) => node.tagName === "button");
const input = section.children.find((node) => node.tagName === "input");
const paragraph = section.children.find((node) => node.tagName === "p");
const list = section.children.find((node) => node.tagName === "ul");
const text = paragraph.children.find((node) => "nodeValue" in node && node.nodeValue.trim());

if (app.getRef("import-file") !== input) throw new Error("import file ref was not registered");
if (input.attributes.ref !== undefined) throw new Error("ref should not be emitted as an attribute");
if (input.attributes.class !== "hidden") throw new Error("hidden input class was wrong");
if (input.attributes.type !== "file") throw new Error("input type was wrong");
if (input.attributes.accept !== "application/json,.json") throw new Error("input accept was wrong");
if (input.attributes.value !== undefined) throw new Error("file input should not be controlled by a value attr");
if (section.attributes["data-status"] !== "Ready") throw new Error("initial import status attr was wrong");
if (section.attributes["data-clicks"] !== "0") throw new Error("initial click count attr was wrong");
if (section.attributes["data-last-ref"] !== "") throw new Error("initial last ref attr was wrong");
if (section.attributes["data-entry-count"] !== "0") throw new Error("initial entry count attr was wrong");
if (text.nodeValue !== "Ready") throw new Error("initial import status text was wrong");
if (listItems().length !== 0) throw new Error("import list should start empty");

button.click();

if (host.children[0] !== section) throw new Error("import trigger section was replaced");
if (section.children.find((node) => node.tagName === "button") !== button) throw new Error("import button was replaced");
if (section.children.find((node) => node.tagName === "input") !== input) throw new Error("hidden import input was replaced");
if (section.children.find((node) => node.tagName === "p") !== paragraph) throw new Error("status paragraph was replaced");
if (paragraph.children.find((node) => "nodeValue" in node && node.nodeValue.trim()) !== text) throw new Error("status text node was replaced");
if (input.clickCount !== 1) throw new Error(`hidden input click count was ${{input.clickCount}}`);
if (app.commands.length !== 1 || app.commands[0].kind !== "dom-ref/click") throw new Error("dom-ref/click command was not logged");
if (app.commands[0].command.ref !== "import-file") throw new Error("dom-ref/click command ref was wrong");
if (app.state.status !== "Waiting for file") throw new Error("import-opened message did not update status");
if (app.state.clicks !== 1) throw new Error("import-opened message did not increment clicks");
if (app.state.lastRef !== "import-file") throw new Error("import-opened message did not store ref");
if (section.attributes["data-status"] !== "Waiting for file") throw new Error("opened status attr was wrong");
if (section.attributes["data-clicks"] !== "1") throw new Error("opened click count attr was wrong");
if (section.attributes["data-last-ref"] !== "import-file") throw new Error("opened last ref attr was wrong");
if (text.nodeValue !== "Waiting for file") throw new Error("opened status text was wrong");

input.selectFiles([{{
  name: "exercise-log.json",
  type: "application/json",
  text: async () => '{{"entries":[{{"id":"warmup"}},{{"id":"intervals"}}]}}'
}}]);
if (app.state.status !== "Reading") throw new Error("file input change did not enter reading state");
if (app.commands.length !== 2 || app.commands[1].kind !== "file/read-selected") throw new Error("file/read-selected command was not logged");
if (app.commands[1].command.ref !== "import-file") throw new Error("file/read-selected command ref was wrong");
if (app.commands[1].command.format !== Symbol.for("json")) throw new Error("file/read-selected command format was wrong");
if (section.attributes["data-status"] !== "Reading") throw new Error("reading status attr was wrong");
if (text.nodeValue !== "Reading") throw new Error("reading status text was wrong");

await new Promise((resolve) => setTimeout(resolve, 0));

if (host.children[0] !== section) throw new Error("section was replaced after selected file import");
if (section.children.find((node) => node.tagName === "button") !== button) throw new Error("button was replaced after selected file import");
if (section.children.find((node) => node.tagName === "input") !== input) throw new Error("input was replaced after selected file import");
if (paragraph.children.find((node) => "nodeValue" in node && node.nodeValue.trim()) !== text) throw new Error("status text was replaced after selected file import");
if (app.state.status !== "Imported 2") throw new Error("selected file import did not update status");
if (app.state.entries.length !== 2 || app.state.entries[1].id !== "intervals") throw new Error("selected file import did not store entries");
if (section.attributes["data-status"] !== "Imported 2") throw new Error("imported status attr was wrong");
if (section.attributes["data-entry-count"] !== "2") throw new Error("imported entry count attr was wrong");
if (text.nodeValue !== "Imported 2") throw new Error("imported status text was wrong");
if (input.value !== "") throw new Error("selected file input value was not cleared");
if (input.files.length !== 0) throw new Error("selected file input files were not cleared");
const rows = listItems();
if (rows.length !== 2) throw new Error(`expected 2 imported rows, found ${{rows.length}}`);
if (rowText(rows[0]) !== "warmup" || rowText(rows[1]) !== "intervals") throw new Error("imported row labels were wrong");
if (app.commands.length !== 2) throw new Error("selected file import completion should only emit Cmd.none");
if (input.clickCount !== 1) throw new Error("file input change should not click the input again");

function listItems() {{
  return list.children.filter((node) => node.tagName === "li");
}}

function rowText(row) {{
  return row.children.find((node) => "nodeValue" in node && node.nodeValue.trim()).nodeValue;
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated import trigger app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_http_app_routes_fetch_results() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-http-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_http_app.clsk");
    let output = temp_dir.join("http-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

const missingFetchMessage = await runtime.createCommandHandlers({{ host: {{}} }})["http/request"]({{
  kind: Symbol.for("http/request"),
  url: "/api/exercise-log",
  onSuccess: Symbol.for("loaded"),
  onError: Symbol.for("load-failed")
}});
if (missingFetchMessage.kind !== Symbol.for("load-failed") || !missingFetchMessage.error.includes("No fetch")) {{
  throw new Error("missing fetch did not route through onError");
}}

const thrownFetchMessage = await runtime.createCommandHandlers({{
  fetch() {{
    throw new Error("network down");
  }}
}})["http/request"]({{
  kind: Symbol.for("http/request"),
  request: {{ url: "/api/exercise-log", method: "GET" }},
  onSuccess: Symbol.for("loaded"),
  onError: Symbol.for("load-failed")
}});
if (thrownFetchMessage.kind !== Symbol.for("load-failed") || thrownFetchMessage.error !== "network down") {{
  throw new Error("fetch throw did not route through onError");
}}

const topLevelRequests = [];
const topLevelMessage = await runtime.createCommandHandlers({{
  async fetch(url, options) {{
    topLevelRequests.push({{ url, options }});
    return {{
      status: 201,
      ok: true,
      async text() {{
        return "created";
      }}
    }};
  }}
}})["http/request"]({{
  kind: Symbol.for("http/request"),
  url: "/api/exercise-log",
  method: "POST",
  headers: {{ "content-type": "application/json" }},
  body: "{{\"id\":\"top-level\"}}",
  response: Symbol.for("text"),
  onSuccess: Symbol.for("created"),
  onError: Symbol.for("load-failed")
}});
if (topLevelRequests.length !== 1 || topLevelRequests[0].url !== "/api/exercise-log") {{
  throw new Error("top-level HTTP URL was not passed to fetch");
}}
if (topLevelRequests[0].options.method !== "POST" || topLevelRequests[0].options.body !== "{{\"id\":\"top-level\"}}") {{
  throw new Error("top-level HTTP init options were not passed to fetch");
}}
if (topLevelRequests[0].options.headers["content-type"] !== "application/json") {{
  throw new Error("top-level HTTP headers were not passed to fetch");
}}
if (topLevelMessage.kind !== Symbol.for("created") || topLevelMessage.value.status !== 201 || topLevelMessage.value.body !== "created") {{
  throw new Error("top-level HTTP success payload was wrong");
}}

const selectedFile = {{ name: "avatar.png", size: 42 }};
const multipartFile = {{ name: "report.pdf", size: 99 }};
class FakeFormData {{
  constructor() {{
    this.entries = [];
  }}
  append(name, value, filename) {{
    this.entries.push({{ name, value, filename }});
  }}
}}

const descriptorRequests = [];
const descriptorDocument = {{
  querySelector(selector) {{
    if (selector === "[data-testid=\"request-body-file\"]") return {{ files: [selectedFile] }};
    if (selector === "[data-testid=\"request-body-multipart-attachment\"]") return {{ files: [multipartFile] }};
    return null;
  }}
}};
const descriptorHandlers = runtime.createCommandHandlers({{
  document: descriptorDocument,
  FormData: FakeFormData,
  async fetch(url, options) {{
    descriptorRequests.push({{ url, options }});
    return {{
      status: 200,
      ok: true,
      async text() {{
        return "ok";
      }}
    }};
  }}
}});

const selectedFileMessage = await descriptorHandlers["http/request"]({{
  kind: Symbol.for("http/request"),
  request: {{
    url: "/upload/file",
    method: "POST",
    body: {{ kind: Symbol.for("browser/selected-file"), testId: "request-body-file" }}
  }},
  response: Symbol.for("text"),
  onSuccess: Symbol.for("uploaded"),
  onError: Symbol.for("upload-failed")
}});
if (selectedFileMessage.kind !== Symbol.for("uploaded")) throw new Error("selected-file descriptor did not succeed");
if (descriptorRequests.length !== 1 || descriptorRequests[0].options.body !== selectedFile) {{
  throw new Error("selected-file descriptor did not resolve to the selected file");
}}

const multipartMessage = await descriptorHandlers["http/request"]({{
  kind: Symbol.for("http/request"),
  request: {{
    url: "/upload/form",
    method: "POST",
    body: {{
      kind: Symbol.for("browser/multipart-form"),
      fields: [
        {{ name: "title", kind: "text" }},
        {{ name: "attachment", kind: "file" }}
      ],
      values: {{ title: "  Quarterly report  " }}
    }}
  }},
  response: Symbol.for("text"),
  onSuccess: Symbol.for("uploaded"),
  onError: Symbol.for("upload-failed")
}});
if (multipartMessage.kind !== Symbol.for("uploaded")) throw new Error("multipart descriptor did not succeed");
const multipartBody = descriptorRequests[1].options.body;
if (!(multipartBody instanceof FakeFormData)) throw new Error("multipart descriptor did not create FormData");
if (multipartBody.entries.length !== 2) throw new Error("multipart descriptor appended the wrong entry count");
if (multipartBody.entries[0].name !== "title" || multipartBody.entries[0].value !== "Quarterly report") {{
  throw new Error("multipart descriptor did not append trimmed text");
}}
if (multipartBody.entries[1].name !== "attachment" || multipartBody.entries[1].value !== multipartFile || multipartBody.entries[1].filename !== "report.pdf") {{
  throw new Error("multipart descriptor did not append the selected file");
}}

const requests = [];
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{
    async fetch(url, options) {{
      requests.push({{ url, options }});
      return {{
        status: 200,
        ok: true,
        async json() {{
          return {{ entries: [{{ id: "a" }}, {{ id: "b" }}] }};
        }}
      }};
    }}
  }})
}});

const section = host.children[0];
const button = section.children.find((node) => node.tagName === "button");
const status = section.children.find((node) => node.tagName === "span");
const message = section.children.find((node) => node.tagName === "small");
const statusText = status.children.find((node) => "nodeValue" in node);
const messageText = message.children.find((node) => "nodeValue" in node);

if (statusText.nodeValue !== "Idle") throw new Error("initial HTTP status was wrong");
if (section.hasAttribute("data-loaded")) throw new Error("loaded attr should start absent");
if (section.attributes["data-count"] !== "0") throw new Error("initial entry count attr was wrong");

button.click();
if (statusText.nodeValue !== "Loading") throw new Error("load click did not enter loading state");
if (app.commands.length !== 1 || app.commands[0].kind !== "http/request") throw new Error("http request command was not logged");
if (app.commands[0].command.request.url !== "/api/exercise-log") throw new Error("http request URL was wrong");
await new Promise((resolve) => setTimeout(resolve, 0));

if (host.children[0] !== section) throw new Error("HTTP section was replaced");
if (section.children.find((node) => node.tagName === "button") !== button) throw new Error("HTTP button was replaced");
if (status.children.find((node) => "nodeValue" in node) !== statusText) throw new Error("HTTP status text was replaced");
if (message.children.find((node) => "nodeValue" in node) !== messageText) throw new Error("HTTP message text was replaced");
if (requests.length !== 1 || requests[0].url !== "/api/exercise-log") throw new Error("fetch was not called with the command URL");
if (requests[0].options.method !== "GET") throw new Error("fetch options were not passed");
if (Object.prototype.hasOwnProperty.call(requests[0].options, "url")) throw new Error("fetch init options should not include request.url");
if (app.state.entries.length !== 2 || app.state.entries[1].id !== "b") throw new Error("HTTP body did not update entries");
if (app.state["loaded?"] !== true) throw new Error("HTTP success did not mark loaded");
if (statusText.nodeValue !== "Loaded") throw new Error("HTTP success status did not render");
if (messageText.nodeValue !== "Status 200") throw new Error("HTTP status message did not render");
if (section.attributes["data-loaded"] !== "") throw new Error("loaded attr was not set");
if (section.attributes["data-count"] !== "2") throw new Error("entry count attr did not update");

const errorHost = new Element("main");
const errorApp = runtime.startApp({{
  root: errorHost,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{
    async fetch() {{
      throw new Error("server asleep");
    }}
  }})
}});
const errorSection = errorHost.children[0];
const errorButton = errorSection.children.find((node) => node.tagName === "button");
const errorStatus = errorSection.children.find((node) => node.tagName === "span");
const errorMessage = errorSection.children.find((node) => node.tagName === "small");
const errorStatusText = errorStatus.children.find((node) => "nodeValue" in node);
const errorMessageText = errorMessage.children.find((node) => "nodeValue" in node);

errorButton.click();
await new Promise((resolve) => setTimeout(resolve, 0));
if (errorApp.commands.length !== 1 || errorApp.commands[0].kind !== "http/request") throw new Error("failed HTTP command was not logged");
if (errorApp.state.status !== "Offline") throw new Error("HTTP failure did not update state");
if (errorApp.state["loaded?"] !== false) throw new Error("HTTP failure should not mark loaded");
if (errorStatusText.nodeValue !== "Offline") throw new Error("HTTP failure status did not render");
if (errorMessageText.nodeValue !== "server asleep") throw new Error("HTTP failure message did not render");
if (errorSection.hasAttribute("data-loaded")) throw new Error("loaded attr should stay absent after failure");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated HTTP app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_http_app_routes_rejected_custom_handler_to_on_error() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-http-rejected-handler-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_http_app.clsk");
    let output = temp_dir.join("http-rejected-handler.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const unhandled = [];
process.on("unhandledRejection", (reason) => {{
  unhandled.push(reason);
}});

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: {{
    "http/request"() {{
      return Promise.reject(new Error("gateway down"));
    }}
  }},
  devtools: (event) => devEvents.push(event)
}});

const section = host.children[0];
const button = section.children.find((node) => node.tagName === "button");
const status = section.children.find((node) => node.tagName === "span");
const message = section.children.find((node) => node.tagName === "small");
const statusText = status.children.find((node) => "nodeValue" in node);
const messageText = message.children.find((node) => "nodeValue" in node);

button.click();
if (statusText.nodeValue !== "Loading") throw new Error("custom rejected handler test did not enter loading state");
await new Promise((resolve) => setTimeout(resolve, 0));
await new Promise((resolve) => setTimeout(resolve, 0));

if (unhandled.length !== 0) throw new Error(`custom handler rejection leaked unhandled: ${{unhandled[0]?.message || unhandled[0]}}`);
if (app.commands.length !== 1 || app.commands[0].kind !== "http/request") throw new Error("custom rejected handler command was not logged");
if (app.state.status !== "Offline") throw new Error("custom rejected handler did not route to onError state");
if (app.state.message !== "gateway down") throw new Error(`custom rejected handler error message was wrong: ${{app.state.message}}`);
if (statusText.nodeValue !== "Offline") throw new Error("custom rejected handler status did not render");
if (messageText.nodeValue !== "gateway down") throw new Error("custom rejected handler error text did not render");
if (section.hasAttribute("data-loaded")) throw new Error("custom rejected handler should not mark loaded");

const commandError = devEvents.find((event) => event.type === "command/error" && event.kind === "http/request");
if (!commandError || commandError.error !== "gateway down") throw new Error("devtools did not report custom command rejection");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated HTTP rejected-handler app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_dispose_ignores_late_http_results() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-dispose-async-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_dispose_async_app.clsk");
    let output = temp_dir.join("dispose-async-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index >= 0) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

const host = new Element("main");
const devEvents = [];
const requests = [];
let resolveFetch;
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{
    fetch(url, options) {{
      requests.push({{ url, options }});
      return new Promise((resolve) => {{
        resolveFetch = resolve;
      }});
    }}
  }}),
  devtools: (event) => devEvents.push(event)
}});

const section = host.children[0];
const button = section.children.find((node) => node.tagName === "button");
const status = section.children.find((node) => node.tagName === "span");
const statusText = status.children.find((node) => "nodeValue" in node);

button.click();
if (statusText.nodeValue !== "Loading") throw new Error("load click did not enter loading state");
if (app.state.status !== "Loading") throw new Error("state did not enter loading state");
if (requests.length !== 1 || requests[0].url !== "/api/later") throw new Error("HTTP request was not started");
if (app.commands.length !== 1 || app.commands[0].kind !== "http/request") throw new Error("HTTP command was not logged");
if (!resolveFetch) throw new Error("test fetch promise was not captured");

app.dispose();
if (host.children.length !== 0) throw new Error("dispose did not remove the mounted section");
const disposeEventIndex = devEvents.findIndex((event) => event.type === "app/dispose");
if (disposeEventIndex < 0) throw new Error("devtools did not report app disposal");

resolveFetch({{
  status: 200,
  ok: true,
  async json() {{
    return {{ entries: [{{ id: "late" }}] }};
  }}
}});
await new Promise((resolve) => setTimeout(resolve, 0));

if (app.state.status !== "Loading") throw new Error("late HTTP success mutated disposed app state");
if (app.state["loaded?"] !== false) throw new Error("late HTTP success marked disposed app loaded");
if (statusText.nodeValue !== "Loading") throw new Error("late HTTP success updated detached DOM text");
if (host.children.length !== 0) throw new Error("late HTTP success remounted disposed DOM");
const postDisposeStateEvents = devEvents
  .slice(disposeEventIndex + 1)
  .filter((event) => event.type === "state/update");
if (postDisposeStateEvents.length !== 0) throw new Error("late HTTP success emitted state updates after dispose");

app.dispatch({{ kind: Symbol.for("loaded"), value: {{ status: 200, ok: true, body: {{ entries: [] }} }} }});
if (app.state.status !== "Loading") throw new Error("manual dispatch mutated disposed app state");
if (host.children.length !== 0) throw new Error("manual dispatch remounted disposed DOM");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated dispose async app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn expand_reports_macro_compile_error_location() {
    let temp_dir = env::temp_dir().join(format!("closkell-macro-error-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let source = temp_dir.join("macro-error.clsk");
    fs::write(
        &source,
        "(defmacro require-kind [] (compile-error \"command macro requires :kind\"))\n(require-kind)\n",
    )
    .expect("macro error source should be written");

    let expand = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("expand")
        .arg(&source)
        .output()
        .expect("closkell expand should run");

    let _ = fs::remove_dir_all(&temp_dir);

    let stdout = String::from_utf8_lossy(&expand.stdout);
    let stderr = String::from_utf8_lossy(&expand.stderr);

    assert!(
        !expand.status.success(),
        "expand should fail for compile-error\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(
        stdout.contains("Error at 2:1: command macro requires :kind"),
        "compile-error diagnostic did not point at macro invocation\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(
        stdout.contains("nil"),
        "compile-error should lower to nil after reporting\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
}

#[test]
fn compiled_hrweb_macro_app_expands_before_build() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-macro-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_macro_app.clsk");
    let output = temp_dir.join("macro-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let emitted = fs::read_to_string(&output).expect("generated macro app should be readable");
    assert!(!emitted.contains("defmacro"));
    assert!(emitted.contains("Symbol.for(\"storage/set\")"));

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
const [savingState, storageCommand] = mod.update(mod.init, Symbol.for("start"));
if (savingState.label !== "Saving") throw new Error("macro-expanded start branch did not run");
if (storageCommand.kind !== Symbol.for("storage/set")) throw new Error("storage-set macro did not expand");
if (storageCommand.msg !== Symbol.for("stored")) throw new Error("storage-set macro did not preserve completion message");

const [savedState, noneCommand] = mod.update(savingState, Symbol.for("stored"));
if (savedState["saved?"] !== true) throw new Error("stored branch did not update state");
if (noneCommand.kind !== Symbol.for("none")) throw new Error("cmd-none macro did not expand");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
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
        "generated macro app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_set_ops_preserve_immutable_sets() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_set_ops.clsk");
    let output = env::temp_dir().join(format!("closkell-set-ops-{}.mjs", std::process::id()));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (!(mod.workout_tags instanceof Set)) throw new Error("set literal did not emit a Set");
if (mod.workout_tags.size !== 2) throw new Error("set literal did not de-duplicate tags");
if (!(mod.workout_tag_set instanceof Set) || mod.workout_tag_set.size !== 3) throw new Error("set constructor did not create unique tags");
if (!mod.known_tag_(mod.workout_tags, "zone2")) throw new Error("contains? did not find an existing tag");
if (mod.known_tag_(mod.workout_tags, "tempo")) throw new Error("contains? matched a missing tag");

const added = mod.add_tags(mod.workout_tags, "tempo", "zone2");
if (!(added instanceof Set) || !added.has("tempo") || added.size !== 3) throw new Error("conj did not return a new Set with the added tag");
if (mod.workout_tags.has("tempo")) throw new Error("conj mutated the original Set");

const removed = mod.remove_tag(added, "steady");
if (!(removed instanceof Set) || removed.has("steady") || !removed.has("zone2")) throw new Error("disj did not remove the requested tag");
if (!added.has("steady")) throw new Error("disj mutated the original Set");

const summary = mod.summarize_tags(removed);
if (summary.count !== 2 || summary.empty !== false || summary.hasZone2 !== true || summary.set !== true) {{
  throw new Error("set summary was wrong");
}}

const empty = mod.summarize_tags(mod.remove_tag(mod.remove_tag(removed, "zone2"), "tempo"));
if (empty.count !== 0 || empty.empty !== true || empty.hasZone2 !== false || empty.set !== true) {{
  throw new Error("empty set summary was wrong");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated set ops module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_exercise_type_set_enumerates_sorted_options() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_exercise_type_set.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-exercise-type-set-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (!(mod.exercise_type_set instanceof Set)) throw new Error("exercise type collection should be a Set");
if (mod.exercise_type_set.size !== 2 || !mod.exercise_type_set.has("LISS") || !mod.exercise_type_set.has("Strength")) {{
  throw new Error("exercise type set did not deduplicate visible typed entries");
}}
if (mod.exercise_type_set.has("")) throw new Error("empty exercise types should not be collected");
if (mod.exercise_type_options.join(",") !== "LISS,Strength") throw new Error("set-values did not feed sorted exercise type options");

const next = [
  {{ id: "ride", exerciseType: "Cycling", hiddenAt: null }},
  {{ id: "walk", exerciseType: "LISS", hiddenAt: null }},
  {{ id: "hidden", exerciseType: "Hidden", hiddenAt: 1 }},
  {{ id: "blank", exerciseType: "", hiddenAt: null }},
  {{ id: "lift", exerciseType: "Strength", hiddenAt: null }},
  {{ id: "ride-2", exerciseType: "Cycling", hiddenAt: null }}
];
const types = mod.collect_type_set(next);
if (!(types instanceof Set) || types.size !== 3 || types.has("Hidden") || types.has("")) throw new Error("collect-type-set mishandled filtered entries");
if (mod.sorted_exercise_types(next).join(",") !== "Cycling,LISS,Strength") throw new Error("sorted exercise types were wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated exercise type set module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_metric_registry_preserves_immutable_maps() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_metric_registry.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-metric-registry-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (!(mod.metric_registry instanceof Map)) throw new Error("hash-map did not emit a Map");
if (mod.metric_registry.size !== 2) throw new Error("metric registry should start with two entries");

const zone2 = mod.select_metric(mod.metric_registry, "zone2");
if (!zone2 || zone2.label !== "Zone 2") throw new Error("map-get did not read an existing metric");
if (mod.select_metric(mod.metric_registry, "hrr") !== null) throw new Error("map-get should return null for a missing metric");

const hrr = {{ id: "hrr", label: "Heart-rate recovery", value: 35, unit: "bpm" }};
const expanded = mod.register_metric(mod.metric_registry, hrr);
if (!(expanded instanceof Map) || expanded.get("hrr") !== hrr) throw new Error("map-assoc did not add the new metric");
if (mod.metric_registry.has("hrr")) throw new Error("map-assoc mutated the original registry");

const pruned = mod.remove_metric(expanded, "trimp");
if (!(pruned instanceof Map) || pruned.has("trimp") || !pruned.has("hrr")) throw new Error("map-dissoc did not remove the requested metric");
if (!expanded.has("trimp")) throw new Error("map-dissoc mutated the expanded registry");

const summary = mod.registry_summary;
if (summary.map !== true || summary.knownZone2 !== true || summary.originalCount !== 2 || summary.expandedCount !== 3 || summary.prunedCount !== 2) {{
  throw new Error("registry summary counts or predicates were wrong");
}}
if (!summary.selected || summary.selected.id !== "hrr") throw new Error("registry summary did not keep the selected metric");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated metric registry module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_map_enumeration_feeds_metric_reductions() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_map_enumeration.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-map-enumeration-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (!(mod.sample_zone_durations instanceof Map)) throw new Error("sample durations should be a Map");
if (mod.total_tracked_ms(mod.sample_zone_durations) !== 60000) throw new Error("map-values did not feed the duration reducer");
if (mod.trimp_from_durations(mod.sample_zone_durations) !== 2.3) throw new Error("map-entries did not feed the TRIMP reducer");

const active = mod.active_zone_ids(mod.sample_zone_durations);
if (active.length !== 2 || active[0] !== 2 || active[1] !== 3) throw new Error("active zone ids were wrong");

const summary = mod.zone_duration_summary;
if (summary.totalMs !== 60000 || summary.trimp !== 2.3) throw new Error("zone duration summary values were wrong");
if (summary.zoneIds.join(",") !== "1,2,3,4,5") throw new Error("map-keys returned the wrong ids");
if (summary.activeZoneIds.join(",") !== "2,3") throw new Error("summary active zones were wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated map enumeration module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_numeric_idioms_match_app_math() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_numeric_idioms.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-numeric-idioms-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (mod.hrr_recovery_ms !== 60000) throw new Error("numeric separator did not preserve recovery ms");
if (mod.hrr_min_peak_gap_ms !== 30000) throw new Error("numeric separator did not preserve peak gap ms");
if (mod.clamp_zone_boundary(189.7, 30, 190) !== 189) throw new Error("zone boundary clamp was wrong");
if (mod.hold_progress(10750, 10000) !== 0.5) throw new Error("hold progress was wrong");
if (mod.hold_progress(9000, 10000) !== 0) throw new Error("hold progress should clamp low");
if (mod.hold_progress(13000, 10000) !== 1) throw new Error("hold progress should clamp high");
if (mod.trend_delta_label(121, 135) !== "14 bpm down") throw new Error("abs did not produce the trend delta label");
if (mod.trend_delta_label(150, 140) !== "10 bpm up") throw new Error("positive trend label was wrong");
if (mod.trend_delta_label(140, 140) !== "Recording") throw new Error("flat trend label was wrong");

const summary = mod.numeric_idiom_summary;
if (summary.recoveryMs !== 60000 || summary.peakGapMs !== 30000 || summary.boundary !== 189 || summary.progress !== 0.5 || summary.delta !== "14 bpm down") {{
  throw new Error("numeric summary values were wrong");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated numeric idioms module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_axis_label_precision_matches_chart_logic() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_axis_label_precision.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-axis-label-precision-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (mod.axis_minute_label(60000, 0) !== "0m") throw new Error("zero minute label was wrong");
if (mod.axis_minute_label(60000, 0.5) !== "1m") throw new Error("sub-minute rounded label was wrong");
if (mod.axis_minute_label(60000, 1) !== "1.0m") throw new Error("one-minute fixed label was wrong");
if (mod.axis_minute_label(3600000, 0.5) !== "30m") throw new Error("long-axis rounded label was wrong");

if (mod.sample_axis_labels.join(",") !== "0m,0m,1m,1.0m") throw new Error("sample axis labels were wrong");
if (mod.long_axis_labels.join(",") !== "0m,30m,60m") throw new Error("long axis labels were wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated axis label precision module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_chart_bounds_uses_numeric_vector_aggregates() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_chart_bounds.clsk");
    let output = env::temp_dir().join(format!("closkell-chart-bounds-{}.mjs", std::process::id()));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));

if (mod.reading_values(mod.sample_readings).join(",") !== "112,138,151,145") {{
  throw new Error("reading values were not projected");
}}
if (mod.desktop_bounds.min !== 50 || mod.desktop_bounds.max !== 170 || mod.desktop_bounds.span !== 120 || mod.desktop_bounds.avg !== 137) {{
  throw new Error(`desktop bounds were wrong: ${{JSON.stringify(mod.desktop_bounds)}}`);
}}
if (mod.mobile_bounds.min !== 95 || mod.mobile_bounds.max !== 159 || mod.mobile_bounds.span !== 64 || mod.mobile_bounds.avg !== 137) {{
  throw new Error(`mobile bounds were wrong: ${{JSON.stringify(mod.mobile_bounds)}}`);
}}

const empty = mod.chart_bounds([], mod.sample_zones, true);
if (empty.min !== 50 || empty.max !== 170 || empty.avg !== 0) {{
  throw new Error(`empty chart bounds did not use fallbacks: ${{JSON.stringify(empty)}}`);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated chart bounds module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_selected_log_validity_uses_any_predicate() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_selected_log_validity.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-selected-log-validity-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (mod.visible_entries(mod.sample_state.entries).length !== 2) throw new Error("visible entries should filter hidden logs");
if (!mod.selected_id_visible_(mod.visible_entries(mod.sample_state.entries), "lift")) throw new Error("any? did not find the selected visible id");
if (mod.selected_id_visible_(mod.visible_entries(mod.sample_state.entries), "archived")) throw new Error("any? matched a hidden id");

if (mod.reconciled_selected.selectedLogId !== "lift") throw new Error("valid selected id should be preserved");
if (mod.reconciled_missing.selectedLogId !== "warmup") throw new Error("missing selected id should fall back to the first visible log");
if (mod.reconciled_empty.selectedLogId !== "") throw new Error("empty visible log should clear the selection sentinel");

const repaired = mod.reconcile_selected_log({{
  entries: [{{ id: "first", hiddenAt: null }}, {{ id: "second", hiddenAt: null }}],
  selectedLogId: "missing"
}});
if (repaired.selectedLogId !== "first") throw new Error("reconcile-selected-log did not pick the first visible entry");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated selected log validity module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_metric_visibility_uses_vector_includes() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_metric_visibility.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-metric-visibility-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (mod.liss_metrics.join(",") !== "zone2,trimp") throw new Error("LISS metrics were wrong");
if (mod.strength_metrics.join(",") !== "hrr,trimp") throw new Error("strength metrics were wrong");
if (mod.untyped_metrics.join(",") !== "trimp") throw new Error("fallback metrics were wrong");

if (!mod.metric_enabled_(["zone2", "trimp"], "zone2")) throw new Error("vector includes? did not find zone2");
if (mod.metric_enabled_(["trimp"], "zone2")) throw new Error("vector includes? matched a missing metric");

const visibility = mod.metric_visibility(["hrr", "trimp"]);
if (visibility.zone2 !== false || visibility.hrr !== true || visibility.trimp !== true) {{
  throw new Error("metric visibility flags were wrong");
}}
if (mod.liss_visibility.zone2 !== true || mod.liss_visibility.hrr !== false || mod.liss_visibility.trimp !== true) {{
  throw new Error("sample LISS visibility flags were wrong");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated metric visibility module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_sort_with_groups_matches_custom_comparators() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_sort_with_groups.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-sort-with-groups-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
const sorted = mod.sorted_type_groups;
if (sorted.map((group) => group.type).join(",") !== "LISS,Strength,Untyped") throw new Error("custom group order was wrong");
if (sorted[0].entries.map((entry) => entry.id).join(",") !== "jog,walk") throw new Error("LISS entries were not sorted newest first");
if (sorted[1].entries.map((entry) => entry.id).join(",") !== "lift-new,lift-old") throw new Error("Strength entries were not sorted newest first");
if (sorted[2].entries[0].id !== "mystery") throw new Error("untyped group was not preserved");

if (mod.sample_type_groups.map((group) => group.type).join(",") !== "Strength,__untyped__,LISS") throw new Error("sort-with mutated the original groups");
if (mod.sample_type_groups[0].entries.map((entry) => entry.id).join(",") !== "lift-old,lift-new") throw new Error("nested sort-with mutated original entries");
if (mod.compare_type_groups({{ type: "LISS", entries: [] }}, {{ type: "Strength", entries: [] }}) >= 0) throw new Error("locale-compare comparator was wrong");
if (mod.compare_type_groups({{ type: "__untyped__", entries: [] }}, {{ type: "LISS", entries: [] }}) <= 0) throw new Error("untyped group should sort last");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated sort-with groups module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_result_helpers_route_import_success_and_failure() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_result_helpers.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-result-helpers-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (mod.valid_result.ok !== true || mod.valid_result.value.length !== 2) throw new Error("valid import did not produce an ok Result");
if (mod.invalid_result.ok !== false || mod.invalid_result.error !== "import payload is missing valid entries") throw new Error("invalid import did not produce an err Result");
if (mod.imported_count !== 2) throw new Error(`expected imported count 2, found ${{mod.imported_count}}`);
if (mod.fallback_count !== 0) throw new Error(`expected fallback count 0, found ${{mod.fallback_count}}`);
if (mod.import_message !== "import payload is missing valid entries") throw new Error("result-error did not expose the error message");
if (mod.import_flags.valid !== true || mod.import_flags.invalid !== true) throw new Error("result predicates returned the wrong flags");

const parsed = mod.parse_import(JSON.stringify({{ entries: [{{ id: "ride", durationMs: 60000 }}] }}));
if (parsed.ok !== true || parsed.value[0].id !== "ride") throw new Error("parse-import did not accept a valid payload");
const rejected = mod.parse_import(JSON.stringify({{ entries: [{{ id: 42 }}] }}));
if (rejected.ok !== false) throw new Error("parse-import accepted an invalid payload");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated result helpers module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_storage_reset_removes_saved_log() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-storage-reset-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_storage_reset_app.clsk");
    let output = temp_dir.join("storage-reset.mjs");
    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));

const storage = {{
  values: new Map([["heartRateExercise.log.v1", JSON.stringify({{ version: 2, entries: ["warmup"] }})]]),
  getItem(key) {{ return this.values.get(key) ?? null; }},
  setItem(key, value) {{ this.values.set(key, value); }},
  removeItem(key) {{ this.values.delete(key); this.removed = key; }}
}};
const handlers = runtime.createCommandHandlers({{ storage }});

const [resetting, command] = mod.update(mod.init, {{ kind: Symbol.for("reset-log") }});
if (resetting.status !== "Resetting") throw new Error("reset action did not enter Resetting state");
if (command.kind !== Symbol.for("storage/remove")) throw new Error("reset action did not emit storage/remove");
if (command.key !== "heartRateExercise.log.v1") throw new Error("storage/remove key was wrong");
if (command.onSuccess !== Symbol.for("reset-complete")) throw new Error("storage/remove success tag was wrong");

const completion = handlers["storage/remove"](command);
if (storage.values.has("heartRateExercise.log.v1")) throw new Error("storage/remove did not delete the saved log");
if (storage.removed !== "heartRateExercise.log.v1") throw new Error("storage/remove did not call removeItem with the key");
if (completion.kind !== Symbol.for("reset-complete")) throw new Error("storage/remove completion kind was wrong");
if (completion.value.key !== "heartRateExercise.log.v1") throw new Error("storage/remove completion payload missed the key");

const [reset, done] = mod.update(resetting, completion);
if (reset.status !== "Reset") throw new Error("reset completion did not set Reset status");
if (reset.entries.length !== 0) throw new Error("reset completion did not clear entries");
if (reset.removedKey !== "heartRateExercise.log.v1") throw new Error("reset completion did not store removed key");
if (done.kind !== Symbol.for("none")) throw new Error("reset completion should emit no command");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated storage reset module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_tail_recursion_runs_without_stack_growth() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_tail_recursion.clsk");
    let output = env::temp_dir().join(format!(
        "closkell-tail-recursion-{}.mjs",
        std::process::id()
    ));

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let emitted = fs::read_to_string(&output).expect("generated tail module should be readable");
    assert!(
        emitted.contains("while (true)") && emitted.contains("continue;"),
        "tail-recursive helper was not lowered to a loop\n{}",
        emitted
    );

    let script = format!(
        r#"
const mod = await import(fileUrl({modulePath}));
if (mod.sample_total !== 380) throw new Error(`expected sample total 380, found ${{mod.sample_total}}`);
const readings = Array.from({{ length: 25000 }}, (_, index) => ({{ bpm: 1, time: index }}));
const total = mod.sum_bpm(readings);
if (total !== 25000) throw new Error(`expected large tail-recursive total 25000, found ${{total}}`);

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
    );

    let node = Command::new("node")
        .arg("--input-type=module")
        .arg("--eval")
        .arg(script)
        .output()
        .expect("node should run");

    let _ = fs::remove_file(&output);

    assert!(
        node.status.success(),
        "generated tail-recursive module failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_log_list_reuses_keyed_rows() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-log-list-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_log_list.clsk");
    let output = temp_dir.join("log-list.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener() {{}}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const host = new Element("main");
const component = mod.view(mod.init);
component.mount(host, () => {{}});

const section = host.children[0];
let rows = articles(section);
if (rows.length !== 2) throw new Error(`expected 2 initial rows, found ${{rows.length}}`);
const rowA = rows[0];
const rowB = rows[1];
if (rowA.attributes["data-id"] !== "a" || rowB.attributes["data-id"] !== "b") throw new Error("initial keyed order was wrong");
if (textOf(rowA, "span") !== "Walk" || textOf(rowB, "strong") !== "45:00") throw new Error("initial row text was wrong");

component.update({{ entries: [
  {{ id: "b", label: "Bike", duration: "46:00" }},
  {{ id: "c", label: "Run", duration: "30:00" }},
  {{ id: "a", label: "Walk", duration: "21:00" }}
] }});

rows = articles(section);
if (rows.length !== 3) throw new Error(`expected 3 updated rows, found ${{rows.length}}`);
if (rows[0] !== rowB) throw new Error("row b was not reused during reorder");
if (rows[2] !== rowA) throw new Error("row a was not reused during reorder");
if (rows.map((row) => row.attributes["data-id"]).join(",") !== "b,c,a") throw new Error("updated keyed order was wrong");
if (textOf(rowB, "strong") !== "46:00") throw new Error("reused row b did not update duration");
if (textOf(rowA, "strong") !== "21:00") throw new Error("reused row a did not update duration");

function articles(parent) {{
  return parent.children.filter((child) => child.tagName === "article");
}}

function textOf(parent, tagName) {{
  const child = parent.children.find((node) => node.tagName === tagName);
  return child.children.find((node) => "nodeValue" in node).nodeValue;
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output)
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
        "generated log list failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_keyed_rows_skip_unchanged_local_slots() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-keyed-granular-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_keyed_granular_app.clsk");
    let output = temp_dir.join("keyed-granular-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of [...(this.listeners.click || [])]) {{
      listener({{ type: "click", currentTarget: this, target: this }});
    }}
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: (event) => devEvents.push(event)
}});

const section = host.children[0];
const rowA = rowById(section, "a");
const rowB = rowById(section, "b");
const labelTextA = textNodeOf(rowA, "label");
const durationTextA = textNodeOf(rowA, "duration");
if (!rowA || !rowB || labelTextA.nodeValue !== "Walk" || durationTextA.nodeValue !== "20:00") {{
  throw new Error("initial keyed rows were not mounted correctly");
}}

buttonByAction(section, "rename-a").click();
if (app.state.entries[0].label !== "Run") {{
  throw new Error("rename message did not update state");
}}
if (rowById(section, "a") !== rowA || rowById(section, "b") !== rowB) {{
  throw new Error("keyed rows were not reused after one item field changed");
}}
if (textNodeOf(rowA, "label") !== labelTextA || labelTextA.nodeValue !== "Run") {{
  throw new Error("changed row label text node was not updated in place");
}}
if (textNodeOf(rowA, "duration") !== durationTextA || durationTextA.nodeValue !== "20:00") {{
  throw new Error("unchanged row duration text node should stay in place");
}}

const rowTemplateUpdates = devEvents.filter((event) =>
  event.type === "template/update" && allSlots(event).some((slot) => hasRead(slot, "entry.label"))
);
const changedRowUpdate = rowTemplateUpdates.find((event) => hasReadIn(event.updatedSlots, "entry.label"));
if (!changedRowUpdate) {{
  throw new Error("devtools did not report the changed keyed row update");
}}
if (!changedRowUpdate.changedPaths.includes("state.entries.0.label")) {{
  throw new Error(`changed row update missed the source state path: ${{changedRowUpdate.changedPaths.join(",")}}`);
}}
if (!changedRowUpdate.localChangedPaths.includes("entry.label")) {{
  throw new Error(`changed row update missed the local item path: ${{changedRowUpdate.localChangedPaths.join(",")}}`);
}}
if (!hasReadIn(changedRowUpdate.skippedSlots, "entry.duration")) {{
  throw new Error("unchanged duration slot was not skipped inside the changed keyed row");
}}
if (hasReadIn(changedRowUpdate.updatedSlots, "entry.duration")) {{
  throw new Error("duration slot updated even though only the row label changed");
}}

const untouchedRowUpdate = rowTemplateUpdates.find((event) =>
  event.localChangedPaths.length === 0 &&
  hasReadIn(event.skippedSlots, "entry.label") &&
  hasReadIn(event.skippedSlots, "entry.duration") &&
  !hasReadIn(event.updatedSlots, "entry.label") &&
  !hasReadIn(event.updatedSlots, "entry.duration")
);
if (!untouchedRowUpdate) {{
  throw new Error("unchanged keyed row did not skip its local text slots");
}}

function articles(parent) {{
  return parent.children.filter((node) => node.tagName === "article");
}}

function rowById(parent, id) {{
  return articles(parent).find((node) => node.attributes["data-id"] === id);
}}

function buttonByAction(parent, action) {{
  return parent.children.find((node) => node.tagName === "button" && node.attributes["data-action"] === action);
}}

function textNodeOf(parent, role) {{
  const node = parent.children.find((child) => child.attributes?.["data-role"] === role);
  return node.children.find((child) => "nodeValue" in child);
}}

function allSlots(event) {{
  return [...(event.updatedSlots || []), ...(event.skippedSlots || [])];
}}

function hasRead(slot, read) {{
  return (slot.reads || []).includes(read);
}}

function hasReadIn(slots, read) {{
  return (slots || []).some((slot) => hasRead(slot, read));
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated keyed granular app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_keyed_list_disposes_detached_removed_rows() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-keyed-cleanup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_keyed_cleanup_app.clsk");
    let output = temp_dir.join("keyed-cleanup-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: (event) => devEvents.push(event)
}});

const section = host.children[0];
const rowA = rowById(section, "a");
const rowB = rowById(section, "b");
const staleSelectA = buttonByAction(rowA, "select");
if (!rowA || !rowB || !staleSelectA) throw new Error("initial keyed rows were not mounted");
if (section.attributes["data-count"] !== "2") throw new Error("initial keyed count was wrong");

section.removeChild(rowA);
if (rowA.parentNode !== null) throw new Error("manual detach did not detach row a");

app.dispatch({{ kind: Symbol.for("hide"), id: "a" }});
if (app.state.entries.map((entry) => entry.id).join(",") !== "b") throw new Error("hide did not remove entry a from state");
if (articles(section).map((row) => row.attributes["data-id"]).join(",") !== "b") throw new Error("keyed DOM did not settle on row b");
if (rowById(section, "b") !== rowB) throw new Error("row b was not preserved after keyed cleanup");
if (app.state.selected !== "") throw new Error("hide unexpectedly selected a row");

const stateUpdatesBeforeStaleClick = devEvents.filter((event) => event.type === "state/update").length;
staleSelectA.click();
if (app.state.selected !== "") throw new Error("stale detached keyed row dispatched after disposal");
if (section.attributes["data-selected"] !== "") throw new Error("stale detached keyed row updated selected attr");
const stateUpdatesAfterStaleClick = devEvents.filter((event) => event.type === "state/update").length;
if (stateUpdatesAfterStaleClick !== stateUpdatesBeforeStaleClick) {{
  throw new Error("stale detached keyed row emitted a state update after disposal");
}}

function articles(parent) {{
  return parent.children.filter((node) => node.tagName === "article");
}}

function rowById(parent, id) {{
  return articles(parent).find((node) => node.attributes["data-id"] === id);
}}

function buttonByAction(parent, action) {{
  return parent.children.find((node) => node.tagName === "button" && node.attributes["data-action"] === action);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated keyed cleanup app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_keyed_list_updates_duplicate_keys_without_orphans() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-duplicate-keyed-app-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_duplicate_keyed_app.clsk");
    let output = temp_dir.join("duplicate-keyed-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: (event) => devEvents.push(event)
}});

const section = host.children[0];
let rows = articles(section);
if (labels(rows).join(",") !== "Walk,Bike") throw new Error(`initial duplicate rows were wrong: ${{labels(rows).join(",")}}`);
const firstDuplicate = rows[0];
const secondDuplicate = rows[1];

buttonOf(secondDuplicate).click();
if (app.state.selected !== "Bike") throw new Error("second duplicate row did not dispatch its own label");

actionButton(section, "replace").click();
rows = articles(section);
if (labels(rows).join(",") !== "Run,Swim,Yoga") throw new Error(`duplicate rows did not update cleanly: ${{labels(rows).join(",")}}`);
if (rows[0] !== firstDuplicate) throw new Error("first duplicate occurrence was not reused");
if (rows[1] !== secondDuplicate) throw new Error("second duplicate occurrence was not reused");
if (section.attributes["data-count"] !== "3") throw new Error("replace count attr was wrong");
buttonOf(secondDuplicate).click();
if (app.state.selected !== "Swim") throw new Error("second duplicate row kept a stale event payload");

actionButton(section, "shrink").click();
rows = articles(section);
if (labels(rows).join(",") !== "Only") throw new Error(`duplicate shrink left orphan rows: ${{labels(rows).join(",")}}`);
if (rows[0] !== firstDuplicate) throw new Error("first duplicate row was not reused after shrink");
if (secondDuplicate.parentNode !== null) throw new Error("second duplicate row was not detached after shrink");

secondDuplicate.children.find((node) => node.tagName === "button").click();
if (app.state.selected !== "Swim") throw new Error("disposed duplicate row dispatched after shrink");
if (section.attributes["data-selected"] !== "Swim") throw new Error("disposed duplicate row changed rendered selection");

function articles(parent) {{
  return parent.children.filter((node) => node.tagName === "article");
}}

function labels(rows) {{
  return rows.map((row) => row.attributes["data-label"]);
}}

function buttonOf(row) {{
  return row.children.find((node) => node.tagName === "button");
}}

function actionButton(parent, action) {{
  return parent.children.find((node) => node.tagName === "button" && node.attributes["data-action"] === action);
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated duplicate keyed app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_zone_style_view_updates_dynamic_styles() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-zone-style-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_zone_style_view.clsk");
    let output = temp_dir.join("zone-style-view.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class StyleDecl {{
  constructor() {{
    this.props = {{}};
    this.cssText = "";
  }}
  setProperty(name, value) {{
    this.props[name] = String(value);
  }}
  removeProperty(name) {{
    delete this.props[name];
  }}
}}

class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = new StyleDecl();
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{ root: host, init: mod.init, update: mod.update, view: mod.view }});

const section = host.children[0];
const zoneWrap = section.children.find((node) => node.tagName === "div");
let buttons = zoneButtons(zoneWrap);
if (buttons.length !== 3) throw new Error(`expected 3 zone buttons, found ${{buttons.length}}`);
const zone2 = buttons[0];
const zone3 = buttons[1];
const zone4 = buttons[2];
const zone2Text = labelText(zone2);
const zone4Text = labelText(zone4);
const status = section.children.find((node) => node.tagName === "span");
const statusText = status.children.find((node) => "nodeValue" in node);

if (section.attributes["data-target"] !== "3") throw new Error("initial target attr was wrong");
if (statusText.nodeValue !== "Zone 3") throw new Error("initial status was wrong");
if (zone2.attributes["data-index"] !== "0" || zone4.attributes["data-index"] !== "2") throw new Error("indexed loop data attrs were wrong");
if (zone2Text.nodeValue !== "1. Zone 2" || zone4Text.nodeValue !== "3. Zone 4") throw new Error("indexed loop labels were wrong");
if (zone2.style.props.background !== "#2a9d8f") throw new Error("zone 2 background style was wrong");
if (zone2.style.props["border-color"] !== "#2a9d8f" || zone2.style.props.borderColor !== undefined) {{
  throw new Error(`zone 2 camelCase border style was not normalized: ${{JSON.stringify(zone2.style.props)}}`);
}}
if (zone2.style.props["--zone-color"] !== "#2a9d8f") throw new Error("zone 2 custom style property was wrong");
if (zone3.style.props["flex-basis"] !== "33%") throw new Error("zone flex basis was wrong");
if (zone2.style.props.opacity !== "0.75" || zone3.style.props.opacity !== "1") throw new Error("initial zone opacity was wrong");
if (zone2.style.props["box-shadow"] !== undefined) throw new Error("unselected zone should not have a shadow");
if (!zone3.style.props["box-shadow"]?.includes("rgba")) throw new Error("selected zone shadow was not set");
if (zone3.style.props.boxShadow !== undefined) throw new Error("camelCase boxShadow should be normalized to box-shadow");
if (zone3.attributes.style !== undefined) throw new Error("style object should not be stringified into an attribute");

zone4.click();
buttons = zoneButtons(zoneWrap);
if (host.children[0] !== section) throw new Error("zone section was replaced after target change");
if (buttons[0] !== zone2 || buttons[1] !== zone3 || buttons[2] !== zone4) throw new Error("keyed zone buttons were not reused");
if (app.state.targetZoneId !== 4) throw new Error("zone click did not update target id");
if (section.attributes["data-target"] !== "4") throw new Error("target attr did not update");
if (statusText.nodeValue !== "Zone 4 rank 3") throw new Error("indexed click message did not update status");
if (zone3.style.props["box-shadow"] !== undefined) throw new Error("stale selected shadow was not removed");
if (!zone4.style.props["box-shadow"]?.includes("rgba")) throw new Error("new selected shadow was not set");
if (zone4.style.props.boxShadow !== undefined) throw new Error("updated camelCase boxShadow should be normalized to box-shadow");
if (zone3.style.props.opacity !== "0.75" || zone4.style.props.opacity !== "1") throw new Error("updated zone opacity was wrong");
if (zone4.style.props.background !== "#f77f00") throw new Error("zone 4 background style changed unexpectedly");
if (zone4.style.props["border-color"] !== "#f77f00" || zone4.style.props.borderColor !== undefined) {{
  throw new Error(`zone 4 camelCase border style was not normalized: ${{JSON.stringify(zone4.style.props)}}`);
}}
if (zone4.style.props["--zone-color"] !== "#f77f00") throw new Error("zone 4 custom style property was wrong");
if (zone4.attributes.style !== undefined) throw new Error("updated style object should not become an attribute");
if (labelText(zone4) !== zone4Text) throw new Error("indexed label text node was replaced");

function zoneButtons(parent) {{
  return parent.children.filter((node) => node.tagName === "button");
}}

function labelText(node) {{
  return node.children.find((child) => "nodeValue" in child && child.nodeValue.includes("Zone"));
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated zone style view failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_conditional_view_swaps_and_reuses_branches() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-conditional-view-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_conditional_view.clsk");
    let output = temp_dir.join("conditional-view.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((candidate) => candidate !== listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: {{ events: devEvents }}
}});

const section = host.children[0];
let article = visibleArticle(section);
const idleArticle = article;
const idleLabel = textOf(article, "em");
const idleDetail = textOf(article, "p");
if (section.hasAttribute("data-connected")) throw new Error("connected attr should start absent");
if (article.attributes["data-kind"] !== "idle") throw new Error("idle branch was not mounted");
if (idleLabel.nodeValue !== "Idle" || idleDetail.nodeValue !== "Tap connect") throw new Error("idle branch text was wrong");

const idleConnectButton = buttonOf(article);
idleConnectButton.click();
article = visibleArticle(section);
const liveArticle = article;
const liveLabel = textOf(article, "strong");
const liveDetail = textOf(article, "p");
if (host.children[0] !== section) throw new Error("outer section was replaced");
if (idleArticle.parentNode !== null) throw new Error("idle branch was not removed");
if (liveArticle.attributes["data-kind"] !== "live") throw new Error("live branch was not mounted");
if (section.attributes["data-connected"] !== "") throw new Error("connected attr was not set");
if (liveLabel.nodeValue !== "Live" || liveDetail.nodeValue !== "Heart-rate monitor connected") throw new Error("live branch text was wrong");

app.dispatch({{ kind: Symbol.for("rename"), label: "Recording" }});
article = visibleArticle(section);
if (article !== liveArticle) throw new Error("live branch was replaced during same-branch update");
if (textOf(article, "strong") !== liveLabel) throw new Error("live label node was replaced");
if (liveLabel.nodeValue !== "Recording") throw new Error("live branch text did not update");
const renameRootUpdate = devEvents.find((event) =>
  event.type === "template/update" &&
  event.name === "template0" &&
  event.changedPaths.join(",") === "state.label"
);
if (!renameRootUpdate) throw new Error("rename did not report a root template update");
if (!slotKinds(renameRootUpdate.updatedSlots).includes("conditional")) {{
  throw new Error("rename did not keep the conditional slot dirty for branch reads");
}}
if (!slotKinds(renameRootUpdate.skippedSlots).includes("attr:data-connected")) {{
  throw new Error("rename did not skip the unchanged connected attr");
}}

const stateUpdatesBeforeStaleClick = devEvents.filter((event) => event.type === "state/update").length;
idleConnectButton.click();
if (visibleArticle(section) !== liveArticle) throw new Error("stale idle button disturbed the active branch");
if (app.state.label !== "Recording") throw new Error("stale idle button dispatched after branch disposal");
if (liveLabel.nodeValue !== "Recording") throw new Error("stale idle button updated active branch text");
const stateUpdatesAfterStaleClick = devEvents.filter((event) => event.type === "state/update").length;
if (stateUpdatesAfterStaleClick !== stateUpdatesBeforeStaleClick) {{
  throw new Error("stale idle button emitted a state update after branch disposal");
}}

buttonOf(article).click();
article = visibleArticle(section);
if (article === liveArticle) throw new Error("live branch should be removed after disconnect");
if (liveArticle.parentNode !== null) throw new Error("live branch was not detached");
if (article.attributes["data-kind"] !== "idle") throw new Error("idle branch was not remounted");
if (section.hasAttribute("data-connected")) throw new Error("connected attr was not removed");
if (textOf(article, "em").nodeValue !== "Idle") throw new Error("idle label after disconnect was wrong");
if (textOf(article, "p").nodeValue !== "Disconnected") throw new Error("idle detail after disconnect was wrong");

const metadataComponent = mod.view(mod.init);
const metadataSlotKinds = metadataComponent.definition.slots.map((slot) => JSON.stringify(slot.kind)).join("|");
if (!metadataSlotKinds.includes("conditional")) throw new Error(`conditional slot metadata missing: ${{metadataSlotKinds}}`);
const conditionalSlot = metadataComponent.definition.slots.find((slot) => slot.kind?.conditional);
if (!conditionalSlot.reads.includes("state.label") || !conditionalSlot.reads.includes("state.detail")) {{
  throw new Error(`conditional slot did not include branch reads: ${{conditionalSlot.reads.join(",")}}`);
}}

function visibleArticle(parent) {{
  return parent.children.find((node) => node.tagName === "article");
}}

function buttonOf(parent) {{
  return parent.children.find((node) => node.tagName === "button");
}}

function textOf(parent, tagName) {{
  const child = parent.children.find((node) => node.tagName === tagName);
  return child.children.find((node) => "nodeValue" in node);
}}

function slotKinds(slots) {{
  return slots.map((slot) => {{
    if (slot.kind === "text") return "text";
    if (slot.kind?.attr) return `attr:${{slot.kind.attr}}`;
    if (slot.kind?.conditional) return "conditional";
    return JSON.stringify(slot.kind);
  }});
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated conditional view failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_detail_tabs_app_swaps_panes_and_reuses_shell() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-detail-tabs-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_detail_tabs_app.clsk");
    let output = temp_dir.join("detail-tabs-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{ root: host, init: mod.init, update: mod.update, view: mod.view }});

const main = host.children.find((node) => node.tagName === "main");
const nav = childByAttr(main, "nav", "data-tabs", "detail");
const tabs = nav.children.filter((node) => node.tagName === "button");
const stats = childByAttr(main, "section", "data-stats", "summary");
const statCards = stats.children.filter((node) => node.tagName === "article");
const statTexts = statCards.map((card) => textOf(card, "strong"));
let body = bodyOf(main);
const liveBody = body;
let livePane = paneOf(body);

if (main.attributes["data-view"] !== "live") throw new Error("initial detail view should be live");
if (main.hasAttribute("data-mobile")) throw new Error("mobile attr should start absent");
if (main.attributes["data-readings"] !== "3") throw new Error("initial reading count attr was wrong");
if (!tabs[0].hasAttribute("data-selected") || tabs[1].hasAttribute("data-selected") || tabs[2].hasAttribute("data-selected")) {{
  throw new Error("initial selected tab attrs were wrong");
}}
if (stats.attributes["data-selected-label"] !== "Warmup walk") throw new Error("initial selected log label was wrong");
expectStatValues(statTexts, ["04:30", "96", "124", "149"], "initial live stats");
if (livePane.attributes["data-pane"] !== "live") throw new Error("live pane was not mounted");
if (livePane.attributes["data-points"] !== "3") throw new Error("live pane point count was wrong");

tabs[1].click();
body = bodyOf(main);
let logBody = body;
let logPane = paneOf(body);
const warmupDetail = childByAttr(logPane, "article", "data-detail", "warmup");
const warmupDetailTitle = textOf(warmupDetail, "h2");
let logButtons = logButtonsOf(logPane);

if (host.children.find((node) => node.tagName === "main") !== main) throw new Error("app shell was replaced after log tab");
if (childByAttr(main, "nav", "data-tabs", "detail") !== nav) throw new Error("tab nav was replaced after log tab");
if (nav.children.filter((node) => node.tagName === "button")[1] !== tabs[1]) throw new Error("tab button component was replaced");
if (childByAttr(main, "section", "data-stats", "summary") !== stats) throw new Error("stats section was replaced");
if (stats.children.filter((node) => node.tagName === "article")[0] !== statCards[0]) throw new Error("stat component root was replaced");
if (textOf(statCards[0], "strong") !== statTexts[0]) throw new Error("stat value text node was replaced");
if (liveBody.parentNode !== null) throw new Error("live body should be detached after log tab");
if (main.attributes["data-view"] !== "log") throw new Error("log body was not active");
if (tabs[0].hasAttribute("data-selected") || !tabs[1].hasAttribute("data-selected") || tabs[2].hasAttribute("data-selected")) {{
  throw new Error("selected tab attrs were wrong after log tab");
}}
if (logPane.attributes["data-pane"] !== "log" || logPane.attributes["data-selected"] !== "warmup") throw new Error("log pane attrs were wrong");
if (logButtons.length !== 2) throw new Error(`hidden entries should stay out of the log list, found ${{logButtons.length}}`);
if (!logButtons[0].hasAttribute("data-selected") || logButtons[1].hasAttribute("data-selected")) throw new Error("initial log selection attrs were wrong");
expectStatValues(statTexts, ["12:00", "93", "118", "132"], "warmup stats");
if (warmupDetailTitle.nodeValue !== "Warmup walk") throw new Error("warmup detail title was wrong");

const firstLogButtons = logButtons;
logButtons[1].click();
body = bodyOf(main);
if (body !== logBody) throw new Error("log body was replaced during log selection");
logPane = paneOf(body);
logButtons = logButtonsOf(logPane);
const intervalsDetail = childByAttr(logPane, "article", "data-detail", "intervals");

if (paneOf(body) !== logPane) throw new Error("log pane lookup failed after selection");
if (logButtons[0] !== firstLogButtons[0] || logButtons[1] !== firstLogButtons[1]) throw new Error("keyed log buttons were replaced after selection");
if (childByAttr(main, "section", "data-stats", "summary") !== stats) throw new Error("stats section was replaced after log selection");
if (textOf(statCards[0], "strong") !== statTexts[0]) throw new Error("stat text node was replaced after log selection");
if (warmupDetailTitle.nodeValue !== "Short intervals") throw new Error("detail title text did not update in place");
if (intervalsDetail !== warmupDetail) throw new Error("same log detail branch should update in place");
if (logPane.attributes["data-selected"] !== "intervals") throw new Error("selected log attr did not update");
if (logButtons[0].hasAttribute("data-selected") || !logButtons[1].hasAttribute("data-selected")) throw new Error("log row selected attrs did not update");
if (stats.attributes["data-selected-label"] !== "Short intervals") throw new Error("selected label attr did not update");
expectStatValues(statTexts, ["18:30", "104", "151", "176"], "interval stats");
if (app.state.selectedLogId !== "intervals" || app.state.message !== "Selected intervals") throw new Error("select-log message did not update app state");

tabs[2].click();
body = bodyOf(main);
const metricsBody = body;
const metricsPane = paneOf(body);
const metricCards = metricsPane.children.filter((node) => node.tagName === "article");

if (main.attributes["data-view"] !== "metrics") throw new Error("metrics body was not active");
if (logBody.parentNode !== null) throw new Error("log body should be detached after metrics tab");
if (childByAttr(main, "nav", "data-tabs", "detail") !== nav) throw new Error("nav was replaced after metrics tab");
if (childByAttr(main, "section", "data-stats", "summary") !== stats) throw new Error("stats section was replaced after metrics tab");
if (tabs[0].hasAttribute("data-selected") || tabs[1].hasAttribute("data-selected") || !tabs[2].hasAttribute("data-selected")) {{
  throw new Error("selected tab attrs were wrong after metrics tab");
}}
if (metricsPane.attributes["data-pane"] !== "metrics" || metricsPane.attributes["data-count"] !== "2") throw new Error("metrics pane attrs were wrong");
if (metricCards.length !== 2 || metricCards[0].attributes["data-metric"] !== "warmup" || metricCards[1].attributes["data-metric"] !== "intervals") {{
  throw new Error("metrics pane should render visible entries only");
}}
expectStatValues(statTexts, ["04:30", "96", "124", "149"], "metrics live stats");

const mobileButton = childByAttr(main, "button", "data-action", "mobile");
mobileButton.click();
if (host.children.find((node) => node.tagName === "main") !== main) throw new Error("main was replaced after mobile toggle");
if (main.attributes["data-mobile"] !== "") throw new Error("mobile attr was not set");
if (bodyOf(main) !== metricsBody) throw new Error("metrics body should update in place during mobile toggle");
if (paneOf(bodyOf(main)) !== metricsPane) throw new Error("metrics pane should update in place during mobile toggle");

tabs[0].click();
body = bodyOf(main);
livePane = paneOf(body);
if (main.attributes["data-view"] !== "live") throw new Error("live body was not restored");
if (metricsBody.parentNode !== null) throw new Error("metrics body should be detached after live tab");
if (childByAttr(main, "nav", "data-tabs", "detail") !== nav) throw new Error("nav was replaced after returning live");
if (childByAttr(main, "section", "data-stats", "summary") !== stats) throw new Error("stats section was replaced after returning live");
if (!tabs[0].hasAttribute("data-selected") || tabs[1].hasAttribute("data-selected") || tabs[2].hasAttribute("data-selected")) {{
  throw new Error("selected tab attrs were wrong after returning live");
}}
expectStatValues(statTexts, ["04:30", "96", "124", "149"], "returned live stats");
if (livePane.attributes["data-pane"] !== "live" || livePane.attributes["data-points"] !== "3") throw new Error("live pane attrs were wrong after return");

const metadataComponent = mod.view(mod.init);
const slotKinds = metadataComponent.definition.slots.map((slot) => JSON.stringify(slot.kind)).join("|");
if (!slotKinds.includes("tab-button") || !slotKinds.includes("stat-tile") || !slotKinds.includes("conditional")) {{
  throw new Error(`detail tab component metadata missing expected slots: ${{slotKinds}}`);
}}

function bodyOf(parent) {{
  return parent.children.find((node) => node.attributes?.["data-pane"] !== undefined);
}}

function paneOf(parent) {{
  if (parent.attributes?.["data-pane"] !== undefined) return parent;
  return parent.children.find((node) => node.attributes?.["data-pane"] !== undefined);
}}

function logButtonsOf(parent) {{
  return childByAttr(parent, "div", "data-list", "exercise-log").children.filter((node) => node.tagName === "button");
}}

function childByAttr(parent, tagName, name, value) {{
  return parent.children.find((node) => node.tagName === tagName && node.attributes?.[name] === value);
}}

function textOf(parent, tagName) {{
  const child = parent.children.find((node) => node.tagName === tagName);
  return child.children.find((node) => "nodeValue" in node);
}}

function expectStatValues(textNodes, expected, label) {{
  const actual = textNodes.map((node) => node.nodeValue);
  if (actual.join(",") !== expected.join(",")) {{
    throw new Error(`${{label}} were wrong: ${{actual.join(",")}}`);
  }}
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated detail tabs app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_component_view_reuses_nested_component() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-component-view-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_component_view.clsk");
    let output = temp_dir.join("component-view.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: (event) => devEvents.push(event)
}});

const section = host.children[0];
let article = articleOf(section);
const initialArticle = article;
const h1Text = textOf(section, "h1");
const labelText = textOf(article, "strong");
const valueText = textOf(article, "span");

if (section.attributes["data-status"] !== "Idle") throw new Error("initial status attr was wrong");
if (h1Text.nodeValue !== "Idle") throw new Error("initial heading was wrong");
if (labelText.nodeValue !== "Resting" || valueText.nodeValue !== "64 bpm") throw new Error("initial component text was wrong");

buttonOf(section, 2).click();
article = articleOf(section);
if (host.children[0] !== section) throw new Error("parent section was replaced after pulse");
if (article !== initialArticle) throw new Error("child component root was replaced after pulse");
if (textOf(article, "strong") !== labelText) throw new Error("child label node was replaced after pulse");
if (textOf(article, "span") !== valueText) throw new Error("child value node was replaced after pulse");
if (section.attributes["data-status"] !== "Idle") throw new Error("status attr changed during pulse");
if (h1Text.nodeValue !== "Idle") throw new Error("heading changed during pulse");
if (labelText.nodeValue !== "Resting" || valueText.nodeValue !== "68 bpm") throw new Error("component value-only prop did not update");

const valueOnlyUpdate = devEvents.find((event) =>
  event.type === "template/update" &&
  event.localChangedPaths?.join(",") === "summary.value" &&
  hasReadIn(event.updatedSlots, "summary.value") &&
  hasReadIn(event.skippedSlots, "summary.label")
);
if (!valueOnlyUpdate) {{
  throw new Error("child component did not skip the unchanged summary.label slot for a value-only prop update");
}}
if (hasReadIn(valueOnlyUpdate.updatedSlots, "summary.label")) {{
  throw new Error("summary.label updated even though only summary.value changed");
}}

buttonOf(section, 0).click();
article = articleOf(section);
if (host.children[0] !== section) throw new Error("parent section was replaced");
if (article !== initialArticle) throw new Error("child component root was replaced");
if (textOf(article, "strong") !== labelText) throw new Error("child label node was replaced");
if (textOf(article, "span") !== valueText) throw new Error("child value node was replaced");
if (section.attributes["data-status"] !== "Live") throw new Error("status attr did not update");
if (h1Text.nodeValue !== "Live") throw new Error("heading did not update");
if (labelText.nodeValue !== "Workout" || valueText.nodeValue !== "142 bpm") throw new Error("component props did not update");

buttonOf(section, 1).click();
article = articleOf(section);
if (article !== initialArticle) throw new Error("child component root was replaced after reset");
if (section.attributes["data-status"] !== "Idle") throw new Error("status attr did not reset");
if (h1Text.nodeValue !== "Idle") throw new Error("heading did not reset");
if (labelText.nodeValue !== "Resting" || valueText.nodeValue !== "64 bpm") throw new Error("component props did not reset");

const metadataComponent = mod.view(mod.init);
const slotKinds = metadataComponent.definition.slots.map((slot) => JSON.stringify(slot.kind)).join("|");
if (!slotKinds.includes("summary-card")) throw new Error(`component slot metadata missing: ${{slotKinds}}`);
const summaryComponent = mod.summary_card(mod.init.summary);
if (summaryComponent.definition.params?.join(",") !== "summary") {{
  throw new Error(`component parameter metadata was missing: ${{summaryComponent.definition.params}}`);
}}

function articleOf(parent) {{
  return parent.children.find((node) => node.tagName === "article");
}}

function buttonOf(parent, index) {{
  return parent.children.filter((node) => node.tagName === "button")[index];
}}

function textOf(parent, tagName) {{
  const child = parent.children.find((node) => node.tagName === tagName);
  return child.children.find((node) => "nodeValue" in node);
}}

function hasReadIn(slots, read) {{
  return (slots || []).some((slot) => (slot.reads || []).includes(read));
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated component view failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_canvas_ref_app_draws_through_runtime_handler() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-canvas-ref-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_canvas_ref_app.clsk");
    let output = temp_dir.join("canvas-ref-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click" }});
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
    this.width = 0;
    this.height = 0;
    this.calls = [];
    this.context = new CanvasContext(this.calls);
  }}
  getContext(kind) {{
    if (kind !== "2d") return null;
    return this.context;
  }}
}}

class CanvasContext {{
  constructor(calls) {{
    this.calls = calls;
  }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, ...args]); }}
  strokeRect(...args) {{ this.calls.push(["strokeRect", this.strokeStyle, this.lineWidth, ...args]); }}
  beginPath() {{ this.calls.push(["beginPath"]); }}
  moveTo(...args) {{ this.calls.push(["moveTo", ...args]); }}
  lineTo(...args) {{ this.calls.push(["lineTo", ...args]); }}
  arc(...args) {{ this.calls.push(["arc", ...args]); }}
  stroke() {{ this.calls.push(["stroke", this.strokeStyle, this.lineWidth]); }}
  fill() {{ this.calls.push(["fill", this.fillStyle]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, ...args]); }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
const canvas = section.children.find((node) => node.tagName === "canvas");
const button = section.children.find((node) => node.tagName === "button");
const span = section.children.find((node) => node.tagName === "span");
const text = span.children.find((node) => "nodeValue" in node);

if (canvas.attributes.ref !== undefined) throw new Error("ref should not be emitted as a DOM attribute");
if (app.getRef("heart-chart") !== canvas) throw new Error("canvas ref was not registered");
if (canvas.width !== 120 || canvas.height !== 60) throw new Error("canvas command did not set dimensions");
if (app.commands.length !== 1 || app.commands[0].kind !== "canvas/draw") throw new Error("initial canvas command was not logged");
if (!canvas.calls.some((call) => call[0] === "stroke")) throw new Error("initial draw did not stroke the chart path");
if (!canvas.calls.some((call) => call[0] === "fillText" && call[3] === "HR")) throw new Error("initial draw did not write chart label");
if (app.state.status !== "Drawn 1" || text.nodeValue !== "Drawn 1") throw new Error("draw completion message did not update state");

const initialCanvas = canvas;
const initialText = text;
button.click();
if (section.children.find((node) => node.tagName === "canvas") !== initialCanvas) throw new Error("canvas node was replaced");
if (span.children.find((node) => "nodeValue" in node) !== initialText) throw new Error("status text node was replaced");
if (app.getRef("heart-chart") !== initialCanvas) throw new Error("canvas ref changed after update");
if (app.commands.length !== 2 || app.commands[1].kind !== "canvas/draw") throw new Error("button draw command was not logged");
if (app.state.status !== "Drawn 2" || initialText.nodeValue !== "Drawn 2") throw new Error("second draw completion did not update state");
if (!canvas.calls.some((call) => call[0] === "fillRect" && call[1] === "#d9184b")) throw new Error("second draw did not fill the HR bar");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated canvas ref app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_canvas_dpr_app_scales_backing_store() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-canvas-dpr-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_canvas_dpr_app.clsk");
    let output = temp_dir.join("canvas-dpr-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = {{}};
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
    this.width = 0;
    this.height = 0;
    this.calls = [];
    this.context = new CanvasContext(this.calls);
  }}
  getContext(kind) {{
    if (kind !== "2d") return null;
    return this.context;
  }}
}}

class CanvasContext {{
  constructor(calls) {{
    this.calls = calls;
    this.fillStyle = "#000";
    this.strokeStyle = "#000";
    this.lineWidth = 1;
    this.lineCap = "butt";
    this.lineJoin = "miter";
    this.font = "";
  }}
  setTransform(...args) {{ this.calls.push(["setTransform", ...args]); }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, ...args]); }}
  beginPath() {{ this.calls.push(["beginPath"]); }}
  moveTo(...args) {{ this.calls.push(["moveTo", ...args]); }}
  lineTo(...args) {{ this.calls.push(["lineTo", ...args]); }}
  stroke() {{ this.calls.push(["stroke", this.strokeStyle, this.lineWidth, this.lineCap, this.lineJoin]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, ...args]); }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ devicePixelRatio: 2 }})
}});

const section = host.children[0];
const canvas = section.children.find((node) => node.tagName === "canvas");
const button = section.children.find((node) => node.tagName === "button");
const paragraph = section.children.find((node) => node.tagName === "p");
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.getRef("dpr-chart") !== canvas) throw new Error("DPR chart ref was not registered");
if (app.commands.length !== 1 || app.commands[0].kind !== "canvas/draw") throw new Error("initial DPR canvas command was not logged");
if (canvas.width !== 480 || canvas.height !== 240) throw new Error(`backing dimensions were not scaled: ${{canvas.width}}x${{canvas.height}}`);
if (app.state.backingWidth !== 480 || app.state.backingHeight !== 240) throw new Error("scaled backing size was not returned");
if (app.state.cssWidth !== 240 || app.state.cssHeight !== 120 || app.state.pixelRatio !== 2) throw new Error("CSS size or DPR result was wrong");
if (section.attributes["data-backing-width"] !== "480" || section.attributes["data-css-width"] !== "240") {{
  throw new Error("DPR sizing attrs were not updated");
}}
if (statusText.nodeValue !== "Drawn 480x240") throw new Error("initial DPR status was wrong");
if (!canvas.calls.some((call) => call[0] === "setTransform" && call[1] === 2 && call[4] === 2)) {{
  throw new Error("canvas context was not transformed by DPR");
}}
if (!canvas.calls.some((call) => call[0] === "clearRect" && call[3] === 240 && call[4] === 120)) {{
  throw new Error("clear op should use CSS dimensions under DPR transform");
}}
if (!canvas.calls.some((call) => call[0] === "fillRect" && call[1] === "#fffdfa" && call[4] === 240 && call[5] === 120)) {{
  throw new Error("fill op should use CSS dimensions under DPR transform");
}}
if (!canvas.calls.some((call) => call[0] === "fillText" && call[1] === "#172019" && call[2] === "700 12px system-ui" && call[3] === "DPR")) {{
  throw new Error("initial DPR label was not drawn");
}}

const initialCanvas = canvas;
const initialText = statusText;
button.click();
if (section.children.find((node) => node.tagName === "canvas") !== initialCanvas) throw new Error("DPR canvas node was replaced");
if (paragraph.children.find((node) => "nodeValue" in node) !== initialText) throw new Error("DPR status text node was replaced");
if (app.commands.length !== 2 || app.commands[1].kind !== "canvas/draw") throw new Error("redraw command was not logged");
if (app.state.draws !== 2 || app.state.backingWidth !== 480 || app.state.pixelRatio !== 2) throw new Error("redraw state was wrong");
if (!canvas.calls.some((call) => call[0] === "fillText" && call[3] === "DPR 2")) {{
  throw new Error("redraw did not use returned DPR state");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated canvas DPR app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_canvas_label_measure_app_measures_before_draw() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-canvas-label-measure-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_canvas_label_measure_app.clsk");
    let output = temp_dir.join("canvas-label-measure-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
    this.width = 0;
    this.height = 0;
    this.calls = [];
    this.context = new CanvasContext(this.calls);
  }}
  getContext(kind) {{
    if (kind !== "2d") return null;
    return this.context;
  }}
}}

class CanvasContext {{
  constructor(calls) {{
    this.calls = calls;
    this.fillStyle = "#000";
    this.strokeStyle = "#000";
    this.lineWidth = 1;
    this.textAlign = "start";
    this.textBaseline = "alphabetic";
    this.font = "";
  }}
  measureText(text) {{
    this.calls.push(["measureText", this.font, text]);
    return {{
      width: String(text).length * 8,
      actualBoundingBoxLeft: 1,
      actualBoundingBoxRight: String(text).length * 8 - 1,
      actualBoundingBoxAscent: 9,
      actualBoundingBoxDescent: 3
    }};
  }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, ...args]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, this.textAlign, this.textBaseline, ...args]); }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
const canvas = section.children.find((node) => node.tagName === "canvas");
const paragraph = section.children.find((node) => node.tagName === "p");
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.getRef("metrics-chart") !== canvas) throw new Error("metrics chart ref was not registered");
if (app.commands.length !== 2) throw new Error(`expected measure and draw commands, found ${{app.commands.length}}`);
if (app.commands[0].kind !== "canvas/measure-text") throw new Error("first command should measure text");
if (app.commands[1].kind !== "canvas/draw") throw new Error("second command should draw measured labels");
if (!canvas.calls.some((call) => call[0] === "measureText" && call[1] === "700 12px system-ui" && call[2] === "Zone 2 adherence")) {{
  throw new Error("zone label was not measured with requested font");
}}
if (!canvas.calls.some((call) => call[0] === "measureText" && call[2] === "Training load")) {{
  throw new Error("training load label was not measured");
}}
if (app.state.zoneWidth !== 128 || app.state.loadWidth !== 104) throw new Error("measured widths were not stored");
if (app.state["stacked?"] !== true) throw new Error("labels should stack when measured widths collide");
if (app.state.draws !== 1) throw new Error("draw completion did not increment draw count");
if (section.attributes["data-zone-width"] !== "128" || section.attributes["data-load-width"] !== "104") {{
  throw new Error("measured widths did not render to attrs");
}}
if (section.attributes["data-stacked"] !== "") throw new Error("stacked attr was not set");
if (statusText.nodeValue !== "Stacked labels drawn") throw new Error("measured label status was wrong");
if (canvas.width !== 260 || canvas.height !== 120) throw new Error("canvas dimensions were not applied for label chart");
if (!canvas.calls.some((call) => call[0] === "fillText" && call[5] === "Zone 2 adherence" && call[6] === 14 && call[7] === 22)) {{
  throw new Error("zone label draw call was wrong");
}}
if (!canvas.calls.some((call) => call[0] === "fillText" && call[3] === "left" && call[5] === "Training load" && call[6] === 14 && call[7] === 46)) {{
  throw new Error("stacked training-load label draw call was wrong");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated canvas label measure app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_event_payload_app_reads_dom_events() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-event-payload-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_event_payload_app.clsk");
    let output = temp_dir.join("event-payload-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.value = "";
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "value") this.value = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  input(value) {{
    this.value = value;
    this.emit("input", {{ type: "input", currentTarget: this, target: this }});
  }}
  change(value) {{
    this.value = value;
    this.emit("change", {{ type: "change", currentTarget: this, target: this }});
  }}
  keydown(key) {{
    this.emit("keydown", {{ type: "keydown", key, currentTarget: this, target: this }});
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{ root: host, init: mod.init, update: mod.update, view: mod.view }});

const section = host.children[0];
const input = section.children.find((node) => node.tagName === "input");
const select = section.children.find((node) => node.tagName === "select");
const button = section.children.find((node) => node.tagName === "button");
const paragraph = section.children.find((node) => node.tagName === "p");
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.getRef("exercise-type") !== input) throw new Error("input ref was not registered");
if (input.value !== "" || select.value !== "week") throw new Error("initial form values were wrong");
if (statusText.nodeValue !== "Editing") throw new Error("initial status was wrong");

input.input("LISS");
if (app.state.draft !== "LISS") throw new Error("input event did not update draft");
if (section.children.find((node) => node.tagName === "input") !== input) throw new Error("input node was replaced");
if (paragraph.children.find((node) => "nodeValue" in node) !== statusText) throw new Error("status text was replaced after input");

input.keydown("Escape");
if (app.state.confirmed !== "") throw new Error("non-Enter key should not confirm");

select.change("month");
if (app.state.grouping !== "month") throw new Error("select change did not update grouping");
if (section.attributes["data-grouping"] !== "month") throw new Error("grouping attr did not update");

input.keydown("Enter");
if (app.state.confirmed !== "LISS") throw new Error("Enter key did not confirm draft");
if (statusText.nodeValue !== "Saved LISS (month)") throw new Error("confirm status was wrong");
if (app.commands.length !== 0) throw new Error("event payload updates should only emit Cmd.none");

input.input("Tempo");
button.click();
if (app.state.confirmed !== "Tempo") throw new Error("button confirm did not use latest draft");
if (statusText.nodeValue !== "Saved Tempo (month)") throw new Error("button status was wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated event payload app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_log_type_filter_app_filters_and_edits_types() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-log-type-filter-app-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_log_type_filter_app.clsk");
    let output = temp_dir.join("log-type-filter-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.value = "";
    this.focusCount = 0;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "value") this.value = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "value") this.value = "";
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  change(value) {{
    this.value = value;
    this.emit("change", {{ type: "change", currentTarget: this, target: this }});
  }}
  input(value) {{
    this.value = value;
    this.emit("input", {{ type: "input", currentTarget: this, target: this }});
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
  focus() {{
    this.focusCount += 1;
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function descendants(node, tagName) {{
  const matches = [];
  for (const child of node.children || []) {{
    if (child.tagName === tagName) matches.push(child);
    matches.push(...descendants(child, tagName));
  }}
  return matches;
}}

function childElements(node, tagName) {{
  return (node.children || []).filter((child) => child.tagName === tagName);
}}

function textContent(node) {{
  if ("nodeValue" in node) return node.nodeValue;
  return (node.children || []).map(textContent).join("");
}}

function optionValues(select) {{
  return childElements(select, "option").map((option) => option.attributes.value ?? "");
}}

function datalistOptionValues(section) {{
  const datalist = descendants(section, "datalist")[0];
  if (!datalist) return [];
  return childElements(datalist, "option").map((option) => option.attributes.value ?? "");
}}

function articles(section) {{
  return descendants(section, "article");
}}

function articleById(section, id) {{
  return articles(section).find((article) => article.attributes["data-id"] === id);
}}

function assertList(actual, expected, label) {{
  const got = actual.join(",");
  const want = expected.join(",");
  if (got !== want) throw new Error(label + " expected " + want + " found " + got);
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
let select = descendants(section, "select")[0];
const initialSelect = select;
if (section.attributes["data-filter"] !== "") throw new Error("initial filter attr was wrong");
if (section.attributes["data-count"] !== "5") throw new Error("initial visible count was wrong");
if (section.attributes["data-status"] !== "Ready") throw new Error("initial status attr was wrong");
if (select.value !== "") throw new Error("initial select value was wrong");
assertList(optionValues(select), ["", "__untyped__", "HIIT", "LISS", "Strength"], "initial options");
assertList(articles(section).map((article) => article.attributes["data-id"]), ["warmup", "lift", "untagged", "intervals", "jog"], "initial rows");
if (articleById(section, "untagged").attributes["data-type"] !== "Untyped") throw new Error("missing type label was wrong");
if (textContent(articleById(section, "jog")).indexOf("LISS") === -1) throw new Error("trimmed type label did not render");
if (descendants(section, "datalist").length !== 0) throw new Error("datalist should only render while editing");
if (app.commands.length !== 0) throw new Error("initial render should not emit commands");

select.change("__untyped__");
if (host.children[0] !== section) throw new Error("section was replaced after filtering");
if (descendants(section, "select")[0] !== initialSelect) throw new Error("select was replaced after filtering");
if (section.attributes["data-filter"] !== "__untyped__") throw new Error("untyped filter attr was wrong");
if (section.attributes["data-count"] !== "1") throw new Error("untyped visible count was wrong");
if (section.attributes["data-status"] !== "Showing untyped") throw new Error("untyped status was wrong");
assertList(articles(section).map((article) => article.attributes["data-id"]), ["untagged"], "untyped rows");
if (app.commands.length !== 0) throw new Error("filtering should only emit Cmd.none");

select.change("LISS");
if (section.attributes["data-count"] !== "2") throw new Error("LISS visible count was wrong");
if (section.attributes["data-status"] !== "Showing LISS") throw new Error("LISS status was wrong");
assertList(articles(section).map((article) => article.attributes["data-id"]), ["warmup", "jog"], "LISS rows");
const warmupArticle = articleById(section, "warmup");
childElements(warmupArticle, "button")[0].click();
if (app.commands.length !== 1 || app.commands[0].kind !== "dom-ref/focus") throw new Error("edit should emit one focus command");
if (app.commands[0].command.ref !== "exercise-type") throw new Error("focus command ref was wrong");
if (articleById(section, "warmup") !== warmupArticle) throw new Error("keyed warmup row was replaced after edit");
const input = descendants(warmupArticle, "input")[0];
const label = descendants(warmupArticle, "label")[0];
if (!input) throw new Error("exercise type input did not render");
if (app.getRef("exercise-type") !== input) throw new Error("exercise type ref was not registered");
if (input.attributes.ref !== undefined) throw new Error("ref should not be emitted as an attribute");
if (input.value !== "LISS") throw new Error("draft value was not loaded from the row");
if (input.focusCount !== 1) throw new Error("runtime did not focus the type picker input");
if (section.attributes["data-status"] !== "Editing warmup") throw new Error("editing status was wrong");
assertList(datalistOptionValues(section), ["HIIT", "LISS", "Strength"], "datalist options");
if (childElements(descendants(section, "datalist")[0], "option").some((option) => option.children.length !== 0)) {{
  throw new Error("self-closing datalist options should not have children");
}}

input.input("Recovery");
if (app.state.typeDraft !== "Recovery") throw new Error("input event did not update the type draft");
if (descendants(warmupArticle, "input")[0] !== input) throw new Error("controlled input was replaced after draft edit");
if (input.value !== "Recovery") throw new Error("controlled input value was not synchronized");
if (app.commands.length !== 1) throw new Error("draft edit should only emit Cmd.none");
childElements(label, "button")[0].click();
if (app.commands.length !== 1) throw new Error("save should only emit Cmd.none");
if (app.state.entries[0].exerciseType !== "Recovery") throw new Error("saved type did not update the entry");
if (app.state.typePickerEntryId !== "") throw new Error("type picker did not close after save");
if (section.attributes["data-status"] !== "Saved Recovery") throw new Error("save status was wrong");
if (section.attributes["data-count"] !== "1") throw new Error("current LISS filter should only keep the jog row");
assertList(articles(section).map((article) => article.attributes["data-id"]), ["jog"], "rows after saving Recovery");
assertList(optionValues(select), ["", "__untyped__", "HIIT", "LISS", "Recovery", "Strength"], "options after saving Recovery");
if (descendants(section, "input").length !== 0) throw new Error("type picker input should be removed after save");

select = descendants(section, "select")[0];
select.change("Recovery");
if (section.attributes["data-filter"] !== "Recovery") throw new Error("Recovery filter attr was wrong");
if (section.attributes["data-count"] !== "1") throw new Error("Recovery visible count was wrong");
if (section.attributes["data-status"] !== "Showing Recovery") throw new Error("Recovery status was wrong");
const recoveryArticle = articleById(section, "warmup");
if (!recoveryArticle || recoveryArticle.attributes["data-type"] !== "Recovery") throw new Error("saved Recovery row did not render");
assertList(articles(section).map((article) => article.attributes["data-id"]), ["warmup"], "Recovery rows");
if (app.commands.length !== 1) throw new Error("Recovery filtering should not emit another command");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated log type filter app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_log_reconcile_app_keeps_selection_and_draft_valid() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-log-reconcile-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_log_reconcile_app.clsk");
    let output = temp_dir.join("log-reconcile-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.value = "";
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "value") this.value = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "value") this.value = "";
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of [...(this.listeners[name] || [])]) listener(event);
  }}
  change(value) {{
    this.value = value;
    this.emit("change", {{ type: "change", currentTarget: this, target: this }});
  }}
  input(value) {{
    this.value = value;
    this.emit("input", {{ type: "input", currentTarget: this, target: this }});
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function descendants(node, tagName) {{
  const matches = [];
  for (const child of node.children || []) {{
    if (child.tagName === tagName) matches.push(child);
    matches.push(...descendants(child, tagName));
  }}
  return matches;
}}

function childElements(node, tagName) {{
  return (node.children || []).filter((child) => child.tagName === tagName);
}}

function articles(section) {{
  return descendants(section, "article");
}}

function articleIds(section) {{
  return articles(section).map((article) => article.attributes["data-id"]);
}}

function articleById(section, id) {{
  return articles(section).find((article) => article.attributes["data-id"] === id);
}}

function optionValues(select) {{
  return childElements(select, "option").map((option) => option.attributes.value ?? "");
}}

function assertList(actual, expected, label) {{
  const got = actual.join(",");
  const want = expected.join(",");
  if (got !== want) throw new Error(`${{label}} expected ${{want}} found ${{got}}`);
}}

function selectedArticle(section) {{
  return articles(section).find((article) => article.hasAttribute("data-selected"));
}}

function editor(section) {{
  return descendants(section, "label")[0];
}}

function editorInput(section) {{
  return descendants(section, "input")[0];
}}

function buttonByAction(section, action) {{
  return descendants(section, "button").find((button) => button.attributes["data-action"] === action);
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
const initialSection = section;
const select = descendants(section, "select")[0];
const initialSelect = select;
if (section.attributes["data-selected"] !== "warmup") throw new Error("initial selected id was wrong");
if (section.attributes["data-draft"] !== "LISS") throw new Error("initial selected draft was wrong");
if (section.attributes["data-count"] !== "4") throw new Error("initial filtered count was wrong");
assertList(articleIds(section), ["warmup", "lift", "untagged", "intervals"], "initial rows");
assertList(optionValues(select), ["", "__untyped__", "HIIT", "LISS", "Strength"], "initial type options");
if (selectedArticle(section).attributes["data-id"] !== "warmup") throw new Error("initial selected row attr was wrong");
if (editor(section).attributes["data-editor"] !== "warmup" || editorInput(section).value !== "LISS") {{
  throw new Error("initial editor did not mirror selected log");
}}
if (app.commands.length !== 0) throw new Error("log reconcile app should not emit initial commands");

select.change("Strength");
if (host.children[0] !== initialSection) throw new Error("section root was replaced after filter");
if (descendants(section, "select")[0] !== initialSelect) throw new Error("select node was replaced after filter");
if (app.state.selectedLogId !== "lift" || app.state.editTypeDraft !== "Strength") {{
  throw new Error("Strength filter did not reconcile selected log and draft");
}}
if (section.attributes["data-filter"] !== "Strength" || section.attributes["data-count"] !== "1") {{
  throw new Error("Strength filter attrs were wrong");
}}
assertList(articleIds(section), ["lift"], "Strength rows");
if (selectedArticle(section).attributes["data-id"] !== "lift") throw new Error("Strength row was not selected");
if (editor(section).attributes["data-editor"] !== "lift" || editorInput(section).value !== "Strength") {{
  throw new Error("Strength editor did not mirror selected log");
}}

select.change("__untyped__");
if (app.state.selectedLogId !== "untagged" || app.state.editTypeDraft !== "") {{
  throw new Error("untyped filter did not reconcile selected log and empty draft");
}}
if (section.attributes["data-filter"] !== "__untyped__" || section.attributes["data-count"] !== "1") {{
  throw new Error("untyped filter attrs were wrong");
}}
assertList(articleIds(section), ["untagged"], "untyped rows");
if (selectedArticle(section).attributes["data-id"] !== "untagged") throw new Error("untyped row was not selected");
if (editor(section).attributes["data-editor"] !== "untagged" || editorInput(section).value !== "") {{
  throw new Error("untyped editor did not mirror selected log");
}}

const untypedInput = editorInput(section);
untypedInput.input("Tempo");
if (app.state.editTypeDraft !== "Tempo" || section.attributes["data-draft"] !== "Tempo") {{
  throw new Error("draft input did not update controlled draft state");
}}
if (editorInput(section) !== untypedInput) throw new Error("editor input was replaced during draft edit");
buttonByAction(section, "save").click();
if (app.state.entries.find((entry) => entry.id === "untagged").exerciseType !== "Tempo") {{
  throw new Error("saving selected type did not update the selected entry");
}}
if (app.state.selectedLogId !== "untagged" || app.state.editTypeDraft !== "Tempo") {{
  throw new Error("saving selected type should keep selected log even when current filter empties");
}}
if (section.attributes["data-count"] !== "0" || articles(section).length !== 0) {{
  throw new Error("untyped filter should be empty after assigning Tempo");
}}
if (editor(section).attributes["data-editor"] !== "untagged" || editorInput(section).value !== "Tempo") {{
  throw new Error("editor should still mirror selected log after filter empties");
}}
assertList(optionValues(select), ["", "__untyped__", "HIIT", "LISS", "Strength", "Tempo"], "options after saving Tempo");

select.change("HIIT");
if (app.state.selectedLogId !== "intervals" || app.state.editTypeDraft !== "HIIT") {{
  throw new Error("HIIT filter did not select the matching log");
}}
if (section.attributes["data-count"] !== "1" || selectedArticle(section).attributes["data-id"] !== "intervals") {{
  throw new Error("HIIT selected row was wrong");
}}
const intervalsArticle = selectedArticle(section);

buttonByAction(section, "hide").click();
if (app.state.entries.find((entry) => entry.id === "intervals").hiddenAt !== 4242) {{
  throw new Error("hide selected did not stamp hiddenAt");
}}
if (app.state.selectedLogId !== "warmup" || app.state.editTypeDraft !== "LISS") {{
  throw new Error("hiding selected HIIT row did not fall back to first visible log and draft");
}}
if (section.attributes["data-filter"] !== "HIIT" || section.attributes["data-count"] !== "0") {{
  throw new Error("HIIT filter should be empty after hiding the only HIIT log");
}}
if (articles(section).length !== 0) throw new Error("hidden HIIT row should be removed from filtered list");
if (intervalsArticle.parentNode !== null) throw new Error("hidden keyed article should be detached");
if (editor(section).attributes["data-editor"] !== "warmup" || editorInput(section).value !== "LISS") {{
  throw new Error("editor did not mirror fallback selected log");
}}

select.change("Strength");
if (app.state.selectedLogId !== "lift" || app.state.editTypeDraft !== "Strength") {{
  throw new Error("Strength filter after hide did not reconcile selected log");
}}
assertList(articleIds(section), ["lift"], "Strength rows after hide");
if (articleById(section, "lift") !== selectedArticle(section)) throw new Error("lift row was not selected after final filter");
if (app.commands.length !== 0) throw new Error("reconciliation workflow should only emit Cmd.none");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated log reconcile app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_zone_toggle_app_reads_checkbox_events() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-zone-toggle-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_zone_toggle_app.clsk");
    let output = temp_dir.join("zone-toggle-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.checked = false;
    this.value = "";
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "checked") this.checked = true;
    if (name === "value") this.value = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "checked") this.checked = false;
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  change(checked) {{
    this.checked = checked;
    this.emit("change", {{ type: "change", currentTarget: this, target: this }});
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{ root: host, init: mod.init, update: mod.update, view: mod.view }});

const section = host.children[0];
const labels = section.children.filter((node) => node.tagName === "label");
const zoneInput = labels[0].children.find((node) => node.tagName === "input");
const autosaveInput = labels[1].children.find((node) => node.tagName === "input");
const status = labels[0].children.find((node) => node.tagName === "span");
const statusText = status.children.find((node) => "nodeValue" in node);
const saveMode = labels[1].children.find((node) => node.tagName === "strong");
const saveModeText = saveMode.children.find((node) => "nodeValue" in node);

if (!zoneInput.checked) throw new Error("zones checkbox should start checked");
if (autosaveInput.checked) throw new Error("autosave checkbox should start unchecked");
if (section.attributes["data-zones-visible"] !== "") throw new Error("visible data attr should start set");
if (section.hasAttribute("data-autosave")) throw new Error("autosave data attr should start absent");
if (statusText.nodeValue !== "Zones visible") throw new Error("initial zone status was wrong");
if (saveModeText.nodeValue !== "Manual save") throw new Error("initial save mode was wrong");

zoneInput.change(false);
if (app.state["zonesVisible?"] !== false) throw new Error("checkbox event did not clear zone visibility");
if (section.hasAttribute("data-zones-visible")) throw new Error("visible data attr was not removed");
if (section.children.filter((node) => node.tagName === "label")[0].children.find((node) => node.tagName === "input") !== zoneInput) {{
  throw new Error("zone checkbox was replaced");
}}
if (status.children.find((node) => "nodeValue" in node) !== statusText) throw new Error("status text node was replaced");
if (zoneInput.checked !== false) throw new Error("zone checkbox property was not updated");
if (statusText.nodeValue !== "Zones hidden") throw new Error("zone status did not update");

autosaveInput.change(true);
if (app.state["autosave?"] !== true) throw new Error("checkbox event did not enable autosave");
if (section.attributes["data-autosave"] !== "") throw new Error("autosave data attr was not set");
if (autosaveInput.checked !== true) throw new Error("autosave checkbox property was not updated");
if (statusText.nodeValue !== "Auto save") throw new Error("autosave status did not update");
if (saveModeText.nodeValue !== "Auto save") throw new Error("autosave label did not update");

autosaveInput.change(false);
if (app.state["autosave?"] !== false) throw new Error("checkbox event did not disable autosave");
if (section.hasAttribute("data-autosave")) throw new Error("autosave data attr was not removed");
if (autosaveInput.checked !== false) throw new Error("autosave checkbox property did not clear");
if (statusText.nodeValue !== "Manual save") throw new Error("manual status did not update");
if (saveModeText.nodeValue !== "Manual save") throw new Error("manual label did not update");
if (app.commands.length !== 0) throw new Error("checkbox updates should only emit Cmd.none");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated zone toggle app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_zone_boundary_app_persists_numeric_edits() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-zone-boundary-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_zone_boundary_app.clsk");
    let output = temp_dir.join("zone-boundary-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = {{}};
    this.value = "";
    this.valueAsNumber = Number.NaN;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "value") {{
      this.value = String(value);
      this.valueAsNumber = Number(value);
    }}
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
  input(value) {{
    this.value = String(value);
    this.valueAsNumber = Number(value);
    this.emit("input", {{ type: "input", currentTarget: this, target: this }});
  }}
  keydown(key) {{
    const event = {{
      type: "keydown",
      key,
      currentTarget: this,
      target: this,
      defaultPrevented: false,
      preventDefault() {{
        this.defaultPrevented = true;
      }}
    }};
    this.emit("keydown", event);
    return event;
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function elements(node, tagName) {{
  return node.children.filter((child) => child.tagName === tagName);
}}

const storage = {{
  values: new Map(),
  getItem(key) {{
    return this.values.has(key) ? this.values.get(key) : null;
  }},
  setItem(key, value) {{
    this.values.set(key, value);
  }},
  removeItem(key) {{
    this.values.delete(key);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ storage }})
}});

const section = host.children[0];
const zoneStrip = elements(section, "div")[0];
const boundaryList = elements(section, "div")[1];
const firstButton = elements(zoneStrip, "button")[0];
const secondButton = elements(zoneStrip, "button")[1];
const firstBoundary = elements(boundaryList, "label")[0];
const firstInput = elements(firstBoundary, "input")[0];
const firstStrong = elements(firstBoundary, "strong")[0];
const firstValueText = firstStrong.children.find((node) => "nodeValue" in node);
const paragraph = elements(section, "p")[0];
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (section.attributes["data-target"] !== "2") throw new Error("initial target attr was wrong");
if (section.attributes["data-status"] !== "Ready") throw new Error("initial status attr was wrong");
if (secondButton.attributes["data-selected"] !== "") throw new Error("initial target button was not selected");
if (firstButton.hasAttribute("data-selected")) throw new Error("non-target button should not be selected");
if (firstButton.style.background !== "#2a9d8f") throw new Error("zone style object was not applied");
if (firstInput.valueAsNumber !== 110) throw new Error(`initial range value was wrong: ${{firstInput.valueAsNumber}}`);
if (firstInput.attributes.min !== "90" || firstInput.attributes.max !== "129") throw new Error("initial range bounds were wrong");
if (firstValueText.nodeValue !== "110") throw new Error("initial boundary label was wrong");
if (statusText.nodeValue !== "Ready") throw new Error("initial status text was wrong");

firstInput.input(126.6);
if (app.state.zones[0].max !== 127 || app.state.zones[1].min !== 128) throw new Error("range input did not round and update adjacent zones");
if (app.state.status !== "Zones saved") throw new Error("storage completion did not update status");
if (section.attributes["data-status"] !== "Zones saved") throw new Error("status attr did not update after save");
if (statusText.nodeValue !== "Zones saved") throw new Error("status text did not update after save");
if (firstInput.valueAsNumber !== 127) throw new Error(`range value was not patched to rounded boundary: ${{firstInput.valueAsNumber}}`);
if (firstValueText.nodeValue !== "127") throw new Error("boundary label did not update");
if (elements(boundaryList, "label")[0] !== firstBoundary) throw new Error("boundary label was replaced");
if (elements(firstBoundary, "input")[0] !== firstInput) throw new Error("range input was replaced");
if (firstStrong.children.find((node) => "nodeValue" in node) !== firstValueText) throw new Error("boundary text node was replaced");
if (app.commands.length !== 1 || app.commands[0].kind !== "storage/set") throw new Error("boundary edit command was not logged");
let stored = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (stored.zones[0].max !== 127 || stored.zones[1].min !== 128 || stored.targetZoneId !== 2) {{
  throw new Error("boundary edit was not persisted");
}}

firstButton.click();
if (app.state.targetZoneId !== 1) throw new Error("target-zone click did not update state");
if (section.attributes["data-target"] !== "1") throw new Error("target attr did not update");
if (firstButton.attributes["data-selected"] !== "") throw new Error("clicked target button was not selected");
if (secondButton.hasAttribute("data-selected")) throw new Error("previous target button remained selected");
if (elements(zoneStrip, "button")[0] !== firstButton) throw new Error("zone button was replaced");
if (app.commands.length !== 2 || app.commands[1].kind !== "storage/set") throw new Error("target change command was not logged");
stored = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (stored.targetZoneId !== 1 || stored.zones[0].max !== 127) throw new Error("target change was not persisted");

firstInput.input(500);
if (app.state.zones[0].max !== 129 || app.state.zones[1].min !== 130) throw new Error("range input did not clamp to right-zone max");
if (firstInput.valueAsNumber !== 129) throw new Error("range input did not patch clamped value");
stored = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (stored.zones[0].max !== 129 || stored.zones[1].min !== 130 || stored.targetZoneId !== 1) {{
  throw new Error("clamped boundary was not persisted");
}}

const ignoredKey = firstInput.keydown("Home");
if (ignoredKey.defaultPrevented) throw new Error("non-arrow key should not be prevented");
if (app.commands.length !== 3) throw new Error("non-arrow key should not emit a command");
if (app.state.zones[0].max !== 129 || firstInput.valueAsNumber !== 129) throw new Error("non-arrow key should not edit boundary");

const leftKey = firstInput.keydown("ArrowLeft");
if (!leftKey.defaultPrevented) throw new Error("ArrowLeft did not prevent default");
if (app.state.zones[0].max !== 128 || app.state.zones[1].min !== 129) throw new Error("ArrowLeft did not decrement boundary");
if (firstInput.valueAsNumber !== 128 || firstValueText.nodeValue !== "128") throw new Error("ArrowLeft did not patch range UI");
if (app.commands.length !== 4 || app.commands[3].kind !== "storage/set") throw new Error("ArrowLeft command was not logged");
stored = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (stored.zones[0].max !== 128 || stored.zones[1].min !== 129 || stored.targetZoneId !== 1) {{
  throw new Error("ArrowLeft boundary was not persisted");
}}

const rightKey = firstInput.keydown("ArrowUp");
if (!rightKey.defaultPrevented) throw new Error("ArrowUp did not prevent default");
if (app.state.zones[0].max !== 129 || app.state.zones[1].min !== 130) throw new Error("ArrowUp did not increment boundary");
if (firstInput.valueAsNumber !== 129 || firstValueText.nodeValue !== "129") throw new Error("ArrowUp did not patch range UI");
if (app.commands.length !== 5 || app.commands[4].kind !== "storage/set") throw new Error("ArrowUp command was not logged");
stored = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (stored.zones[0].max !== 129 || stored.zones[1].min !== 130 || stored.targetZoneId !== 1) {{
  throw new Error("ArrowUp boundary was not persisted");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated zone boundary app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_zone_drag_app_tracks_window_pointer_drag() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-zone-drag-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_zone_drag_app.clsk");
    let output = temp_dir.join("zone-drag-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.style = {{}};
    this.rect = {{
      x: 10,
      y: 0,
      width: 300,
      height: 32,
      top: 0,
      right: 310,
      bottom: 32,
      left: 10
    }};
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  pointerdown(clientX) {{
    const event = {{
      type: "pointerdown",
      clientX,
      clientY: 12,
      currentTarget: this,
      target: this,
      defaultPrevented: false,
      preventDefault() {{
        this.defaultPrevented = true;
      }}
    }};
    this.emit("pointerdown", event);
    return event;
  }}
  getBoundingClientRect() {{
    return this.rect;
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

class EventTarget {{
  constructor() {{
    this.listeners = {{}};
  }}
  addEventListener(name, listener, options) {{
    this.listeners[name] ||= [];
    this.listeners[name].push({{ listener, options }});
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((entry) => entry.listener !== listener);
  }}
  dispatch(name, event = {{}}) {{
    for (const entry of [...(this.listeners[name] || [])]) {{
      entry.listener({{ type: name, ...event }});
    }}
  }}
  count(name) {{
    return (this.listeners[name] || []).length;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function elements(node, tagName) {{
  return node.children.filter((child) => child.tagName === tagName);
}}

function textOf(node) {{
  return node.children.find((child) => "nodeValue" in child);
}}

function textContent(node) {{
  return (node.children || []).map((child) => "nodeValue" in child ? child.nodeValue : textContent(child)).join("");
}}

function buttonByAttr(parent, name, value) {{
  return elements(parent, "button").find((button) => button.attributes[name] === value);
}}

const storage = {{
  values: new Map(),
  getItem(key) {{
    return this.values.has(key) ? this.values.get(key) : null;
  }},
  setItem(key, value) {{
    this.values.set(key, value);
  }},
  removeItem(key) {{
    this.values.delete(key);
  }}
}};

const eventTarget = new EventTarget();
const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ storage, eventTarget, resizeTarget: eventTarget }})
}});

const section = host.children[0];
const track = app.getRef("zone-track");
const paragraph = elements(section, "p")[0];
const statusText = textOf(paragraph);
const firstHandle = buttonByAttr(track, "data-boundary", "1");

if (!track || track.attributes.class !== "zone-strip") throw new Error("zone track ref was not registered");
if (!firstHandle) throw new Error("first drag handle was not rendered");
if (app.commands.length !== 1 || app.commands[0].kind !== "dom-ref/resize-watch") throw new Error("initial resize watch was not logged");
if (eventTarget.count("resize") !== 1) throw new Error("resize fallback listener was not registered");
if (app.state.trackLeft !== 10 || app.state.trackWidth !== 300) throw new Error("track geometry was not stored");
if (section.attributes["data-track-width"] !== "300") throw new Error("track width attr was not updated");
if (firstHandle.style.left !== "33.33333333333333%") throw new Error(`initial handle style was wrong: ${{firstHandle.style.left}}`);

const down = firstHandle.pointerdown(160);
if (!down.defaultPrevented) throw new Error("pointerdown did not prevent default");
if (app.state.zones[0].max !== 120 || app.state.zones[1].min !== 121) throw new Error("pointerdown did not set the boundary");
if (app.state.draggingIndex !== 0) throw new Error("dragging index was not stored");
if (section.attributes["data-dragging"] !== "") throw new Error("dragging attr was not set");
if (statusText.nodeValue !== "Dragging 120") throw new Error("pointerdown status was wrong");
if (app.commands.length !== 5) throw new Error(`expected resize, storage, and three watch commands; found ${{app.commands.length}}`);
if (app.commands.slice(1).map((entry) => entry.kind).join(",") !== "storage/set,window/event-watch,window/event-watch,window/event-watch") {{
  throw new Error("drag start command sequence was wrong");
}}
if (eventTarget.count("pointermove") !== 1 || eventTarget.count("pointerup") !== 1 || eventTarget.count("pointercancel") !== 1) {{
  throw new Error("pointer listeners were not registered");
}}
let stored = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (stored.zones[0].max !== 120 || stored.zones[1].min !== 121 || stored.targetZoneId !== 2) {{
  throw new Error("pointerdown boundary was not persisted");
}}

const handleAfterDown = buttonByAttr(track, "data-boundary", "1");
if (handleAfterDown !== firstHandle) throw new Error("drag handle was replaced after pointerdown");
if (!textContent(firstHandle).includes("120")) throw new Error("handle label did not update after pointerdown");

eventTarget.dispatch("pointermove", {{ clientX: 210, clientY: 14, pointerId: 7, pointerType: "mouse", buttons: 1, isPrimary: true }});
if (app.state.zones[0].max !== 129 || app.state.zones[1].min !== 130) throw new Error("pointermove did not clamp and update boundary");
if (statusText.nodeValue !== "Dragging 129") throw new Error("pointermove status was wrong");
if (buttonByAttr(track, "data-boundary", "1") !== firstHandle) throw new Error("drag handle was replaced after pointermove");
if (!textContent(firstHandle).includes("129")) throw new Error("handle label did not update after pointermove");
if (app.commands.length !== 6 || app.commands[5].kind !== "storage/set") throw new Error("pointermove should persist one storage command");
stored = JSON.parse(storage.getItem("heartRateExercise.zones.v1"));
if (stored.zones[0].max !== 129 || stored.zones[1].min !== 130) throw new Error("pointermove boundary was not persisted");

eventTarget.dispatch("pointerup", {{ clientX: 210, clientY: 14, pointerId: 7, pointerType: "mouse" }});
if (app.state.draggingIndex !== null) throw new Error("dragging index was not cleared");
if (section.hasAttribute("data-dragging")) throw new Error("dragging attr was not removed");
if (statusText.nodeValue !== "Drag complete") throw new Error("drag end status was wrong");
if (app.commands.length !== 9) throw new Error(`expected three unwatch commands, found ${{app.commands.length}} total`);
if (eventTarget.count("pointermove") !== 0 || eventTarget.count("pointerup") !== 0 || eventTarget.count("pointercancel") !== 0) {{
  throw new Error("pointer listeners were not removed");
}}

eventTarget.dispatch("pointermove", {{ clientX: 20, buttons: 1 }});
if (app.commands.length !== 9) throw new Error("pointermove after cleanup emitted another command");
if (app.state.zones[0].max !== 129 || statusText.nodeValue !== "Drag complete") throw new Error("state changed after pointer cleanup");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated zone drag app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_canvas_measure_app_measures_ref_before_draw() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-canvas-measure-app-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_canvas_measure_app.clsk");
    let output = temp_dir.join("canvas-measure-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
    this.width = 0;
    this.height = 0;
    this.calls = [];
    this.context = new CanvasContext(this.calls);
  }}
  getBoundingClientRect() {{
    return {{
      x: 4,
      y: 8,
      width: 320.4,
      height: 180.2,
      top: 8,
      right: 324.4,
      bottom: 188.2,
      left: 4
    }};
  }}
  getContext(name) {{
    if (name !== "2d") return null;
    return this.context;
  }}
}}

class CanvasContext {{
  constructor(calls) {{
    this.calls = calls;
    this.fillStyle = "#000";
    this.strokeStyle = "#000";
    this.lineWidth = 1;
    this.lineCap = "butt";
    this.lineJoin = "miter";
    this.font = "";
  }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, ...args]); }}
  beginPath() {{ this.calls.push(["beginPath"]); }}
  moveTo(...args) {{ this.calls.push(["moveTo", ...args]); }}
  lineTo(...args) {{ this.calls.push(["lineTo", ...args]); }}
  stroke() {{ this.calls.push(["stroke", this.strokeStyle, this.lineWidth, this.lineCap, this.lineJoin]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, ...args]); }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function close(actual, expected) {{
  return Math.abs(actual - expected) < 0.000001;
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
const canvas = section.children.find((node) => node.tagName === "canvas");
const button = section.children.find((node) => node.tagName === "button");
const paragraph = section.children.find((node) => node.tagName === "p");
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.getRef("responsive-chart") !== canvas) throw new Error("responsive chart ref was not registered");
if (statusText.nodeValue !== "Ready") throw new Error("initial measured chart status was wrong");
if (app.commands.length !== 0) throw new Error("measure app should not emit an initial command");

button.click();
if (app.commands.length !== 2) throw new Error(`expected measure and draw commands, found ${{app.commands.length}}`);
if (app.commands[0].kind !== "dom-ref/measure") throw new Error("first command should measure the canvas ref");
if (app.commands[0].command.ref !== "responsive-chart") throw new Error("measure command ref was wrong");
if (app.commands[1].kind !== "canvas/draw") throw new Error("second command should draw after measuring");
if (!close(app.commands[1].command.width, 320.4) || !close(app.commands[1].command.height, 180.2)) {{
  throw new Error("canvas draw command did not use measured dimensions");
}}
if (!close(app.state.width, 320.4) || !close(app.state.height, 180.2)) throw new Error("measured dimensions were not stored");
if (app.state.draws !== 1) throw new Error("chart draw completion did not increment draw count");
if (statusText.nodeValue !== "Drawn 320x180") throw new Error("draw completion status was wrong");
if (section.attributes["data-width"] !== "320.4" || section.attributes["data-height"] !== "180.2") {{
  throw new Error("measured dimensions did not render to attrs");
}}
if (section.attributes["data-draws"] !== "1") throw new Error("draw count attr did not update");
if (canvas.width !== 320.4 || canvas.height !== 180.2) throw new Error("canvas dimensions were not set before draw");
if (!canvas.calls.some((call) => call[0] === "clearRect" && close(call[3], 320.4) && close(call[4], 180.2))) {{
  throw new Error("clear op did not use measured canvas size");
}}
if (!canvas.calls.some((call) => call[0] === "fillRect" && call[1] === "#fffdfa" && close(call[4], 320.4) && close(call[5], 180.2))) {{
  throw new Error("background fill did not use measured canvas size");
}}
if (!canvas.calls.some((call) => call[0] === "stroke" && call[1] === "#d9184b" && call[2] === 4 && call[3] === "round" && call[4] === "round")) {{
  throw new Error("stroke state was not applied");
}}
if (!canvas.calls.some((call) => call[0] === "fillText" && call[1] === "#172019" && call[2] === "700 12px system-ui" && call[3] === "320x180")) {{
  throw new Error("measured label was not drawn");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated canvas measure app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_canvas_measure_app_routes_missing_handler_to_on_error() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-canvas-measure-missing-handler-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_canvas_measure_app.clsk");
    let output = temp_dir.join("canvas-measure-missing-handler.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: (event) => devEvents.push(event)
}});

const section = host.children[0];
const button = section.children.find((node) => node.tagName === "button");
const paragraph = section.children.find((node) => node.tagName === "p");
const statusText = paragraph.children.find((node) => "nodeValue" in node);

button.click();
if (app.commands.length !== 1 || app.commands[0].kind !== "dom-ref/measure") throw new Error("missing handler command was not logged");
if (app.state.status !== "No handler registered for command kind dom-ref/measure") {{
  throw new Error(`missing handler did not route through onError: ${{app.state.status}}`);
}}
if (statusText.nodeValue !== "No handler registered for command kind dom-ref/measure") {{
  throw new Error("missing handler status did not render");
}}
if (section.attributes["data-status"] !== "No handler registered for command kind dom-ref/measure") {{
  throw new Error("missing handler status attr did not update");
}}
if (app.commands.length !== 1) throw new Error("missing handler should not emit follow-up commands");
const commandError = devEvents.find((event) => event.type === "command/error" && event.kind === "dom-ref/measure");
if (!commandError || commandError.error !== "No handler registered for command kind dom-ref/measure") {{
  throw new Error("devtools did not report missing command handler");
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated canvas measure missing-handler app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_canvas_resize_app_redraws_from_resize_observer() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-canvas-resize-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_canvas_resize_app.clsk");
    let output = temp_dir.join("canvas-resize-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  insertBefore(node, marker) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) return this.appendChild(node);
    this.children.splice(index, 0, node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) listener({{ type: "click", currentTarget: this, target: this }});
  }}
}}

class CanvasElement extends Element {{
  constructor() {{
    super("canvas");
    this.width = 0;
    this.height = 0;
    this.calls = [];
    this.context = new CanvasContext(this.calls);
    this.setRect(300, 160);
  }}
  setRect(width, height) {{
    this.rect = {{
      x: 0,
      y: 0,
      width,
      height,
      top: 0,
      right: width,
      bottom: height,
      left: 0
    }};
  }}
  getBoundingClientRect() {{
    return this.rect;
  }}
  getContext(name) {{
    if (name !== "2d") return null;
    return this.context;
  }}
}}

class CanvasContext {{
  constructor(calls) {{
    this.calls = calls;
    this.fillStyle = "#000";
    this.strokeStyle = "#000";
    this.lineWidth = 1;
    this.lineCap = "butt";
    this.lineJoin = "miter";
    this.font = "";
  }}
  clearRect(...args) {{ this.calls.push(["clearRect", ...args]); }}
  fillRect(...args) {{ this.calls.push(["fillRect", this.fillStyle, ...args]); }}
  strokeRect(...args) {{ this.calls.push(["strokeRect", this.strokeStyle, this.lineWidth, ...args]); }}
  beginPath() {{ this.calls.push(["beginPath"]); }}
  moveTo(...args) {{ this.calls.push(["moveTo", ...args]); }}
  lineTo(...args) {{ this.calls.push(["lineTo", ...args]); }}
  stroke() {{ this.calls.push(["stroke", this.strokeStyle, this.lineWidth, this.lineCap, this.lineJoin]); }}
  fillText(...args) {{ this.calls.push(["fillText", this.fillStyle, this.font, ...args]); }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

const observers = [];
class FakeResizeObserver {{
  constructor(callback) {{
    this.callback = callback;
    this.nodes = [];
    this.disconnected = false;
    observers.push(this);
  }}
  observe(node) {{
    this.nodes.push(node);
  }}
  unobserve(node) {{
    this.nodes = this.nodes.filter((item) => item !== node);
  }}
  disconnect() {{
    this.disconnected = true;
    this.nodes = [];
  }}
  trigger(node, rect) {{
    if (this.disconnected) return;
    this.callback([{{ target: node, contentRect: rect }}]);
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return tagName === "canvas" ? new CanvasElement() : new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

function close(actual, expected) {{
  return Math.abs(actual - expected) < 0.000001;
}}

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers({{ ResizeObserver: FakeResizeObserver }})
}});

const section = host.children[0];
const canvas = section.children.find((node) => node.tagName === "canvas");
const button = section.children.find((node) => node.tagName === "button");
const paragraph = section.children.find((node) => node.tagName === "p");
const statusText = paragraph.children.find((node) => "nodeValue" in node);

if (app.getRef("responsive-chart") !== canvas) throw new Error("responsive chart ref was not registered");
if (observers.length !== 1 || observers[0].nodes[0] !== canvas) throw new Error("resize observer did not watch the canvas");
if (app.commands.length !== 2) throw new Error(`expected initial watch and draw commands, found ${{app.commands.length}}`);
if (app.commands[0].kind !== "dom-ref/resize-watch") throw new Error("first command should watch the canvas ref");
if (app.commands[1].kind !== "canvas/draw") throw new Error("second command should draw after the initial resize message");
if (!close(app.state.width, 300) || !close(app.state.height, 160)) throw new Error("initial observed dimensions were not stored");
if (app.state.draws !== 1) throw new Error("initial draw completion did not increment draw count");
if (statusText.nodeValue !== "Drawn 300x160") throw new Error("initial draw status was wrong");
if (canvas.width !== 300 || canvas.height !== 160) throw new Error("initial canvas dimensions were not applied");
if (!canvas.calls.some((call) => call[0] === "fillText" && call[3] === "300x160")) {{
  throw new Error("initial canvas label was not drawn");
}}

const initialCanvas = canvas;
const initialText = statusText;
canvas.setRect(420, 210);
observers[0].trigger(canvas, canvas.rect);
if (section.children.find((node) => node.tagName === "canvas") !== initialCanvas) throw new Error("canvas node was replaced after resize");
if (paragraph.children.find((node) => "nodeValue" in node) !== initialText) throw new Error("status text node was replaced after resize");
if (app.commands.length !== 3 || app.commands[2].kind !== "canvas/draw") throw new Error("resize should emit a second draw command");
if (!close(app.state.width, 420) || !close(app.state.height, 210)) throw new Error("resized dimensions were not stored");
if (app.state.draws !== 2) throw new Error("resize draw completion did not increment draw count");
if (initialText.nodeValue !== "Drawn 420x210") throw new Error("resize draw status was wrong");
if (section.attributes["data-width"] !== "420" || section.attributes["data-height"] !== "210") {{
  throw new Error("resized dimensions did not render to attrs");
}}
if (canvas.width !== 420 || canvas.height !== 210) throw new Error("resized canvas dimensions were not applied");
if (!canvas.calls.some((call) => call[0] === "fillText" && call[3] === "420x210")) {{
  throw new Error("resized canvas label was not drawn");
}}

button.click();
if (app.commands.length !== 4 || app.commands[3].kind !== "dom-ref/resize-unwatch") throw new Error("stop should emit resize-unwatch");
if (!observers[0].disconnected) throw new Error("resize observer was not disconnected");
if (app.state["watching?"] !== false) throw new Error("watching flag was not cleared");
if (app.state.status !== "Stopped" || initialText.nodeValue !== "Stopped") throw new Error("stop status was wrong");

canvas.setRect(500, 260);
observers[0].trigger(canvas, canvas.rect);
if (app.commands.length !== 4) throw new Error("disconnected observer should not dispatch another draw");
if (app.state.draws !== 2 || app.state.status !== "Stopped") throw new Error("state changed after observer cleanup");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated canvas resize app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_focus_app_focuses_registered_ref() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-focus-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_focus_app.clsk");
    let output = temp_dir.join("focus-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
    this.value = "";
    this.focusCount = 0;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
    if (name === "value") this.value = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "value") this.value = "";
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  emit(name, event) {{
    for (const listener of this.listeners[name] || []) listener(event);
  }}
  click() {{
    this.emit("click", {{ type: "click", currentTarget: this, target: this }});
  }}
  input(value) {{
    this.value = value;
    this.emit("input", {{ type: "input", currentTarget: this, target: this }});
  }}
  focus() {{
    this.focusCount += 1;
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  handlers: runtime.createCommandHandlers()
}});

const section = host.children[0];
const input = section.children.find((node) => node.tagName === "input");
const button = section.children.find((node) => node.tagName === "button");
const span = section.children.find((node) => node.tagName === "span");
const text = span.children.find((node) => "nodeValue" in node);

if (app.getRef("exercise-type") !== input) throw new Error("exercise type ref was not registered");
if (input.attributes.ref !== undefined) throw new Error("ref should not be emitted as an attribute");
if (input.value !== "") throw new Error("initial input value was wrong");
if (text.nodeValue !== "Idle") throw new Error("initial focus status was wrong");
if (section.hasAttribute("data-focused")) throw new Error("focused attr should start absent");

button.click();
if (host.children[0] !== section) throw new Error("focus section was replaced");
if (section.children.find((node) => node.tagName === "input") !== input) throw new Error("focus input was replaced");
if (span.children.find((node) => "nodeValue" in node) !== text) throw new Error("focus status text was replaced");
if (app.commands.length !== 1 || app.commands[0].kind !== "dom-ref/focus") throw new Error("focus command was not logged");
if (app.commands[0].command.ref !== "exercise-type") throw new Error("focus command ref was wrong");
if (input.focusCount !== 1) throw new Error("registered input was not focused");
if (app.state["focused?"] !== true) throw new Error("focus completion did not update state");
if (text.nodeValue !== "Editing") throw new Error("focus completion status did not render");
if (section.attributes["data-focused"] !== "") throw new Error("focused attr was not set");

input.input("Strength");
if (app.state.draft !== "Strength") throw new Error("input event did not update draft after focus");
if (section.children.find((node) => node.tagName === "input") !== input) throw new Error("input was replaced after draft update");
if (input.value !== "Strength") throw new Error("controlled input value did not stay synchronized");
if (app.commands.length !== 1) throw new Error("draft edit should only emit Cmd.none");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated focus app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_hrweb_match_app_handles_tagged_messages() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-match-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_match_app.clsk");
    let output = temp_dir.join("match-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
  }}
  appendChild(node) {{
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
}}

class TextNode {{
  constructor(value) {{
    this.nodeValue = value;
    this.parentNode = null;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const app = runtime.startApp({{ root: host, init: mod.init, update: mod.update, view: mod.view }});
const button = host.children[0];
const text = button.children[0];

app.dispatch({{ kind: Symbol.for("heart-rate"), bpm: 142 }});
if (host.children[0] !== button) throw new Error("button was replaced after match dispatch");
if (button.children[0] !== text) throw new Error("text node was replaced after match dispatch");
if (app.state.latest !== 142) throw new Error("record pattern did not bind bpm");
if (text.nodeValue !== "142") throw new Error("match branch did not update label");
if (button.attributes["data-bpm"] !== "142") throw new Error("match branch did not update bpm attr");

const [startState, cmd] = mod.update(mod.init, {{ kind: Symbol.for("start") }});
if (startState.label !== "Listening") throw new Error("start record pattern did not match");
if (cmd.kind !== Symbol.for("none")) throw new Error("start branch command was wrong");

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated match app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_indexed_slots_skip_unrelated_vector_element_updates() {
    if !node_available() {
        eprintln!("skipping Node smoke test because node is not on PATH");
        return;
    }

    let temp_dir =
        env::temp_dir().join(format!("closkell-indexed-slot-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_indexed_slot_app.clsk");
    let output = temp_dir.join("indexed-slot-app.mjs");

    let build = Command::new(env!("CARGO_BIN_EXE_closkell"))
        .arg("build")
        .arg(&example)
        .arg("--out")
        .arg(&output)
        .output()
        .expect("closkell build should run");
    assert!(
        build.status.success(),
        "closkell build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
class Element {{
  constructor(tagName) {{
    this.tagName = tagName;
    this.children = [];
    this.attributes = {{}};
    this.listeners = {{}};
    this.parentNode = null;
  }}
  appendChild(node) {{
    if (node.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    node.parentNode = this;
    return node;
  }}
  removeChild(node) {{
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    node.parentNode = null;
    return node;
  }}
  setAttribute(name, value) {{
    this.attributes[name] = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  click() {{
    for (const listener of this.listeners.click || []) {{
      listener({{ type: "click", currentTarget: this, target: this }});
    }}
  }}
}}

class TextNode {{
  constructor(value) {{
    this._nodeValue = value;
    this.writeCount = 0;
    this.parentNode = null;
  }}
  get nodeValue() {{
    return this._nodeValue;
  }}
  set nodeValue(value) {{
    this.writeCount += 1;
    this._nodeValue = value;
  }}
}}

globalThis.document = {{
  createElement(tagName) {{
    return new Element(tagName);
  }},
  createTextNode(value) {{
    return new TextNode(value);
  }}
}};

const mod = await import(fileUrl({modulePath}));
const runtime = await import(fileUrl({runtimePath}));
const host = new Element("main");
const devEvents = [];
const app = runtime.startApp({{
  root: host,
  init: mod.init,
  update: mod.update,
  view: mod.view,
  devtools: (event) => devEvents.push(event)
}});

const section = host.children.find((node) => node.tagName === "section");
const renameSecond = childByAttr(section, "button", "data-action", "rename-second");
const firstSpan = childByAttr(section, "span", "data-slot", "first");
const secondStrong = childByAttr(section, "strong", "data-slot", "second");
const firstText = textChild(firstSpan);
const secondText = textChild(secondStrong);
const firstWrites = firstText.writeCount;
const secondWrites = secondText.writeCount;

if (section.attributes["data-first"] !== "Warmup") throw new Error("initial first attr was wrong");
if (section.attributes["data-second"] !== "Tempo") throw new Error("initial second attr was wrong");
if (firstText.nodeValue !== "Warmup" || secondText.nodeValue !== "Tempo") throw new Error("initial indexed labels were wrong");

renameSecond.click();
if (app.state.entries[1].label !== "Intervals") throw new Error("second rename did not update state");
if (host.children.find((node) => node.tagName === "section") !== section) throw new Error("section was replaced after second rename");
if (childByAttr(section, "span", "data-slot", "first") !== firstSpan) throw new Error("first indexed span was replaced");
if (childByAttr(section, "strong", "data-slot", "second") !== secondStrong) throw new Error("second indexed strong was replaced");
if (textChild(firstSpan) !== firstText) throw new Error("first indexed text node was replaced");
if (textChild(secondStrong) !== secondText) throw new Error("second indexed text node was replaced");
if (firstText.writeCount !== firstWrites) throw new Error("first indexed text was rewritten for an unrelated second-entry change");
if (secondText.writeCount !== secondWrites + 1) throw new Error("second indexed text was not rewritten");
if (section.attributes["data-first"] !== "Warmup") throw new Error("first attr changed unexpectedly");
if (section.attributes["data-second"] !== "Intervals") throw new Error("second attr did not update");

const secondUpdate = lastTemplateUpdate();
if (!secondUpdate.changedPaths.includes("state.entries.1.label")) throw new Error("second update did not report indexed changed path");
if (!hasSlotWithRead(secondUpdate.skippedSlots, "state.entries.0.label")) throw new Error("first indexed slot was not reported as skipped");
if (!hasSlotWithRead(secondUpdate.updatedSlots, "state.entries.1.label")) throw new Error("second indexed slot was not reported as updated");

function childByAttr(parent, tagName, attr, value) {{
  const match = parent.children.find((node) => node.tagName === tagName && node.attributes[attr] === value);
  if (!match) throw new Error(`missing ${{tagName}}[${{attr}}="${{value}}"]`);
  return match;
}}

function textChild(node) {{
  const match = node.children.find((child) => "nodeValue" in child);
  if (!match) throw new Error("missing text child");
  return match;
}}

function templateUpdates() {{
  return devEvents.filter((event) => event.type === "template/update");
}}

function lastTemplateUpdate() {{
  const updates = templateUpdates();
  if (!updates.length) throw new Error("no template update events were emitted");
  return updates[updates.length - 1];
}}

function hasSlotWithRead(slots, read) {{
  return (slots || []).some((slot) => (slot.reads || []).includes(read));
}}

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"#,
        modulePath = js_string(&output),
        runtimePath = js_string(&runtime)
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
        "generated indexed slot app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn node_available() -> bool {
    Command::new("node").arg("--version").output().is_ok()
}

fn copy_runtime_package(temp_dir: &Path) {
    let package_dir = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime");
    let source_dir = workspace_root().join("runtime-js");
    fs::create_dir_all(package_dir.join("src")).expect("runtime package dir should be created");
    fs::copy(
        source_dir.join("package.json"),
        package_dir.join("package.json"),
    )
    .expect("runtime package manifest should copy");
    fs::copy(
        source_dir.join("src").join("index.js"),
        package_dir.join("src").join("index.js"),
    )
    .expect("runtime package entry should copy");
}

fn js_string(path: &Path) -> String {
    let value = path.display().to_string();
    let escaped = value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect::<String>();
    format!("\"{}\"", escaped)
}
