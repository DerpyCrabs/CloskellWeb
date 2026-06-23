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
const text = span.children.find((node) => "nodeValue" in node);

if (section.attributes.class !== "panel idle even") throw new Error(`initial class map was wrong: ${{section.attributes.class}}`);
if (section.className !== "panel idle even") throw new Error("className mirror was not updated");
if (section.attributes["data-active"] !== undefined) throw new Error("false data-active attr should be absent");
if (section.attributes["data-count"] !== "0") throw new Error("initial data-count attr was wrong");

button.click();
if (host.children[0] !== section) throw new Error("section was replaced after class update");
if (section.children.find((node) => node.tagName === "span") !== span) throw new Error("span was replaced after class update");
if (span.children.find((node) => "nodeValue" in node) !== text) throw new Error("text node was replaced after class update");
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
