use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn dynamic_class_attrs_accept_records_and_structured_runtime_values() {
    if !node_available() {
        eprintln!("skipping dynamic class attr test because node is unavailable");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-class-attr-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let example = workspace_root()
        .join("fixtures")
        .join("hrweb")
        .join("hrweb_class_attr_app.clsk");
    let output = temp_dir.join("class-attr-app.mjs");

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
    this.className = "";
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
    if (name === "class") this.className = String(value);
  }}
  removeAttribute(name) {{
    delete this.attributes[name];
    if (name === "class") this.className = "";
  }}
  hasAttribute(name) {{
    return Object.prototype.hasOwnProperty.call(this.attributes, name);
  }}
  addEventListener(name, listener) {{
    this.listeners[name] ||= [];
    this.listeners[name].push(listener);
  }}
  removeEventListener(name, listener) {{
    this.listeners[name] = (this.listeners[name] || []).filter((item) => item !== listener);
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

const runtime = await import(fileUrl({runtimePath}));

const scratch = new Element("div");
runtime.setAttr({{ values: [] }}, 0, scratch, "class", [
  "base",
  ["nested", "base"],
  new Set(["ready", Symbol.for("hot")]),
  new Map([["enabled", true], ["disabled", false], [Symbol.for("keyworded"), true]]),
  {{ final: true, hidden: false }}
]);
if (scratch.attributes.class !== "base nested ready hot enabled keyworded final") {{
  throw new Error(`structured class value was not normalized: ${{scratch.attributes.class}}`);
}}
runtime.setAttr({{ values: [] }}, 0, scratch, "class", {{ base: false }});
if (scratch.hasAttribute("class") || scratch.className !== "") throw new Error("empty class object did not clear class attr");

const mod = await import(fileUrl({modulePath}));
const host = new Element("main");
const app = runtime.startApp({{ root: host, init: mod.init, update: mod.update, view: mod.view }});

const section = host.children[0];
const button = section.children.find((node) => node.tagName === "button");
const span = section.children.find((node) => node.tagName === "span");
const text = span.children.find((node) => node.nodeValue === "0");

if (section.attributes.class !== "panel idle even") throw new Error(`initial class map was wrong: ${{section.attributes.class}}`);
if (section.className !== "panel idle even") throw new Error("className mirror was not updated");
if (section.attributes["data-active"] !== undefined) throw new Error("false data-active attr should be absent");
if (section.attributes["data-count"] !== "0") throw new Error("initial data-count attr was wrong");

button.click();
if (host.children[0] !== section) throw new Error("section was replaced after class update");
if (section.children.find((node) => node.tagName === "span") !== span) throw new Error("span was replaced after class update");
if (!span.children.includes(text)) throw new Error("text node was replaced after class update");
if (app.state["active?"] !== true || app.state.count !== 1) throw new Error("toggle did not update state");
if (section.attributes.class !== "panel active") throw new Error(`updated class map was wrong: ${{section.attributes.class}}`);
if (section.attributes["data-active"] !== "") throw new Error("true data-active attr should be present");
if (section.attributes["data-count"] !== "1") throw new Error("updated data-count attr was wrong");
if (text.nodeValue !== "1") throw new Error("count text did not update");

button.click();
if (section.attributes.class !== "panel idle even") throw new Error(`restored class map was wrong: ${{section.attributes.class}}`);
if (section.attributes["data-active"] !== undefined) throw new Error("data-active attr should clear when false again");
if (text.nodeValue !== "2") throw new Error("second count text did not update");

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
        "generated class attr app failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_template_test_dom_preserves_comment_placeholder_width() {
    if !node_available() {
        eprintln!("skipping compiled template placeholder test because node is unavailable");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-template-placeholders-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));
runtime.render(null);

const component = runtime.createCompiledHtmlTemplateComponent(
  "<main><!----> <!----><section><div><!----></div></section></main>",
  "1;30",
  (instance, dispatch, context) => {{
    if (!instance.nodes[0]) throw new Error("missing text slot node");
    if (!instance.nodes[1]) throw new Error("missing structural node after text placeholder pair");
    runtime.setText(instance, 0, instance.nodes[0], "Ready", context);
    instance.nodes[1].setAttribute("data-testid", "target");
  }}
);

const host = document.createElement("div");
component.mount(host, () => {{}});

if (host.textContent !== "Ready") throw new Error(`text slot updated wrong node: ${{host.textContent}}`);
const target = host.querySelector("[data-testid='target']");
if (!target || target.tagName !== "div") throw new Error("structural path after placeholder pair did not resolve");

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

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "compiled template placeholder runtime test failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_composite_slots_skip_unrelated_updates() {
    if !node_available() {
        eprintln!("skipping compiled composite slot test because node is unavailable");
        return;
    }

    let runtime = workspace_root()
        .join("runtime-js")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

function parentNode() {{
  return {{
    children: [],
    insertBefore(node, marker) {{
      if (node.parentNode?.removeChild) node.parentNode.removeChild(node);
      const index = this.children.indexOf(marker);
      const at = index < 0 ? this.children.length : index;
      this.children.splice(at, 0, node);
      node.parentNode = this;
      return node;
    }},
    removeChild(node) {{
      const index = this.children.indexOf(node);
      if (index >= 0) this.children.splice(index, 1);
      node.parentNode = null;
      return node;
    }}
  }};
}}

function child(name, counter) {{
  return {{
    __closkellArity: 0,
    definition: {{ name, params: [] }},
    root: {{}},
    update() {{ counter.count += 1; }},
    dispose() {{}}
  }};
}}

function skippedContext(changedPath) {{
  const frame = {{ updatedSlots: [], skippedSlots: [] }};
  return {{ context: {{ changedPaths: [changedPath], frames: [frame] }}, frame }};
}}

const dispatch = () => {{}};

const componentParent = parentNode();
const componentMarker = {{ parentNode: componentParent }};
const componentCounter = {{ count: 0 }};
const component = child("Child", componentCounter);
const componentInstance = {{
  definition: {{ slots: [{{ id: 0, reads: ["state.rows"] }}] }},
  componentSlots: []
}};
runtime.setCompiledComponent(componentInstance, 0, componentMarker, () => component, [], dispatch, null, "Child");
componentCounter.count = 0;
let skipped = skippedContext("state.unrelated");
runtime.setCompiledComponent(componentInstance, 0, componentMarker, () => component, [], dispatch, skipped.context, "Child");
if (componentCounter.count !== 0) throw new Error("compiled component slot updated after being skipped");
if (skipped.frame.skippedSlots.length !== 1 || skipped.frame.updatedSlots.length !== 0) throw new Error("compiled component slot was not recorded as skipped");

const conditionalParent = parentNode();
const conditionalMarker = {{ parentNode: conditionalParent }};
const conditionalCounter = {{ count: 0 }};
const conditionalComponent = child("Conditional", conditionalCounter);
const conditionalInstance = {{
  definition: {{ slots: [{{ id: 0, reads: ["state.visible"] }}] }},
  conditionalSlots: []
}};
runtime.setCompiledConditional(conditionalInstance, 0, conditionalMarker, true, () => conditionalComponent, () => child("Else", {{ count: 0 }}), dispatch, null);
conditionalCounter.count = 0;
skipped = skippedContext("state.unrelated");
runtime.setCompiledConditional(conditionalInstance, 0, conditionalMarker, true, () => conditionalComponent, () => child("Else", {{ count: 0 }}), dispatch, skipped.context);
if (conditionalCounter.count !== 0) throw new Error("compiled conditional slot updated after being skipped");
if (skipped.frame.skippedSlots.length !== 1 || skipped.frame.updatedSlots.length !== 0) throw new Error("compiled conditional slot was not recorded as skipped");

const keyedParent = parentNode();
const keyedMarker = {{ parentNode: keyedParent }};
const keyedCounter = {{ count: 0 }};
const keyedComponent = child("Row", keyedCounter);
const keyedItem = {{ id: "a", label: "A" }};
const keyedInstance = {{
  definition: {{ slots: [{{ id: 0, reads: ["state.rows"], kind: {{ keyed: "row" }} }}] }},
  keyedSlots: []
}};
runtime.setCompiledKeyedList(keyedInstance, 0, keyedMarker, [keyedItem], (item) => item.id, () => keyedComponent, dispatch, null, false);
keyedCounter.count = 0;
skipped = skippedContext("state.unrelated");
runtime.setCompiledKeyedList(keyedInstance, 0, keyedMarker, [keyedItem], (item) => item.id, () => keyedComponent, dispatch, skipped.context, false);
if (keyedCounter.count !== 0) throw new Error("compiled keyed list slot updated after being skipped");
if (skipped.frame.skippedSlots.length !== 1 || skipped.frame.updatedSlots.length !== 0) throw new Error("compiled keyed list slot was not recorded as skipped");

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
        "compiled composite slot runtime test failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_app_subscription_keys_accept_symbol_ids() {
    if !node_available() {
        eprintln!("skipping compiled app subscription key test because node is unavailable");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-compiled-subscription-symbols-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

let timerStarted = 0;
let timerStopped = 0;
let mediaStarted = 0;
let mediaStopped = 0;
let windowRemoved = 0;
let windowListener = null;
const originalAddEventListener = globalThis.addEventListener;
const originalRemoveEventListener = globalThis.removeEventListener;
globalThis.addEventListener = (type, listener, options) => {{
  if (type !== "keydown") throw new Error(`unexpected window event type: ${{type}}`);
  windowListener = {{ type, listener, options }};
}};
globalThis.removeEventListener = (type, listener, options) => {{
  if (type !== "keydown") throw new Error(`unexpected window event removal type: ${{type}}`);
  if (!windowListener || windowListener.listener !== listener) throw new Error("wrong window listener removed");
  windowRemoved += 1;
  windowListener = null;
}};
const root = {{}};
const handlers = {{}};
runtime.registerCompiledWindowEventCommandHandlers(handlers, {{ disposers: [] }});
Object.assign(handlers, {{
  "timer/every"(command) {{
    if (command.id !== Symbol.for("clock")) throw new Error("symbol id was not preserved");
    timerStarted += 1;
    return null;
  }},
  "timer/cancel"(command) {{
    if (command.id !== Symbol.for("clock")) throw new Error("symbol id was not preserved on stop");
    timerStopped += 1;
    return null;
  }},
  "media-query/watch"(command) {{
    if (command.kind !== Symbol.for("media-query/watch")) throw new Error("direct symbol command kind was not preserved");
    mediaStarted += 1;
    return null;
  }},
  "media-query/unwatch"(command) {{
    if (command.id !== "mobile") throw new Error("direct subscription stop id was not preserved");
    mediaStopped += 1;
    return null;
  }}
}});
const app = runtime.startCompiledApp({{
  root,
  init() {{
    return [{{ count: 0, keys: 0 }}, {{ kind: Symbol.for("none") }}];
  }},
  update(state, msg) {{
    if (msg.kind === Symbol.for("key")) {{
      return [{{ ...state, keys: state.keys + 1 }}, {{ kind: Symbol.for("none") }}];
    }}
    return [{{ ...state, count: state.count + 1 }}, {{ kind: Symbol.for("none") }}];
  }},
  view(state) {{
    return {{
      root: {{ state }},
      mount(parent) {{
        parent.child = this.root;
      }},
      update(nextState) {{
        this.root.state = nextState;
      }},
      dispose() {{}}
    }};
  }},
  subscriptions() {{
    return {{
      kind: Symbol.for("batch"),
      subscriptions: [
        {{ kind: "sub/timer/every", id: Symbol.for("clock"), ms: 1000, onTick: Symbol.for("tick") }},
        {{ kind: Symbol.for("media-query/watch"), s: "media-query/unwatch", id: "mobile", query: "(max-width: 700px)", onChange: Symbol.for("media-changed") }},
        {{ kind: Symbol.for("window/event-watch"), s: "window/event-unwatch", id: "keyboard", type: "keydown", onEvent: Symbol.for("key"), preventDefault: {{ key: "h", ctrlKey: true }} }}
      ]
    }};
  }},
  handlers
}});

if (timerStarted !== 1) throw new Error(`timer subscription did not start once: ${{timerStarted}}`);
if (mediaStarted !== 1) throw new Error(`media subscription did not start once: ${{mediaStarted}}`);
if (!windowListener) throw new Error("compiled direct window subscription did not install");
let prevented = false;
windowListener.listener({{
  type: "keydown",
  key: "H",
  ctrlKey: true,
  shiftKey: false,
  altKey: false,
  metaKey: false,
  preventDefault() {{
    prevented = true;
  }}
}});
if (!prevented) throw new Error("compiled window event preventDefault guard did not match");
if (app.state.keys !== 1) throw new Error(`compiled window event did not dispatch: ${{app.state.keys}}`);
app.dispatch({{ kind: Symbol.for("tick") }});
if (timerStarted !== 1) throw new Error(`stable symbol subscription restarted: ${{timerStarted}}`);
if (mediaStarted !== 1) throw new Error(`stable direct subscription restarted: ${{mediaStarted}}`);
if (app.state.count !== 1) throw new Error(`dispatch failed after symbol subscription sync: ${{app.state.count}}`);
app.dispose();
if (timerStopped !== 1) throw new Error(`timer subscription did not stop once: ${{timerStopped}}`);
if (mediaStopped !== 1) throw new Error(`media subscription did not stop once: ${{mediaStopped}}`);
if (windowRemoved !== 1) throw new Error(`window subscription did not stop once: ${{windowRemoved}}`);
globalThis.addEventListener = originalAddEventListener;
globalThis.removeEventListener = originalRemoveEventListener;

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

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "compiled app subscription symbol runtime test failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_canvas_draw_accepts_symbol_op_kinds() {
    if !node_available() {
        eprintln!("skipping compiled canvas draw symbol op test because node is unavailable");
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-compiled-canvas-symbols-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r##"
const runtime = await import(fileUrl({runtimePath}));

const calls = [];
const ctx = {{
  fillStyle: "",
  lineCap: "",
  setTransform() {{}},
  fillRect(x, y, width, height) {{
    calls.push(["fillRect", x, y, width, height, this.fillStyle, this.lineCap]);
  }}
}};
const canvas = {{
  width: 0,
  height: 0,
  getContext(kind) {{
    if (kind !== "2d") throw new Error(`unexpected context: ${{kind}}`);
    return ctx;
  }},
  getBoundingClientRect() {{
    return {{ width: 100, height: 50 }};
  }}
}};
const dispatch = () => {{}};
dispatch.__closkellRefs = new Map([["chart", canvas]]);

const handlers = {{}};
runtime.registerCompiledCanvasDrawCommandHandlers(handlers);
const message = handlers["canvas/draw"]({{
  kind: Symbol.for("canvas/draw"),
  ref: "chart",
  cssWidth: 100,
  cssHeight: 50,
  ops: [
    {{ kind: Symbol.for("set"), name: Symbol.for("lineCap"), value: "round" }},
    {{ kind: Symbol.for("fill-rect"), color: "#123456", x: 1, y: 2, width: 3, height: 4 }}
  ],
  msg: {{ kind: Symbol.for("drawn") }}
}}, dispatch);

if (message.kind !== Symbol.for("drawn")) throw new Error("compiled draw did not return success message");
if (canvas.width !== 100 || canvas.height !== 50) throw new Error(`canvas size was wrong: ${{canvas.width}}x${{canvas.height}}`);
if (calls.length !== 1) throw new Error(`fill rect was not called once: ${{JSON.stringify(calls)}}`);
const call = calls[0];
if (call[5] !== "#123456") throw new Error(`symbol fill op did not apply color: ${{JSON.stringify(call)}}`);
if (call[6] !== "round") throw new Error(`symbol set op did not apply property: ${{JSON.stringify(call)}}`);

function fileUrl(path) {{
  return "file:///" + path.replace(/\\/g, "/").replace(/^([A-Za-z]):/, "$1:");
}}
"##,
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
        "compiled canvas draw symbol op runtime test failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn compiled_file_read_selected_accepts_symbol_format() {
    if !node_available() {
        eprintln!(
            "skipping compiled file read-selected symbol format test because node is unavailable"
        );
        return;
    }

    let temp_dir = env::temp_dir().join(format!(
        "closkell-compiled-file-read-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let runtime = temp_dir
        .join("node_modules")
        .join("@closkell")
        .join("runtime")
        .join("src")
        .join("index.js");
    let script = format!(
        r#"
const runtime = await import(fileUrl({runtimePath}));

const input = {{
  value: "selected",
  files: [
    {{
      name: "hrweb-import.json",
      type: "application/json",
      async text() {{
        return JSON.stringify({{ version: 2, entries: [{{ id: "imported-tempo" }}] }});
      }}
    }}
  ]
}};
const dispatch = () => {{}};
dispatch.__closkellRefs = new Map([["import-file", input]]);

const handlers = {{}};
runtime.registerCompiledFileReadSelectedCommandHandlers(handlers);
const message = await handlers["file/read-selected"]({{
  kind: Symbol.for("file/read-selected"),
  ref: "import-file",
  format: Symbol.for("json"),
  toMessage: (value) => ({{ kind: Symbol.for("log-imported"), payload: value }})
}}, dispatch);

if (message.kind !== Symbol.for("log-imported")) throw new Error("file read did not produce success message");
if (message.payload.version !== 2) throw new Error(`json file was not parsed: ${{JSON.stringify(message.payload)}}`);
if (message.payload.entries[0].id !== "imported-tempo") throw new Error("parsed entries were wrong");
if (input.files.length !== 0) throw new Error("file input was not cleared");

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

    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        node.status.success(),
        "compiled file read-selected symbol format runtime test failed under Node\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&node.stdout),
        String::from_utf8_lossy(&node.stderr)
    );
}

#[test]
fn runtime_hydrate_app_uses_init_state_without_boot_command() {
    if !node_available() {
        eprintln!("skipping hydrateApp runtime test because node is unavailable");
        return;
    }

    let temp_dir = env::temp_dir().join(format!("closkell-hydrate-app-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");
    copy_runtime_package(&temp_dir);

    let source = temp_dir.join("hydrate_app.clsk");
    fs::write(
        &source,
        "(def init [{:count 0} {:kind :time/now :onSuccess :loaded}])\n\
         \n\
         (defn update [state msg]\n\
           (match msg\n\
             {:kind :inc} [(assoc state :count (+ state.count 1)) {:kind :none}]\n\
             _ [state {:kind :none}]))\n\
         \n\
         (defn view [state]\n\
           #html <section data-testid=\"count\"><button type=\"button\" on:click={{:kind :inc}}>Count {state.count}</button></section>)\n",
    )
    .expect("hydrate app source should be written");
    let output = temp_dir.join("hydrate-app.mjs");

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
const runtime = await import(fileUrl({runtimePath}));
const mod = await import(fileUrl({modulePath}));
runtime.render(null);

const root = document.createElement("main");
const serverSection = document.createElement("section");
serverSection.setAttribute("data-closkell-template", "template0");
serverSection.setAttribute("data-testid", "count");
const serverButton = document.createElement("button");
serverButton.setAttribute("type", "button");
serverButton.appendChild(document.createTextNode("Count "));
const serverValue = document.createTextNode("7");
serverButton.appendChild(serverValue);
serverSection.appendChild(serverButton);
root.appendChild(serverSection);

let bootCommands = 0;
const app = runtime.hydrateApp({{
  root,
  initState: {{ count: 7 }},
  update: mod.update,
  view: mod.view,
  handlers: {{
    "time/now"() {{
      bootCommands += 1;
      return {{ kind: Symbol.for("loaded"), value: 100 }};
    }}
  }}
}});

if (bootCommands !== 0) throw new Error("hydrateApp ran the boot command");
if (root.children.length !== 1) throw new Error(`hydrateApp left duplicate roots: ${{root.children.length}}`);
if (root.children[0] !== serverSection) throw new Error("hydrateApp did not reuse the server root");
if (root.textContent !== "Count 7") throw new Error(`hydrateApp rendered wrong initial state: ${{root.textContent}}`);
if (!root.children[0].hasAttribute("data-closkell-hydrated")) throw new Error("hydrateApp did not mark hydrated root");
if (!serverButton.listeners.click?.length) throw new Error("hydrateApp did not attach event listener to server button");

serverButton.click();
if (app.state.count !== 8) throw new Error(`hydrateApp dispatch returned wrong state: ${{app.state.count}}`);
if (root.textContent !== "Count 8") throw new Error(`hydrateApp did not update DOM: ${{root.textContent}}`);
if (serverValue.nodeValue !== "8") throw new Error(`hydrateApp did not update the reused text node: ${{serverValue.nodeValue}}`);

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
        "hydrateApp runtime test failed under Node\nstdout:\n{}\nstderr:\n{}",
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
