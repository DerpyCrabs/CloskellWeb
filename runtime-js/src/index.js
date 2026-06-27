class CloskellTestTextNode {
  constructor(value = "", ownerDocument = null) {
    this.nodeType = 3;
    this.nodeValue = String(value ?? "");
    this.parentNode = null;
    this.ownerDocument = ownerDocument;
  }

  get textContent() {
    return this.nodeValue;
  }

  set textContent(value) {
    this.nodeValue = String(value ?? "");
  }

  get nextSibling() {
    return siblingForNode(this, 1);
  }

  cloneNode() {
    return new CloskellTestTextNode(this.nodeValue, this.ownerDocument);
  }
}

class CloskellTestElement {
  constructor(tagName = "div", ownerDocument = null, namespaceURI = "http://www.w3.org/1999/xhtml") {
    this.nodeType = 1;
    this.tagName = String(tagName || "div").toLowerCase();
    this.nodeName = this.tagName;
    this.namespaceURI = namespaceURI;
    this.ownerDocument = ownerDocument;
    this.parentNode = null;
    this.children = [];
    this.attributes = {};
    this.listeners = {};
    this.style = new CloskellTestStyle();
    this.className = "";
    this.value = "";
    this.checked = false;
  }

  get childNodes() {
    return this.children;
  }

  get firstChild() {
    return this.children[0] || null;
  }

  get nextSibling() {
    return siblingForNode(this, 1);
  }

  appendChild(node) {
    if (node?.nodeType === 11) {
      for (const child of [...node.children]) this.appendChild(child);
      return node;
    }
    if (node?.parentNode) node.parentNode.removeChild(node);
    this.children.push(node);
    if (node) node.parentNode = this;
    return node;
  }

  insertBefore(node, marker) {
    if (node?.nodeType === 11) {
      for (const child of [...node.children]) this.insertBefore(child, marker);
      return node;
    }
    if (node?.parentNode) node.parentNode.removeChild(node);
    const index = this.children.indexOf(marker);
    if (index === -1) this.children.push(node);
    else this.children.splice(index, 0, node);
    if (node) node.parentNode = this;
    return node;
  }

  removeChild(node) {
    const index = this.children.indexOf(node);
    if (index !== -1) this.children.splice(index, 1);
    if (node) node.parentNode = null;
    return node;
  }

  replaceChildren(...nodes) {
    for (const child of [...this.children]) this.removeChild(child);
    for (const node of nodes) this.appendChild(node);
  }

  setAttribute(name, value) {
    const key = String(name);
    const next = String(value ?? "");
    this.attributes[key] = next;
    if (key === "class") this.className = next;
    if (key === "value") this.value = next;
    if (key === "checked") this.checked = true;
  }

  getAttribute(name) {
    const key = String(name);
    return Object.prototype.hasOwnProperty.call(this.attributes, key) ? this.attributes[key] : null;
  }

  hasAttribute(name) {
    return Object.prototype.hasOwnProperty.call(this.attributes, String(name));
  }

  removeAttribute(name) {
    const key = String(name);
    delete this.attributes[key];
    if (key === "class") this.className = "";
    if (key === "checked") this.checked = false;
  }

  addEventListener(type, listener) {
    this.listeners[type] ||= [];
    this.listeners[type].push(listener);
  }

  removeEventListener(type, listener) {
    this.listeners[type] = (this.listeners[type] || []).filter((entry) => entry !== listener);
  }

  dispatchEvent(event) {
    event.target ||= this;
    event.currentTarget = this;
    for (const listener of [...(this.listeners[event.type] || [])]) listener(event);
    if (event.bubbles !== false && !event.propagationStopped) {
      const nextTarget = this.parentNode?.dispatchEvent ? this.parentNode : this.ownerDocument;
      if (nextTarget?.dispatchEvent && nextTarget !== this) nextTarget.dispatchEvent(event);
    }
    return !event.defaultPrevented;
  }

  click() {
    return this.dispatchEvent(createTestEvent("click", { target: this, currentTarget: this }));
  }

  matches(selector) {
    return selectorMatchesNode(this, selector);
  }

  querySelector(selector) {
    return this.querySelectorAll(selector)[0] || null;
  }

  querySelectorAll(selector) {
    return querySelectorAllFrom(this, selector);
  }

  cloneNode(deep = false) {
    const clone = new CloskellTestElement(this.tagName, this.ownerDocument, this.namespaceURI);
    for (const [name, value] of Object.entries(this.attributes)) clone.setAttribute(name, value);
    clone.style.cssText = this.style.cssText;
    clone.value = this.value;
    clone.checked = this.checked;
    if (deep) {
      for (const child of this.children) clone.appendChild(child.cloneNode?.(true) ?? child);
    }
    return clone;
  }

  get textContent() {
    return this.children.map((child) => child.textContent ?? "").join("");
  }

  set textContent(value) {
    this.replaceChildren(new CloskellTestTextNode(value, this.ownerDocument));
  }

  get innerHTML() {
    return this.children.map(serializeTestNode).join("");
  }

  set innerHTML(value) {
    if (this.tagName === "template" && this.content) {
      this.content.replaceChildren(...parseTestHtmlFragment(value, this.ownerDocument).children);
      return;
    }
    this.replaceChildren(new CloskellTestTextNode(value, this.ownerDocument));
  }
}

class CloskellTestDocumentFragment extends CloskellTestElement {
  constructor(ownerDocument = null) {
    super("#fragment", ownerDocument);
    this.nodeType = 11;
  }

  cloneNode(deep = false) {
    const fragment = new CloskellTestDocumentFragment(this.ownerDocument);
    if (deep) {
      for (const child of this.children) fragment.appendChild(child.cloneNode?.(true) ?? child);
    }
    return fragment;
  }
}

class CloskellTestStyle {
  constructor() {
    this.values = {};
  }

  setProperty(name, value) {
    this.values[String(name)] = String(value ?? "");
  }

  getPropertyValue(name) {
    return this.values[String(name)] ?? "";
  }

  removeProperty(name) {
    const key = String(name);
    const previous = this.values[key] ?? "";
    delete this.values[key];
    return previous;
  }

  get cssText() {
    return Object.entries(this.values).map(([name, value]) => `${name}: ${value};`).join(" ");
  }

  set cssText(value) {
    this.values = {};
    for (const part of String(value || "").split(";")) {
      const index = part.indexOf(":");
      if (index === -1) continue;
      this.setProperty(part.slice(0, index).trim(), part.slice(index + 1).trim());
    }
  }
}

function parseTestHtmlFragment(value, documentRef) {
  const fragment = new CloskellTestDocumentFragment(documentRef);
  const stack = [fragment];
  const tokens = String(value ?? "").match(/<!--[\s\S]*?-->|<\/?[A-Za-z][^>]*>|[^<]+/g) || [];
  for (let tokenIndex = 0; tokenIndex < tokens.length; tokenIndex += 1) {
    const token = tokens[tokenIndex];
    const parent = stack[stack.length - 1];
    if (token.startsWith("<!--")) {
      if (/^\s*$/.test(tokens[tokenIndex + 1] || "") && (tokens[tokenIndex + 2] || "").startsWith("<!--")) {
        parent.appendChild(createRuntimeTextNode(documentRef, ""));
        tokenIndex += 2;
        continue;
      }
      parent.appendChild(createRuntimeTextNode(documentRef, ""));
      continue;
    }
    if (token.startsWith("</")) {
      const closing = token.match(/^<\/\s*([^\s>]+)/)?.[1]?.toLowerCase();
      if (stack.length > 1 && stack[stack.length - 1].tagName === closing) stack.pop();
      continue;
    }
    if (token.startsWith("<")) {
      const match = token.match(/^<\s*([^\s/>]+)([\s\S]*?)\/?\s*>$/);
      if (!match) continue;
      const [, tagName, rawAttrs] = match;
      const node = documentRef.createElement(tagName);
      for (const [name, attrValue] of parseTestHtmlAttrs(rawAttrs)) {
        node.setAttribute(name, attrValue);
      }
      parent.appendChild(node);
      if (!token.endsWith("/>") && !isTestVoidElement(tagName)) stack.push(node);
      continue;
    }
    parent.appendChild(createRuntimeTextNode(documentRef, decodeTestHtml(token)));
  }
  return fragment;
}

function createRuntimeTextNode(documentRef, value) {
  return documentRef?.createTextNode ? documentRef.createTextNode(value) : new CloskellTestTextNode(value, documentRef);
}

function parseTestHtmlAttrs(value) {
  const attrs = [];
  const pattern = /([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+)))?/g;
  for (const match of String(value ?? "").matchAll(pattern)) {
    attrs.push([match[1], decodeTestHtml(match[2] ?? match[3] ?? match[4] ?? "")]);
  }
  return attrs;
}

function decodeTestHtml(value) {
  return String(value ?? "")
    .replace(/&quot;/g, "\"")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

function isTestVoidElement(tagName) {
  return /^(area|base|br|col|embed|hr|img|input|link|meta|param|source|track|wbr)$/i.test(tagName);
}

function ensureRuntimeDocument() {
  if (globalThis.document?.createElement && globalThis.document?.createTextNode) {
    return globalThis.document;
  }
  const listeners = {};
  const documentRef = {
    createElement(tagName) {
      const element = new CloskellTestElement(tagName, documentRef);
      if (String(tagName).toLowerCase() === "template") {
        element.content = new CloskellTestDocumentFragment(documentRef);
      }
      return element;
    },
    createElementNS(namespaceURI, tagName) {
      return new CloskellTestElement(tagName, documentRef, namespaceURI);
    },
    createTextNode(value) {
      return new CloskellTestTextNode(value, documentRef);
    },
    createDocumentFragment() {
      return new CloskellTestDocumentFragment(documentRef);
    },
    querySelector(selector) {
      return documentRef.body.querySelector(selector);
    },
    querySelectorAll(selector) {
      return documentRef.body.querySelectorAll(selector);
    },
    addEventListener(type, listener) {
      listeners[type] ||= [];
      listeners[type].push(listener);
    },
    removeEventListener(type, listener) {
      listeners[type] = (listeners[type] || []).filter((entry) => entry !== listener);
    },
    dispatchEvent(event) {
      event.target ||= documentRef;
      event.currentTarget = documentRef;
      for (const listener of [...(listeners[event.type] || [])]) listener(event);
      return !event.defaultPrevented;
    }
  };
  documentRef.body = new CloskellTestElement("body", documentRef);
  globalThis.document = documentRef;
  return documentRef;
}

export function htmlTemplate(source) {
  const documentRef = ensureRuntimeDocument();
  const template = documentRef.createElement("template");
  template.innerHTML = source;
  return {
    mount(parent) {
      const fragment = template.content.cloneNode(true);
      parent.appendChild(fragment);
    },
    source
  };
}

export function createDevtoolsOverlay(options = {}) {
  const config = normalizeDevtoolsOverlayOptions(options);
  const host = config.host || globalThis;
  const documentRef = config.document || host.document;
  const maxEvents = positiveNumber(config.maxEvents, 80);
  const maxVisible = positiveNumber(config.maxVisible, 18);
  const events = [];
  const overlay = createDevtoolsOverlayRoot(documentRef);
  let disposed = false;

  const api = {
    events,
    emit(event) {
      if (disposed) return;
      events.push(event);
      while (events.length > maxEvents) events.shift();
      config.onEvent?.(event);
      renderDevtoolsOverlay(overlay, events, maxVisible);
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      overlay.root?.parentNode?.removeChild?.(overlay.root);
    },
    get root() {
      return overlay.root;
    }
  };

  renderDevtoolsOverlay(overlay, events, maxVisible);
  return api;
}

function normalizeDevtoolsOverlayOptions(options) {
  if (typeof options === "function") return normalizeDevtoolsOverlayOptions(options());
  if (options === true || options == null) return {};
  if (typeof options === "object") return options;
  return {};
}

function createDevtoolsOverlayRoot(documentRef) {
  if (!documentRef?.createElement) return { root: null, summary: null, list: null };

  const root = documentRef.createElement("section");
  root.setAttribute?.("data-closkell-devtools-overlay", "");
  root.style.cssText = [
    "position:fixed",
    "right:12px",
    "bottom:12px",
    "z-index:2147483647",
    "width:min(420px,calc(100vw - 24px))",
    "max-height:min(460px,calc(100vh - 24px))",
    "overflow:hidden",
    "border:1px solid rgba(23,32,25,0.22)",
    "border-radius:8px",
    "background:rgba(255,255,255,0.96)",
    "box-shadow:0 18px 45px rgba(23,32,25,0.18)",
    "color:#172019",
    "font:12px/1.35 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace"
  ].join(";");

  const header = documentRef.createElement("div");
  header.style.cssText = [
    "display:flex",
    "align-items:center",
    "justify-content:space-between",
    "gap:8px",
    "border-bottom:1px solid rgba(23,32,25,0.12)",
    "padding:8px 10px",
    "font-weight:700"
  ].join(";");

  const title = documentRef.createElement("span");
  title.textContent = "Closkell";
  const summary = documentRef.createElement("span");
  summary.style.cssText = "color:#617066;font-weight:700";
  header.appendChild(title);
  header.appendChild(summary);

  const list = documentRef.createElement("div");
  list.style.cssText = "display:grid;gap:1px;max-height:390px;overflow:auto;padding:6px";

  root.appendChild(header);
  root.appendChild(list);
  documentRef.body?.appendChild?.(root);

  return { root, summary, list };
}

function renderDevtoolsOverlay(overlay, events, maxVisible) {
  if (!overlay.summary || !overlay.list) return;

  overlay.summary.textContent = `${events.length} event${events.length === 1 ? "" : "s"}`;
  clearNode(overlay.list);

  const visible = events.slice(-maxVisible).reverse();
  for (const event of visible) {
    const row = overlay.list.ownerDocument?.createElement
      ? overlay.list.ownerDocument.createElement("div")
      : overlay.list.parentNode?.ownerDocument?.createElement?.("div");
    const item = row || createDetachedOverlayRow(overlay.list);
    item.style.cssText = [
      "overflow:hidden",
      "border-radius:5px",
      "padding:5px 6px",
      "background:#f7faf8",
      "text-overflow:ellipsis",
      "white-space:nowrap"
    ].join(";");
    item.textContent = devtoolsEventSummary(event);
    overlay.list.appendChild?.(item);
  }
}

function createDetachedOverlayRow(list) {
  const documentRef = list?.ownerDocument || globalThis.document;
  return documentRef?.createElement?.("div") || { style: {}, textContent: "" };
}

function clearNode(node) {
  if (node.replaceChildren) {
    node.replaceChildren();
    return;
  }
  while (node.firstChild) node.removeChild(node.firstChild);
}

function devtoolsEventSummary(event = {}) {
  const type = event.type || "event";
  switch (type) {
    case "state/update":
      return `${type} ${devtoolsValueName(event.message)} ${devtoolsPathList(event.changedPaths)}`;
    case "template/update":
      return `${type} ${event.name || ""} +${event.updatedSlots?.length || 0} -${event.skippedSlots?.length || 0} ${devtoolsPathList(event.changedPaths)}`;
    case "template/mount":
      return `${type} ${event.name || ""} ${event.slots?.length || 0} slots`;
    case "template/dispose":
      return `${type} ${event.name || ""}`;
    case "command/run":
      return `${type} ${event.kind || ""}`;
    case "command/error":
      return `${type} ${event.kind || ""} ${event.error || ""}`;
    case "app/init":
    case "app/mount":
    case "app/dispose":
      return type;
    default:
      return type;
  }
}

function devtoolsPathList(paths) {
  return Array.isArray(paths) && paths.length ? paths.join(",") : "";
}

function devtoolsValueName(value) {
  if (typeof value === "symbol") return `:${Symbol.keyFor(value) || value.description || ""}`;
  if (value && typeof value === "object" && "kind" in value) return devtoolsValueName(value.kind);
  if (value == null) return "";
  return String(value);
}

function positiveNumber(value, fallback) {
  const number = Number(value);
  return Number.isFinite(number) && number > 0 ? Math.floor(number) : fallback;
}

export function createTemplateComponent(definition) {
  let instance = null;
  let lastDispatch = () => {};

  const ensureInstance = () => {
    if (!instance) {
      instance = definition.create();
      instance.values = [];
      instance.eventSlots = [];
      instance.keyedSlots = [];
      instance.conditionalSlots = [];
      instance.componentSlots = [];
      instance.refSlots = [];
    }
    return instance;
  };

  const disposeInstance = (current) => {
    if (!current) return;
    if (current.__closkellTemplateReported) {
      emitDispatchDevtools(lastDispatch, {
        type: "template/dispose",
        name: definition.name
      });
    }
    disposeEventSlots(current);
    disposeRefs(current);
    for (const slot of current.keyedSlots) {
      if (slot) for (const entry of slot.byKey.values()) disposeComponent(entry.component);
    }
    for (const slot of current.conditionalSlots) if (slot) disposeComponent(slot.component);
    for (const slot of current.componentSlots) if (slot) disposeComponent(slot.component);
    if (current.root?.parentNode?.removeChild) current.root.parentNode.removeChild(current.root);
    current.mounted = false;
  };

  return {
    definition,
    mount(parent, dispatch = lastDispatch, hydrateNode = null) {
      lastDispatch = dispatch || lastDispatch;
      const current = ensureInstance();
      reportTemplateMount(current, lastDispatch, definition);
      if (!current.mounted) {
        if (hydrateNode && claimHydratedTemplateInstance(current, definition, hydrateNode)) {
          if (current.root.parentNode !== parent) parent.appendChild(current.root);
        } else {
          parent.appendChild(current.root);
        }
        current.mounted = true;
      }
      definition.update(current, lastDispatch, null);
      return current.root;
    },
    update(dispatch = lastDispatch, updateContext = null) {
      lastDispatch = dispatch || lastDispatch;
      const current = ensureInstance();
      reportTemplateMount(current, lastDispatch, definition);
      const frame = beginTemplateUpdate(updateContext, definition);
      definition.update(current, lastDispatch, updateContext);
      endTemplateUpdate(updateContext, frame);
      return current.root;
    },
    dispose() {
      disposeInstance(instance);
      instance = null;
    },
    get root() {
      return ensureInstance().root;
    },
    get __closkellInstance() {
      return instance;
    }
  };
}

export function createCompiledTemplateComponent(definition) {
  let instance = null;
  let lastDispatch = () => {};

  const ensureInstance = () => {
    if (!instance) {
      instance = definition.create();
      instance.values = [];
      instance.eventSlots = [];
      instance.keyedSlots = [];
      instance.conditionalSlots = [];
      instance.componentSlots = [];
      instance.refSlots = [];
      instance.definition = definition;
    }
    return instance;
  };

  const disposeInstance = (current, detached = false) => {
    if (!current) return;
    disposeEventSlots(current, detached);
    disposeRefs(current);
    for (const slot of current.keyedSlots) {
      if (slot) for (const entry of slot.byKey.values()) disposeComponent(entry.component, detached);
    }
    for (const slot of current.conditionalSlots) if (slot) disposeComponent(slot.component, detached);
    for (const slot of current.componentSlots) if (slot) disposeComponent(slot.component, detached);
    if (!detached && current.root?.parentNode?.removeChild) current.root.parentNode.removeChild(current.root);
    current.mounted = false;
  };

  return {
    mount(parent, dispatch = lastDispatch) {
      lastDispatch = dispatch || lastDispatch;
      const current = ensureInstance();
      if (!current.mounted) {
        parent.appendChild(current.root);
        current.mounted = true;
      }
      definition.update(current, lastDispatch, null);
      return current.root;
    },
    update(dispatch = lastDispatch, updateContext = null) {
      lastDispatch = dispatch || lastDispatch;
      const current = ensureInstance();
      definition.update(current, lastDispatch, updateContext);
      return current.root;
    },
    dispose(detached = false) {
      disposeInstance(instance, detached);
      instance = null;
    },
    get root() {
      return ensureInstance().root;
    }
  };
}

function createCompiledHtmlTemplateComponentFromShape(shape, update) {
  const cloneTemplate = shape.cloneTemplate;
  const nodePaths = shape.nodePaths;
  let instance = null;
  let lastDispatch = () => {};

  const ensureInstance = () => {
    if (!instance) {
      const root = cloneTemplate();
      instance = {
        root,
        nodes: nodePaths.map((path) => compiledHtmlTemplateNode(root, path)),
        values: [],
        eventSlots: [],
        keyedSlots: [],
        conditionalSlots: [],
        componentSlots: [],
        refSlots: []
      };
    }
    return instance;
  };

  const disposeInstance = (current, detached = false) => {
    if (!current) return;
    disposeEventSlots(current, detached);
    disposeRefs(current);
    for (const slot of current.keyedSlots) {
      if (slot) for (const entry of slot.byKey.values()) disposeComponent(entry.component, detached);
    }
    for (const slot of current.conditionalSlots) if (slot) disposeComponent(slot.component, detached);
    for (const slot of current.componentSlots) if (slot) disposeComponent(slot.component, detached);
    if (!detached && current.root?.parentNode?.removeChild) current.root.parentNode.removeChild(current.root);
    current.mounted = false;
  };

  return {
    mount(parent, dispatch = lastDispatch) {
      lastDispatch = dispatch || lastDispatch;
      const current = ensureInstance();
      update(current, lastDispatch);
      if (!current.mounted) {
        parent.appendChild(current.root);
        current.mounted = true;
      }
      return current.root;
    },
    update(dispatch = lastDispatch) {
      lastDispatch = dispatch || lastDispatch;
      const current = ensureInstance();
      update(current, lastDispatch);
      return current.root;
    },
    dispose(detached = false) {
      disposeInstance(instance, detached);
      instance = null;
    },
    get root() {
      return ensureInstance().root;
    }
  };
}

export function createCompiledHtmlTemplateComponent(html, paths, update) {
  return createCompiledHtmlTemplateComponentFromShape(compiledHtmlTemplateShape(html, paths), update);
}

export function createBrowserCompiledHtmlTemplateComponent(html, paths, update) {
  return createCompiledHtmlTemplateComponentFromShape(browserCompiledHtmlTemplateShape(html, paths), update);
}

function createCompiledHtmlTemplateFactory(html) {
  const documentRef = ensureRuntimeDocument();
  const template = documentRef.createElement("template");
  template.innerHTML = html;
  if (template.content?.firstChild) {
    return () => template.content.firstChild.cloneNode(true);
  }
  return () => parseTestHtmlFragment(html, documentRef).firstChild;
}

const compiledHtmlTemplateShapes = new Map();
const browserCompiledHtmlTemplateShapes = new Map();

function compiledHtmlTemplateShape(html, paths) {
  let byPaths = compiledHtmlTemplateShapes.get(html);
  if (!byPaths) {
    byPaths = new Map();
    compiledHtmlTemplateShapes.set(html, byPaths);
  }
  const key = paths || "";
  let shape = byPaths.get(key);
  if (!shape) {
    shape = {
      cloneTemplate: createCompiledHtmlTemplateFactory(html),
      nodePaths: paths ? paths.split(";").map(compiledHtmlTemplatePath) : []
    };
    byPaths.set(key, shape);
  }
  return shape;
}

function createBrowserCompiledHtmlTemplateFactory(html) {
  const documentRef = browserRuntimeDocument();
  const template = documentRef.createElement("template");
  template.innerHTML = html;
  return () => template.content.firstChild.cloneNode(true);
}

function browserRuntimeDocument() {
  const documentRef = globalThis.document;
  if (!documentRef?.createElement) {
    throw new Error("Closkell browser app runtime requires a DOM document.");
  }
  return documentRef;
}

function browserCompiledHtmlTemplateShape(html, paths) {
  let byPaths = browserCompiledHtmlTemplateShapes.get(html);
  if (!byPaths) {
    byPaths = new Map();
    browserCompiledHtmlTemplateShapes.set(html, byPaths);
  }
  const key = paths || "";
  let shape = byPaths.get(key);
  if (!shape) {
    shape = {
      cloneTemplate: createBrowserCompiledHtmlTemplateFactory(html),
      nodePaths: paths ? paths.split(";").map(compiledHtmlTemplatePath) : []
    };
    byPaths.set(key, shape);
  }
  return shape;
}

function compiledHtmlTemplateNode(root, path) {
  if (path === null) return root;
  let node = root;
  for (let index = 0; index < path.length; index += 1) {
    const children = node.childNodes || node.children;
    node = children[path[index]] ?? (children.length === 1 ? children[0] : undefined);
  }
  return node;
}

function compiledHtmlTemplatePath(path) {
  if (path === "-") return null;
  const indexes = new Array(path.length);
  for (let index = 0; index < path.length; index += 1) {
    indexes[index] = parseInt(path[index], 36);
  }
  return indexes;
}

export function bindCompiledComponent(component, arity, bind) {
  if (arity === 1) {
    return {
      __closkellArity: arity,
      mount(parent, dispatch) {
        return component.mount(parent, dispatch);
      },
      update(value, dispatch) {
        if (bind) bind(value, dispatch);
        return component.update(dispatch);
      },
      dispose(detached = false) {
        component.dispose(detached);
      },
      get root() {
        return component.root;
      }
    };
  }
  if (arity === 2) {
    return {
      __closkellArity: arity,
      mount(parent, dispatch) {
        return component.mount(parent, dispatch);
      },
      update(value, index, dispatch) {
        if (bind) bind(value, index, dispatch);
        return component.update(dispatch);
      },
      dispose(detached = false) {
        component.dispose(detached);
      },
      get root() {
        return component.root;
      }
    };
  }
  return {
    __closkellArity: arity,
    mount(parent, dispatch) {
      return component.mount(parent, dispatch);
    },
    update(...args) {
      if (bind) bind(...args);
      return component.update(args[args.length - 1]);
    },
    dispose(detached = false) {
      component.dispose(detached);
    },
    get root() {
      return component.root;
    }
  };
}

export function shouldUpdateSlot(instance, slot, updateContext) {
  const slotMetadata = instance.definition?.slots?.[slot] || { id: slot, reads: [] };
  const shouldUpdate = shouldUpdateSlotForReads(slotMetadata.reads || [], updateContext);
  recordTemplateSlot(updateContext, slotMetadata, shouldUpdate);
  return shouldUpdate;
}

export function shouldUpdateCompiledSlot(reads, updateContext) {
  return shouldUpdateSlotForReads(reads || [], updateContext);
}

function claimHydratedTemplateInstance(instance, definition, hydrateNode) {
  if (!hydrateNode || hydrateNode.nodeType !== 1) return false;
  const templateName = hydrateNode.getAttribute?.("data-closkell-template");
  if (!templateName || templateName !== definition.name) return false;

  const nodeMap = new Map();
  if (!claimHydratedTree(instance.root, hydrateNode, nodeMap)) return false;

  instance.root = hydrateNode;
  instance.nodes = (instance.nodes || []).map((node) => nodeMap.get(node) || node);
  instance.hydrated = true;
  return true;
}

function claimHydratedTree(blueprint, existing, nodeMap) {
  if (!hydrationNodesCompatible(blueprint, existing)) return false;
  nodeMap.set(blueprint, existing);

  const blueprintChildren = Array.from(blueprint?.childNodes || blueprint?.children || []);
  const existingChildren = Array.from(existing?.childNodes || existing?.children || []);
  if (blueprintChildren.length !== existingChildren.length) return false;

  for (let index = 0; index < blueprintChildren.length; index += 1) {
    if (!claimHydratedTree(blueprintChildren[index], existingChildren[index], nodeMap)) {
      return false;
    }
  }
  return true;
}

function hydrationNodesCompatible(blueprint, existing) {
  if (!blueprint || !existing || blueprint.nodeType !== existing.nodeType) return false;
  if (blueprint.nodeType !== 1) return true;
  return String(blueprint.tagName || "").toLowerCase() === String(existing.tagName || "").toLowerCase();
}

function shouldUpdateSlotForReads(reads, updateContext) {
  if (!updateContext || updateContext.force) return true;
  if (!reads.length) return true;

  const localReadPrefixes = Array.isArray(updateContext.localReadPrefixes)
    ? updateContext.localReadPrefixes
    : [];
  if (reads.some((read) => !isStatePath(read) && !isLocalReadPath(read, localReadPrefixes))) {
    return true;
  }

  const changedPaths = changedPathsForUpdate(updateContext);
  if (!changedPaths.length) return false;
  return reads.some((read) => changedPaths.some((changed) => pathsOverlap(read, changed)));
}

function beginTemplateUpdate(updateContext, definition) {
  if (!updateContext) return null;
  const frame = {
    type: "template/update",
    name: definition.name,
    changedPaths: updateContext.changedPaths || [],
    localChangedPaths: updateContext.localChangedPaths || [],
    updatedSlots: [],
    skippedSlots: []
  };
  updateContext.frames ||= [];
  updateContext.frames.push(frame);
  return frame;
}

function endTemplateUpdate(updateContext, frame) {
  if (!updateContext || !frame) return;
  updateContext.frames.pop();
  emitDevtools(updateContext.devtools, frame);
}

function recordTemplateSlot(updateContext, slot, updated) {
  const frame = updateContext?.frames?.[updateContext.frames.length - 1];
  if (!frame) return;
  const target = updated ? frame.updatedSlots : frame.skippedSlots;
  target.push(slot);
}

function reportTemplateMount(instance, dispatch, definition) {
  if (instance.__closkellTemplateReported) return;
  instance.__closkellTemplateReported = true;
  emitDispatchDevtools(dispatch, {
    type: "template/mount",
    name: definition.name,
    slots: definition.slots || []
  });
}

export function setText(instance, slot, node, value) {
  const next = value == null ? "" : String(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  node.nodeValue = next;
}

export function setAttr(instance, slot, node, name, value) {
  const previous = instance.values[slot];
  if (previous === value) return;
  instance.values[slot] = value;
  const isSvg = node.namespaceURI === "http://www.w3.org/2000/svg";

  if (name === "style" && isStyleObject(value)) {
    applyStyleObject(node, value, previous);
    return;
  }

  if (name === "style" && isStyleObject(previous)) {
    clearStyleObject(node, previous);
  }

  if (name === "class" && isStructuredClassValue(value)) {
    applyClassValue(node, value);
    return;
  }

  if (value === false || value == null) {
    if (name === "style") {
      clearStyleAttribute(node);
      return;
    }
    node.removeAttribute(name);
    if (!isSvg && name in node) setDomProperty(node, name, false);
    return;
  }

  if (value === true) {
    node.setAttribute(name, "");
    if (!isSvg && name in node) setDomProperty(node, name, true);
    return;
  }

  node.setAttribute(name, String(value));
  if (name === "style") {
    if (node.style) node.style.cssText = String(value);
    return;
  }
  if (!isSvg && name in node) setDomProperty(node, name, value);
}

export function setCompiledAttr(instance, slot, node, name, value) {
  const previous = instance.values[slot];
  if (previous === value) return;
  instance.values[slot] = value;
  const isSvg = node.namespaceURI === "http://www.w3.org/2000/svg";

  if (value === false || value == null) {
    node.removeAttribute(name);
    if (!isSvg && name in node) setDomProperty(node, name, false);
    return;
  }

  if (value === true) {
    node.setAttribute(name, "");
    if (!isSvg && name in node) setDomProperty(node, name, true);
    return;
  }

  node.setAttribute(name, String(value));
  if (!isSvg && name in node) setDomProperty(node, name, value);
}

export function setCompiledTextAttr(instance, slot, node, name, value) {
  const next = String(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  node.setAttribute(name, next);
}

export function setCompiledNullableTextAttr(instance, slot, node, name, value) {
  const next = value == null ? null : String(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  if (next == null) node.removeAttribute(name);
  else node.setAttribute(name, next);
}

export function setCompiledTextProperty(instance, slot, node, name, value) {
  const next = String(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  node.setAttribute(name, next);
  node[name] = next;
}

export function setCompiledNullableTextProperty(instance, slot, node, name, value) {
  const next = value == null ? null : String(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  if (next == null) {
    node.removeAttribute(name);
    node[name] = false;
  } else {
    node.setAttribute(name, next);
    node[name] = next;
  }
}

export function setCompiledPresenceAttr(instance, slot, node, name, value) {
  const next = Boolean(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  if (next) node.setAttribute(name, "");
  else node.removeAttribute(name);
}

export function setCompiledBooleanProperty(instance, slot, node, name, value) {
  const next = Boolean(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  if (next) node.setAttribute(name, "");
  else node.removeAttribute(name);
  node[name] = next;
}

export function setCompiledClassName(instance, slot, node, value) {
  const next = String(value);
  if (instance.values[slot] === next) return;
  instance.values[slot] = next;
  if (next === "" && node.className === "" && !node.hasAttribute("class")) return;
  node.setAttribute("class", next);
  node.className = next;
}

export function setCompiledClass(instance, slot, node, value) {
  const previous = instance.values[slot];
  if (previous === value) return;
  instance.values[slot] = value;
  if (isStructuredClassValue(value)) {
    applyClassValue(node, value);
    return;
  }
  if (value === false || value == null) {
    node.removeAttribute("class");
    if ("className" in node) node.className = "";
    return;
  }
  const className = String(value);
  node.setAttribute("class", className);
  if ("className" in node) node.className = className;
}

export function setCompiledStyle(instance, slot, node, value) {
  const previous = instance.values[slot];
  if (previous === value) return;
  instance.values[slot] = value;
  if (isStyleObject(value)) {
    applyStyleObject(node, value, previous);
    return;
  }
  if (isStyleObject(previous)) clearStyleObject(node, previous);
  if (value === false || value == null) {
    clearStyleAttribute(node);
    return;
  }
  node.setAttribute("style", String(value));
  if (node.style) node.style.cssText = String(value);
}

export function setCompiledStyleRecord(instance, slot, node, value) {
  const previous = instance.values[slot];
  if (previous === value) return;
  instance.values[slot] = value;
  if (!node.style) node.style = {};

  if (previous) {
    for (const name of Object.keys(previous)) {
      if (!Object.prototype.hasOwnProperty.call(value, name)) removeStyleProperty(node, name);
    }
  }

  for (const [name, rawValue] of Object.entries(value)) {
    if (rawValue === false || rawValue == null) {
      removeStyleProperty(node, name);
    } else {
      setStyleProperty(node, name, rawValue);
    }
  }
}

function setDomProperty(node, name, value) {
  try {
    node[name] = value;
  } catch {
    // Some reflected attributes expose read-only DOM properties, for example button.form.
  }
}

function isStructuredClassValue(value) {
  return Array.isArray(value)
    || value instanceof Set
    || value instanceof Map
    || (value !== null && typeof value === "object");
}

function applyClassValue(node, value) {
  const className = classValueToString(value);
  if (className) {
    node.setAttribute("class", className);
    if ("className" in node) node.className = className;
  } else {
    node.removeAttribute("class");
    if ("className" in node) node.className = "";
  }
}

function classValueToString(value) {
  const tokens = [];
  appendClassTokens(value, tokens);
  return [...new Set(tokens)].join(" ");
}

function appendClassTokens(value, tokens) {
  if (value === false || value == null) return;
  if (value === true) return;
  if (Array.isArray(value)) {
    for (const item of value) appendClassTokens(item, tokens);
    return;
  }
  if (value instanceof Set) {
    for (const item of value) appendClassTokens(item, tokens);
    return;
  }
  if (value instanceof Map) {
    for (const [name, enabled] of value.entries()) {
      if (enabled) appendClassTokens(name, tokens);
    }
    return;
  }
  if (typeof value === "object") {
    for (const [name, enabled] of Object.entries(value)) {
      if (enabled) tokens.push(name);
    }
    return;
  }
  const token = classTokenName(value);
  if (token) tokens.push(token);
}

function classTokenName(value) {
  return String(value);
}

function isStyleObject(value) {
  return value instanceof Map || (value !== null && typeof value === "object" && !Array.isArray(value));
}

function styleEntries(styles) {
  if (styles instanceof Map) return styles.entries();
  return Object.entries(styles || {});
}

function styleKeys(styles) {
  if (styles instanceof Map) return styles.keys();
  return Object.keys(styles || {});
}

function hasStyleKey(styles, name) {
  if (styles instanceof Map) return styles.has(name);
  return Object.prototype.hasOwnProperty.call(styles || {}, name);
}

function applyStyleObject(node, next, previous) {
  if (!node.style) node.style = {};
  if (isStyleObject(previous)) {
    for (const name of styleKeys(previous)) {
      if (!hasStyleKey(next, name)) removeStyleProperty(node, name);
    }
  } else if (typeof previous === "string") {
    clearStyleAttribute(node);
  }

  for (const [name, rawValue] of styleEntries(next)) {
    if (rawValue === false || rawValue == null) {
      removeStyleProperty(node, name);
    } else {
      setStyleProperty(node, name, rawValue);
    }
  }
}

function clearStyleObject(node, styles) {
  for (const name of styleKeys(styles)) removeStyleProperty(node, name);
}

function clearStyleAttribute(node) {
  node.removeAttribute?.("style");
  if (node.style?.cssText !== undefined) node.style.cssText = "";
}

function setStyleProperty(node, name, value) {
  if (!node.style) node.style = {};
  if (node.style.setProperty) {
    node.style.setProperty(cssStylePropertyName(name), String(value));
  } else {
    node.style[jsStylePropertyName(name)] = String(value);
  }
}

function removeStyleProperty(node, name) {
  if (!node.style) return;
  if (node.style.removeProperty) {
    node.style.removeProperty(cssStylePropertyName(name));
  } else {
    delete node.style[jsStylePropertyName(name)];
  }
}

function cssStylePropertyName(name) {
  const property = stylePropertyName(name);
  if (property.startsWith("--")) return property;
  return property.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
}

function jsStylePropertyName(name) {
  const property = stylePropertyName(name);
  if (property.startsWith("--")) return property;
  return property.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
}

function stylePropertyName(name) {
  return String(name);
}

export function setEvent(instance, slot, node, eventName, messageForEvent, dispatch) {
  const current = instance.eventSlots[slot] || {};
  if (current.listener && (current.node !== node || current.eventName !== eventName)) {
    current.node?.removeEventListener?.(current.eventName, current.listener);
    current.listener = null;
  }

  current.node = node;
  current.eventName = eventName;
  current.messageForEvent = messageForEvent;
  current.dispatch = dispatch || (() => {});

  if (!current.listener) {
    current.listener = (event) => {
      dispatchTemplateEventResult(current.messageForEvent(event), event, current.dispatch);
    };
    node.addEventListener(eventName, current.listener);
  }

  instance.eventSlots[slot] = current;
}

export function setCompiledEvent(instance, slot, node, eventName, messageForEvent, dispatch) {
  let current = instance.eventSlots[slot];
  if (current) {
    if (current.node !== node || current.eventName !== eventName) {
      if (current.delegated) removeDelegatedCompiledEventSlot(current);
      else current.node?.removeEventListener?.(current.eventName, current.listener);
      current = null;
    } else {
      current.messageForEvent = messageForEvent;
      current.dispatch = dispatch;
      return;
    }
  }

  if (canDelegateCompiledEvent(eventName)) {
    current = {
      node,
      eventName,
      messageForEvent,
      dispatch,
      delegated: true
    };
    setDelegatedCompiledEventSlot(node, eventName, current);
    instance.eventSlots[slot] = current;
    return;
  }

  current = {
    node,
    eventName,
    messageForEvent,
    dispatch,
    listener(event) {
      dispatchTemplateEventResult(current.messageForEvent(event), event, current.dispatch);
    }
  };
  node.addEventListener(eventName, current.listener);
  instance.eventSlots[slot] = current;
}

const delegatedCompiledEvents = new Set(["click"]);
const delegatedCompiledEventListeners = new Map();
const delegatedCompiledEventSlots = new WeakMap();

function canDelegateCompiledEvent(eventName) {
  return delegatedCompiledEvents.has(eventName) && typeof document !== "undefined" && typeof document.addEventListener === "function";
}

function setDelegatedCompiledEventSlot(node, eventName, current) {
  let slots = delegatedCompiledEventSlots.get(node);
  if (!slots) {
    slots = new Map();
    delegatedCompiledEventSlots.set(node, slots);
  }
  slots.set(eventName, current);
  ensureDelegatedCompiledEventListener(eventName);
}

function removeDelegatedCompiledEventSlot(current) {
  const slots = delegatedCompiledEventSlots.get(current.node);
  if (!slots) return;
  slots.delete(current.eventName);
}

function ensureDelegatedCompiledEventListener(eventName) {
  if (delegatedCompiledEventListeners.has(eventName)) return;
  const listener = (event) => dispatchDelegatedCompiledEvent(eventName, event);
  document.addEventListener(eventName, listener);
  delegatedCompiledEventListeners.set(eventName, listener);
}

function dispatchDelegatedCompiledEvent(eventName, event) {
  let node = event.target;
  if (node?.nodeType === 3) node = node.parentNode;

  while (node && node !== document) {
    const current = delegatedCompiledEventSlots.get(node)?.get(eventName);
    if (current && !current.disposed) {
      dispatchTemplateEventResult(current.messageForEvent(delegatedCompiledEvent(event, node)), event, current.dispatch);
      if (event.cancelBubble) return;
    }
    node = node.parentNode;
  }
}

function delegatedCompiledEvent(event, currentTarget) {
  if (event.currentTarget === currentTarget) return event;
  return new Proxy(event, {
    get(target, property, receiver) {
      if (property === "currentTarget") return currentTarget;
      const value = Reflect.get(target, property, receiver);
      return typeof value === "function" ? value.bind(target) : value;
    }
  });
}

function dispatchTemplateEventResult(result, event, dispatch) {
  if (result?.__ce === 1) {
    if (result.p) event.preventDefault();
    if (result.s) event.stopPropagation();
    if (result.m !== undefined && result.m !== null) dispatch(result.m, event);
    return;
  }

  if (result !== undefined && result !== null) dispatch(result, event);
}

export function setRef(instance, slot, node, value, dispatch) {
  const registry = registryForDispatch(dispatch);
  const name = refName(value);
  const current = instance.refSlots[slot];

  if (current && (current.registry !== registry || current.name !== name || current.node !== node)) {
    unregisterRef(current.registry, current.name, current.node);
  }

  if (!name) {
    instance.refSlots[slot] = null;
    return;
  }

  registry.set(name, node);
  instance.refSlots[slot] = { registry, name, node };
}

export function setCompiledRef(instance, slot, node, value, dispatch) {
  const registry = compiledRegistryForDispatch(dispatch);
  const name = compiledRefName(value);
  const current = instance.refSlots[slot];

  if (current && (current.registry !== registry || current.name !== name || current.node !== node)) {
    unregisterRef(current.registry, current.name, current.node);
  }

  if (!name) {
    instance.refSlots[slot] = null;
    return;
  }

  registry.set(name, node);
  instance.refSlots[slot] = { registry, name, node };
}

export function setKeyedList(instance, slot, marker, items, keyForItem, renderItem, dispatch, updateContext, itemName = null, indexName = null) {
  const parent = marker.parentNode;
  if (!parent) return;

  const current = instance.keyedSlots[slot] || { byKey: new Map(), duplicateKeys: new Map(), entries: [] };
  normalizeCompiledKeyedSlot(current);
  if (items.length === 0) {
    clearKeyedEntries(parent, marker, current);
    instance.keyedSlots[slot] = current;
    return;
  }
  const slotMetadata = itemName ? null : instance.definition?.slots?.[slot] || {};
  const keyedKind = slotMetadata?.kind || {};
  itemName ||= typeof keyedKind.keyed === "string" ? keyedKind.keyed : null;
  indexName ||= typeof keyedKind.index === "string" ? keyedKind.index : null;
  const nextByKey = new Map();
  const nextDuplicateKeys = new Map();
  const seenKeys = new Map();
  const orderedEntries = [];

  let index = 0;
  for (const item of items) {
    const rawKey = keyForItem(item, index);
    const occurrence = seenKeys.get(rawKey) || 0;
    seenKeys.set(rawKey, occurrence + 1);
    const key = occurrence === 0 ? rawKey : duplicateStorageKey(current, nextDuplicateKeys, rawKey, occurrence);
    let entry = current.byKey.get(key);
    if (!entry) {
      const component = renderItem(item, index);
      updateKeyedComponent(component, item, index, dispatch, forceUpdateContext(updateContext));
      entry = { key, component, item, index, oldIndex: -1 };
    } else {
      entry.oldIndex = entry.index;
      const itemUpdateContext = keyedItemUpdateContext(
        updateContext,
        itemName,
        indexName,
        entry.item,
        item,
        entry.index,
        index
      );
      updateKeyedComponent(entry.component, item, index, dispatch, itemUpdateContext);
      entry.item = item;
      entry.index = index;
    }

    nextByKey.set(key, entry);
    orderedEntries.push(entry);
    index += 1;
  }

  reorderKeyedEntries(parent, marker, orderedEntries);

  for (const [key, entry] of current.byKey) {
    if (!nextByKey.has(key)) {
      if (entry.component.root?.parentNode?.removeChild) {
        entry.component.root.parentNode.removeChild(entry.component.root);
      }
      disposeComponent(entry.component);
    }
  }

  current.byKey = nextByKey;
  current.duplicateKeys = nextDuplicateKeys;
  instance.keyedSlots[slot] = current;
}

export function setCompiledKeyedList(instance, slot, marker, items, keyForItem, renderItem, dispatch, stableItemUpdate = false) {
  const parent = marker.parentNode;
  if (!parent) return;

  const current = instance.keyedSlots[slot] || { byKey: new Map(), duplicateKeys: new Map() };
  if (!current.duplicateKeys) current.duplicateKeys = new Map();
  if (items.length === 0) {
    clearKeyedEntries(parent, marker, current);
    instance.keyedSlots[slot] = current;
    return;
  }
  if (
    current.duplicateKeys.size === 0 &&
    updateCompiledKeyedListSameOrder(current, items, keyForItem, dispatch, stableItemUpdate)
  ) {
    instance.keyedSlots[slot] = current;
    return;
  }
  if (
    current.duplicateKeys.size === 0 &&
    updateCompiledKeyedListSameSequence(current, parent, marker, items, keyForItem, renderItem, dispatch, stableItemUpdate)
  ) {
    instance.keyedSlots[slot] = current;
    return;
  }
  if (
    current.duplicateKeys.size === 0 &&
    setCompiledKeyedListUnique(current, parent, marker, items, keyForItem, renderItem, dispatch, stableItemUpdate)
  ) {
    instance.keyedSlots[slot] = current;
    return;
  }

  setCompiledKeyedListWithDuplicates(current, parent, marker, items, keyForItem, renderItem, dispatch, stableItemUpdate);
  instance.keyedSlots[slot] = current;
}

function updateCompiledKeyedListSameOrder(current, items, keyForItem, dispatch, stableItemUpdate) {
  if (current.byKey.size !== items.length || current.byKey.size === 0) return false;

  let index = 0;
  for (const entry of current.entries) {
    if (!sameMapKey(entry.key, keyForItem(items[index], index))) return false;
    index += 1;
  }

  index = 0;
  for (const entry of current.entries) {
    const item = items[index];
    if (!canSkipCompiledKeyedEntry(entry, item, index, dispatch, stableItemUpdate)) {
      updateCompiledKeyedEntry(entry, item, index, dispatch);
      entry.dispatch = dispatch;
    }
    entry.item = item;
    entry.index = index;
    index += 1;
  }
  return true;
}

function updateCompiledKeyedListSameSequence(current, parent, marker, items, keyForItem, renderItem, dispatch, stableItemUpdate) {
  const currentSize = current.byKey.size;
  if (currentSize === 0 || currentSize === items.length) return false;

  if (items.length > currentSize) {
    let index = 0;
    for (const entry of current.entries) {
      if (!sameMapKey(entry.key, keyForItem(items[index], index))) return false;
      index += 1;
    }

    const nextByKey = new Map(current.byKey);
    const appendedEntries = [];
    for (; index < items.length; index += 1) {
      const item = items[index];
      const key = keyForItem(item, index);
      if (nextByKey.has(key)) {
        disposeNewKeyedEntries(appendedEntries);
        return false;
      }
      const component = renderItem(item, index);
      const entry = { key, component, arity: compiledComponentArity(component), item, index, oldIndex: -1 };
      appendedEntries.push(entry);
      nextByKey.set(key, entry);
    }

    index = 0;
    for (const entry of current.entries) {
      const item = items[index];
      if (!canSkipCompiledKeyedEntry(entry, item, index, dispatch, stableItemUpdate)) {
        updateCompiledKeyedEntry(entry, item, index, dispatch);
        entry.dispatch = dispatch;
      }
      entry.item = item;
      entry.index = index;
      index += 1;
    }
    for (const entry of appendedEntries) {
      updateCompiledKeyedEntry(entry, entry.item, entry.index, dispatch);
      entry.dispatch = dispatch;
    }
    insertKeyedEntries(parent, marker, appendedEntries);

    current.byKey = nextByKey;
    current.duplicateKeys = new Map();
    current.entries = current.entries.concat(appendedEntries);
    return true;
  }

  let index = 0;
  const removed = [];
  const nextByKey = new Map();
  const nextEntries = [];
  for (const entry of current.entries) {
    if (index < items.length && sameMapKey(entry.key, keyForItem(items[index], index))) {
      nextByKey.set(entry.key, entry);
      nextEntries.push(entry);
      index += 1;
    } else {
      removed.push(entry);
    }
  }
  if (index !== items.length) return false;

  const removedEntries = removed.length > 0 ? new Set(removed) : null;
  index = 0;
  for (const entry of current.entries) {
    if (removedEntries?.has(entry)) continue;
    const item = items[index];
    if (!canSkipCompiledKeyedEntry(entry, item, index, dispatch, stableItemUpdate)) {
      updateCompiledKeyedEntry(entry, item, index, dispatch);
      entry.dispatch = dispatch;
    }
    entry.item = item;
    entry.index = index;
    index += 1;
  }
  for (const entry of removed) {
    if (entry.component.root?.parentNode?.removeChild) {
      entry.component.root.parentNode.removeChild(entry.component.root);
    }
    disposeComponent(entry.component);
  }

  current.byKey = nextByKey;
  current.duplicateKeys = new Map();
  current.entries = nextEntries;
  return true;
}

function setCompiledKeyedListUnique(current, parent, marker, items, keyForItem, renderItem, dispatch, stableItemUpdate) {
  const nextByKey = new Map();
  const orderedEntries = [];
  let needsReorder = false;
  let lastOldIndex = -1;
  let reusedEntries = 0;

  let index = 0;
  for (const item of items) {
    const key = keyForItem(item, index);
    if (nextByKey.has(key)) {
      disposeNewKeyedEntries(orderedEntries);
      return false;
    }
    let entry = current.byKey.get(key);
    if (!entry) {
      const component = renderItem(item, index);
      entry = { key, component, arity: compiledComponentArity(component), item, index, oldIndex: -1 };
      needsReorder = true;
    } else {
      entry.oldIndex = entry.index;
      reusedEntries += 1;
      if (entry.oldIndex < lastOldIndex) needsReorder = true;
      lastOldIndex = entry.oldIndex;
    }
    if (!canSkipCompiledKeyedEntry(entry, item, index, dispatch, stableItemUpdate)) {
      updateCompiledKeyedEntry(entry, item, index, dispatch);
      entry.dispatch = dispatch;
    }
    entry.item = item;
    entry.index = index;
    nextByKey.set(key, entry);
    orderedEntries.push(entry);
    index += 1;
  }

  if (current.byKey.size > 0 && reusedEntries === 0) {
    clearKeyedEntries(parent, marker, current);
    needsReorder = true;
  }

  if (reusedEntries === 0) insertKeyedEntries(parent, marker, orderedEntries);
  else if (needsReorder) reorderKeyedEntries(parent, marker, orderedEntries);

  for (const [key, entry] of current.byKey) {
    if (!nextByKey.has(key)) {
      if (entry.component.root?.parentNode?.removeChild) {
        entry.component.root.parentNode.removeChild(entry.component.root);
      }
      disposeComponent(entry.component);
    }
  }

  current.byKey = nextByKey;
  current.duplicateKeys = new Map();
  current.entries = orderedEntries;
  return true;
}

function setCompiledKeyedListWithDuplicates(current, parent, marker, items, keyForItem, renderItem, dispatch, stableItemUpdate) {
  const nextByKey = new Map();
  const nextDuplicateKeys = new Map();
  const seenKeys = new Map();
  const orderedEntries = [];
  let needsReorder = false;
  let lastOldIndex = -1;
  let reusedEntries = 0;

  let index = 0;
  for (const item of items) {
    const rawKey = keyForItem(item, index);
    const occurrence = seenKeys.get(rawKey) || 0;
    seenKeys.set(rawKey, occurrence + 1);
    const key = occurrence === 0 ? rawKey : duplicateStorageKey(current, nextDuplicateKeys, rawKey, occurrence);
    let entry = current.byKey.get(key);
    if (!entry) {
      const component = renderItem(item, index);
      entry = { key, component, arity: compiledComponentArity(component), item, index, oldIndex: -1 };
      needsReorder = true;
    } else {
      entry.oldIndex = entry.index;
      reusedEntries += 1;
      if (entry.oldIndex < lastOldIndex) needsReorder = true;
      lastOldIndex = entry.oldIndex;
    }
    if (!canSkipCompiledKeyedEntry(entry, item, index, dispatch, stableItemUpdate)) {
      updateCompiledKeyedEntry(entry, item, index, dispatch);
      entry.dispatch = dispatch;
    }
    entry.item = item;
    entry.index = index;
    nextByKey.set(key, entry);
    orderedEntries.push(entry);
    index += 1;
  }

  if (current.byKey.size > 0 && reusedEntries === 0) {
    clearKeyedEntries(parent, marker, current);
    needsReorder = true;
  }

  if (reusedEntries === 0) insertKeyedEntries(parent, marker, orderedEntries);
  else if (needsReorder) reorderKeyedEntries(parent, marker, orderedEntries);

  for (const [key, entry] of current.byKey) {
    if (!nextByKey.has(key)) {
      if (entry.component.root?.parentNode?.removeChild) {
        entry.component.root.parentNode.removeChild(entry.component.root);
      }
      disposeComponent(entry.component);
    }
  }

  current.byKey = nextByKey;
  current.duplicateKeys = nextDuplicateKeys;
  current.entries = orderedEntries;
}

function clearKeyedEntries(parent, marker, current) {
  if (current.byKey.size === 0) {
    current.duplicateKeys = new Map();
    return;
  }

  let firstRoot = null;
  for (const entry of current.entries) {
    const root = entry.component.root;
    if (root?.parentNode === parent) {
      firstRoot = root;
      break;
    }
  }

  let detached = false;
  const documentRef = parent.ownerDocument || (typeof document !== "undefined" ? document : null);
  if (firstRoot && documentRef?.createRange) {
    const range = documentRef.createRange();
    range.setStartBefore(firstRoot);
    range.setEndBefore(marker);
    range.deleteContents();
    detached = true;
  }

  for (const entry of current.entries) {
    if (detached && canDropDetachedComponent(entry.component)) continue;
    disposeComponent(entry.component, detached);
  }
  current.byKey = new Map();
  current.duplicateKeys = new Map();
  current.entries = [];
}

function normalizeCompiledKeyedSlot(current) {
  if (!current.duplicateKeys) current.duplicateKeys = new Map();
  if (!Array.isArray(current.entries)) current.entries = Array.from(current.byKey.values());
}

function disposeNewKeyedEntries(entries) {
  for (const entry of entries) {
    if (entry.oldIndex < 0) disposeComponent(entry.component);
  }
}

function insertKeyedEntries(parent, marker, entries) {
  if (entries.length === 0) return;
  const documentRef = parent.ownerDocument || (typeof document !== "undefined" ? document : null);
  if (!documentRef?.createDocumentFragment || entries.length === 1) {
    for (const entry of entries) parent.insertBefore(entry.component.root, marker);
    return;
  }
  const fragment = documentRef.createDocumentFragment();
  for (const entry of entries) fragment.appendChild(entry.component.root);
  parent.insertBefore(fragment, marker);
}

function reorderKeyedEntries(parent, marker, orderedEntries) {
  if (reorderTwoMovedKeyedEntries(parent, orderedEntries)) return;

  const previousIndexes = orderedEntries.map((entry) => entry.oldIndex ?? -1);
  const stableIndexes = longestIncreasingSubsequenceIndexes(previousIndexes);
  let stableCursor = stableIndexes.length - 1;
  let cursor = marker;

  for (let index = orderedEntries.length - 1; index >= 0; index -= 1) {
    const root = orderedEntries[index].component.root;
    if (stableCursor >= 0 && stableIndexes[stableCursor] === index) {
      cursor = root;
      stableCursor -= 1;
      continue;
    }
    if (root.parentNode !== parent || root.nextSibling !== cursor) {
      parent.insertBefore(root, cursor);
    }
    cursor = root;
  }
}

function reorderTwoMovedKeyedEntries(parent, orderedEntries) {
  let first = -1;
  let second = -1;
  for (let index = 0; index < orderedEntries.length; index += 1) {
    const oldIndex = orderedEntries[index].oldIndex ?? -1;
    if (oldIndex === index) continue;
    if (oldIndex < 0) return false;
    if (first < 0) first = index;
    else if (second < 0) second = index;
    else return false;
  }

  if (first < 0) return true;
  if (second < 0) return false;
  const firstEntry = orderedEntries[first];
  const secondEntry = orderedEntries[second];
  if (firstEntry.oldIndex !== second || secondEntry.oldIndex !== first) return false;

  const firstRoot = firstEntry.component.root;
  const secondRoot = secondEntry.component.root;
  if (firstRoot?.parentNode !== parent || secondRoot?.parentNode !== parent) return false;

  const afterFirstRoot = firstRoot.nextSibling;
  parent.insertBefore(firstRoot, secondRoot);
  parent.insertBefore(secondRoot, afterFirstRoot);
  return true;
}

function longestIncreasingSubsequenceIndexes(values) {
  const predecessors = new Array(values.length);
  const result = [];

  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value < 0) continue;

    let low = 0;
    let high = result.length;
    while (low < high) {
      const mid = (low + high) >> 1;
      if (values[result[mid]] < value) low = mid + 1;
      else high = mid;
    }

    if (low > 0) predecessors[index] = result[low - 1];
    result[low] = index;
  }

  let cursor = result[result.length - 1];
  for (let index = result.length - 1; index >= 0; index -= 1) {
    result[index] = cursor;
    cursor = predecessors[cursor];
  }
  return result;
}

function sameMapKey(left, right) {
  return left === right || (left !== left && right !== right);
}

function duplicateStorageKey(current, nextDuplicateKeys, rawKey, occurrence) {
  const currentByOccurrence = current.duplicateKeys?.get(rawKey);
  let key = currentByOccurrence?.get(occurrence);
  if (!key) key = { rawKey, occurrence };

  let nextByOccurrence = nextDuplicateKeys.get(rawKey);
  if (!nextByOccurrence) {
    nextByOccurrence = new Map();
    nextDuplicateKeys.set(rawKey, nextByOccurrence);
  }
  nextByOccurrence.set(occurrence, key);
  return key;
}

function updateKeyedComponent(component, item, index, dispatch, updateContext) {
  if (component.update.length >= 4) {
    return component.update(item, index, dispatch, updateContext);
  }
  return component.update(item, dispatch, updateContext);
}

function updateCompiledKeyedEntry(entry, item, index, dispatch) {
  if (compiledEntryArity(entry) >= 2) {
    return entry.component.update(item, index, dispatch);
  }
  return entry.component.update(item, dispatch);
}

function canSkipCompiledKeyedEntry(entry, item, index, dispatch, stableItemUpdate) {
  if (!stableItemUpdate || entry.item !== item || entry.dispatch !== dispatch) return false;
  return compiledEntryArity(entry) < 2 || entry.index === index;
}

function compiledEntryArity(entry) {
  return entry.arity ??= compiledComponentArity(entry.component);
}

function compiledComponentArity(component) {
  return component?.__closkellArity ?? 0;
}

function keyedItemUpdateContext(updateContext, itemName, indexName, previousItem, nextItem, previousIndex, nextIndex) {
  if (!updateContext || !itemName) return updateContext;
  const localChangedPaths = changedStatePaths(previousItem, nextItem, itemName);
  if (indexName && !Object.is(previousIndex, nextIndex)) {
    localChangedPaths.push(indexName);
  }
  const localReadPrefixes = [itemName];
  if (indexName) localReadPrefixes.push(indexName);
  return localUpdateContext(updateContext, localChangedPaths, localReadPrefixes);
}

function localUpdateContext(updateContext, localChangedPaths, localReadPrefixes) {
  if (!updateContext) return updateContext;
  return {
    ...updateContext,
    localChangedPaths: [...(updateContext.localChangedPaths || []), ...localChangedPaths],
    localReadPrefixes: [...(updateContext.localReadPrefixes || []), ...localReadPrefixes],
    frames: updateContext.frames
  };
}

export function setConditional(instance, slot, marker, condition, renderThen, renderElse, dispatch, updateContext) {
  const parent = marker.parentNode;
  if (!parent) return;

  const nextBranch = condition ? "then" : "else";
  const render = condition ? renderThen : renderElse;
  let current = instance.conditionalSlots[slot] || {};

  if (current.branch !== nextBranch) {
    if (current.component?.root?.parentNode) {
      current.component.root.parentNode.removeChild(current.component.root);
    }
    disposeComponent(current.component);
    current = {
      branch: nextBranch,
      component: typeof render === "function" ? render() : null,
      fresh: true
    };
  }

  if (current.component) {
    current.component.update(dispatch, current.fresh ? forceUpdateContext(updateContext) : updateContext);
    current.fresh = false;
    if (current.component.root.parentNode !== parent) {
      parent.insertBefore(current.component.root, marker);
    }
  }

  instance.conditionalSlots[slot] = current;
}

export function setCompiledConditional(instance, slot, marker, condition, renderThen, renderElse, dispatch) {
  const parent = marker.parentNode;
  if (!parent) return;

  const nextBranch = condition ? "then" : "else";
  const render = condition ? renderThen : renderElse;
  let current = instance.conditionalSlots[slot] || {};

  if (current.branch !== nextBranch) {
    if (current.component?.root?.parentNode) {
      current.component.root.parentNode.removeChild(current.component.root);
    }
    disposeComponent(current.component);
    current = {
      branch: nextBranch,
      component: render()
    };
  }

  if (current.component) {
    current.component.update(dispatch);
    if (current.component.root.parentNode !== parent) {
      parent.insertBefore(current.component.root, marker);
    }
  }

  instance.conditionalSlots[slot] = current;
}

export function setComponent(instance, slot, marker, render, args, dispatch, updateContext, expectedKey = null) {
  const parent = marker.parentNode;
  if (!parent) return;

  const current = instance.componentSlots[slot] || {};
  const canReuseExpected = expectedKey && current.component && current.renderKey === expectedKey;
  const rendered = canReuseExpected ? current.component : (typeof render === "function" ? render() : render);
  const renderedKey = canReuseExpected ? current.renderKey : (componentRenderKey(rendered) || expectedKey);
  let fresh = false;
  if (!current.component) {
    current.component = rendered;
    current.renderKey = renderedKey;
    fresh = true;
  } else if (
    rendered
    && rendered !== current.component
    && renderedKey
    && current.renderKey
    && renderedKey !== current.renderKey
  ) {
    if (current.component.root?.parentNode) {
      current.component.root.parentNode.removeChild(current.component.root);
    }
    disposeComponent(current.component);
    current.component = rendered;
    current.renderKey = renderedKey;
    current.args = [];
    fresh = true;
  } else if (rendered && rendered !== current.component) {
    disposeComponent(rendered);
  }

  if (current.component) {
    const nextArgs = Array.isArray(args) ? args : [];
    const params = componentParams(current.component);
    const componentContext = fresh
      ? forceUpdateContext(updateContext)
      : componentUpdateContext(updateContext, params, current.args, nextArgs);
    if (params.length) {
      current.component.update(...nextArgs, dispatch, componentContext);
      current.args = nextArgs;
    } else {
      current.component.update(dispatch, componentContext);
      current.args = [];
    }
    if (current.component.root.parentNode !== parent) {
      parent.insertBefore(current.component.root, marker);
    }
  }

  instance.componentSlots[slot] = current;
}

export function setCompiledComponent(instance, slot, marker, render, args, dispatch, expectedKey) {
  const parent = marker.parentNode;
  if (!parent) return;

  const current = instance.componentSlots[slot] || {};
  const currentArity = compiledComponentArity(current.component);
  const shouldRecreate =
    !current.component ||
    current.renderKey !== expectedKey ||
    (args.length > 0 && currentArity === 0 && !sameCompiledComponentArgs(current.args, args));
  if (shouldRecreate) {
    if (current.component?.root?.parentNode) {
      current.component.root.parentNode.removeChild(current.component.root);
    }
    disposeComponent(current.component);
    current.component = render();
    current.renderKey = expectedKey;
    current.args = args.slice();
  }

  if (current.component) {
    const arity = compiledComponentArity(current.component);
    if (arity > 0) {
      current.component.update(...args.slice(0, arity), dispatch);
    } else {
      current.component.update(dispatch);
    }
    if (current.component.root.parentNode !== parent) {
      parent.insertBefore(current.component.root, marker);
    }
  }

  instance.componentSlots[slot] = current;
}

function sameCompiledComponentArgs(previous = [], next = []) {
  if (previous.length !== next.length) return false;
  for (let index = 0; index < next.length; index += 1) {
    if (!Object.is(previous[index], next[index])) return false;
  }
  return true;
}

function componentRenderKey(component) {
  return component?.definition?.name || null;
}

function componentParams(component) {
  const params = component?.definition?.params;
  return Array.isArray(params) ? params : [];
}

function disposeComponent(component, detached = false) {
  if (!component) return;
  component.dispose(detached);
}

function canDropDetachedComponent(component) {
  const instance = component?.__closkellInstance;
  if (!instance) return false;
  return (
    slotsEmpty(instance.refSlots) &&
    slotsEmpty(instance.keyedSlots) &&
    slotsEmpty(instance.conditionalSlots) &&
    slotsEmpty(instance.componentSlots)
  );
}

function slotsEmpty(slots) {
  return !Array.isArray(slots) || slots.every((slot) => !slot);
}

function componentUpdateContext(updateContext, params, previousArgs = [], nextArgs = []) {
  if (!updateContext || !Array.isArray(params) || !params.length) return updateContext;
  const localChangedPaths = [];
  for (let index = 0; index < params.length; index += 1) {
    const param = params[index];
    if (typeof param !== "string" || !param) continue;
    localChangedPaths.push(...changedStatePaths(previousArgs[index], nextArgs[index], param));
  }
  return localUpdateContext(updateContext, localChangedPaths, params);
}

function disposeEventSlots(instance, detached = false) {
  for (const current of instance.eventSlots) {
    if (!current) continue;
    if (current.delegated) {
      if (detached) {
        current.disposed = true;
        current.messageForEvent = null;
        current.dispatch = null;
      } else {
        removeDelegatedCompiledEventSlot(current);
      }
    }
    else current.node?.removeEventListener?.(current.eventName, current.listener);
  }
  instance.eventSlots = [];
}

function disposeRefs(instance) {
  for (const current of instance.refSlots) if (current) unregisterRef(current.registry, current.name, current.node);
  instance.refSlots = [];
}

function registryForDispatch(dispatch) {
  if (!dispatch || (typeof dispatch !== "function" && typeof dispatch !== "object")) return new Map();
  if (!dispatch.__closkellRefs) dispatch.__closkellRefs = new Map();
  return dispatch.__closkellRefs;
}

function compiledRegistryForDispatch(dispatch) {
  return dispatch.__closkellRefs ??= new Map();
}

function unregisterRef(registry, name, node) {
  if (!registry || !name) return;
  if (registry.get(name) === node) registry.delete(name);
}

function refName(value) {
  if (value === false || value == null) return null;
  if (typeof value === "symbol") return Symbol.keyFor(value) || value.description || null;
  return String(value);
}

export const Cmd = {
  none() {
    return { kind: Symbol.for("none") };
  },
  batch(commands) {
    return { kind: Symbol.for("batch"), commands };
  },
  bluetoothRequestDevice(options, onSuccess, onError) {
    return { kind: Symbol.for("bluetooth/request-device"), options, onSuccess, onError };
  },
  bluetoothConnectHeartRate(id, options, onSuccess, onReading, onDisconnected, onError) {
    return {
      kind: Symbol.for("bluetooth/connect-heart-rate"),
      id,
      options,
      onSuccess,
      onReading,
      onDisconnected,
      onError
    };
  },
  bluetoothDisconnect(id, msg) {
    return { kind: Symbol.for("bluetooth/disconnect"), id, msg };
  },
  timerAfter(ms, msg, id) {
    return { kind: Symbol.for("timer/after"), ms, msg, id };
  },
  timerEvery(ms, msg, id) {
    return { kind: Symbol.for("timer/every"), ms, msg, id };
  },
  timerCancel(id) {
    return { kind: Symbol.for("timer/cancel"), id };
  },
  animationFrame(onFrame, id, msg) {
    if (commandOptions(id)) return { kind: Symbol.for("animation/frame"), onFrame, ...id };
    return { kind: Symbol.for("animation/frame"), onFrame, id, msg };
  },
  animationCancel(id, msg) {
    if (commandOptions(msg)) return { kind: Symbol.for("animation/cancel"), id, ...msg };
    return { kind: Symbol.for("animation/cancel"), id, msg };
  },
  timeNow(onSuccess, onError) {
    return { kind: Symbol.for("time/now"), onSuccess, onError };
  },
  storageGet(key, onSuccess, onError, format) {
    return { kind: Symbol.for("storage/get"), key, onSuccess, onError, format };
  },
  storageSet(key, value, msg, onError) {
    return { kind: Symbol.for("storage/set"), key, value, msg, onError };
  },
  storageRemove(key, msg, onError) {
    if (plainObject(msg)) return { kind: Symbol.for("storage/remove"), key, ...msg };
    return { kind: Symbol.for("storage/remove"), key, msg, onError };
  },
  randomNumber(min = 0, max = 1, onSuccess, onError) {
    return { kind: Symbol.for("random/number"), min, max, onSuccess, onError };
  },
  simulationHeartRate(id, options = {}, onSuccess, onReading, onDisconnected, onError) {
    return {
      kind: Symbol.for("simulation/heart-rate"),
      id,
      ...options,
      onSuccess,
      onReading,
      onDisconnected,
      onError
    };
  },
  simulationStop(id, msg, onSuccess, onError) {
    return { kind: Symbol.for("simulation/stop"), id, msg, onSuccess, onError };
  },
  httpRequest(request, onSuccess, onError, response) {
    if (typeof request === "string") {
      if (plainObject(onSuccess)) {
        return { kind: Symbol.for("http/request"), ...onSuccess, url: request };
      }
      return { kind: Symbol.for("http/request"), url: request, onSuccess, onError, response };
    }
    return { kind: Symbol.for("http/request"), request, onSuccess, onError, response };
  },
  fileDownload(name, content, mime = "application/octet-stream", msg, onError) {
    return { kind: Symbol.for("file/download"), name, content, mime, msg, onError };
  },
  fileImport(accept = "", format = "text", onSuccess, onError) {
    return { kind: Symbol.for("file/import"), accept, format, onSuccess, onError };
  },
  fileReadSelected(ref, format = "text", onSuccess, onError) {
    return { kind: Symbol.for("file/read-selected"), ref, format, onSuccess, onError };
  },
  canvasDraw(ref, ops, msg, onError, options = {}) {
    return { kind: Symbol.for("canvas/draw"), ref, ops, msg, onError, ...options };
  },
  canvasMeasureText(ref, texts, onSuccess, font, onError) {
    return { kind: Symbol.for("canvas/measure-text"), ref, texts, onSuccess, font, onError };
  },
  domRefFocus(ref, msg, onError) {
    if (commandOptions(msg)) return { kind: Symbol.for("dom-ref/focus"), ref, ...msg };
    return { kind: Symbol.for("dom-ref/focus"), ref, msg, onError };
  },
  domRefClick(ref, msg, onError) {
    if (commandOptions(msg)) return { kind: Symbol.for("dom-ref/click"), ref, ...msg };
    return { kind: Symbol.for("dom-ref/click"), ref, msg, onError };
  },
  domRefMeasure(ref, onSuccess, onError) {
    return { kind: Symbol.for("dom-ref/measure"), ref, onSuccess, onError };
  },
  domRefResizeWatch(ref, onChange, id, onError) {
    return { kind: Symbol.for("dom-ref/resize-watch"), ref, onChange, id, onError };
  },
  domRefResizeUnwatch(id, msg) {
    if (commandOptions(msg)) return { kind: Symbol.for("dom-ref/resize-unwatch"), id, ...msg };
    return { kind: Symbol.for("dom-ref/resize-unwatch"), id, msg };
  },
  windowEventWatch(type, onEvent, id, options, onError) {
    return { kind: Symbol.for("window/event-watch"), type, onEvent, id, options, onError };
  },
  windowEventUnwatch(id, msg) {
    if (commandOptions(msg)) return { kind: Symbol.for("window/event-unwatch"), id, ...msg };
    return { kind: Symbol.for("window/event-unwatch"), id, msg };
  },
  mediaQueryWatch(query, onChange, id, onError) {
    return { kind: Symbol.for("media-query/watch"), query, onChange, id, onError };
  },
  mediaQueryUnwatch(id, msg) {
    if (commandOptions(msg)) return { kind: Symbol.for("media-query/unwatch"), id, ...msg };
    return { kind: Symbol.for("media-query/unwatch"), id, msg };
  }
};

export const Sub = {
  none: { kind: Symbol.for("none") },
  batch(subscriptions) {
    return { kind: Symbol.for("batch"), subscriptions };
  },
  timerEvery(id, ms, msg) {
    return { kind: Symbol.for("sub/timer/every"), id, ms, msg };
  },
  domRefResize(ref, onChange, id, onError) {
    return { kind: Symbol.for("sub/dom-ref/resize"), ref, onChange, id: id || ref, onError };
  },
  windowEvent(type, onEvent, id, options, onError) {
    return { kind: Symbol.for("sub/window/event"), type, onEvent, id: id || type, options, onError };
  },
  mediaQuery(query, onChange, id, onError) {
    return { kind: Symbol.for("sub/media-query"), query, onChange, id: id || query, onError };
  }
};

export const Task = {
  succeed(value) {
    return { kind: Symbol.for("task/succeed"), value };
  },
  fail(error) {
    return { kind: Symbol.for("task/fail"), error };
  },
  map(task, mapper) {
    return { kind: Symbol.for("task/map"), task, mapper };
  },
  mapError(task, mapper) {
    return { kind: Symbol.for("task/map-error"), task, mapper };
  },
  andThen(task, next) {
    return { kind: Symbol.for("task/and-then"), task, next };
  },
  perform(task, onSuccess, onError) {
    return { kind: Symbol.for("task/perform"), task, onSuccess, onError };
  }
};

export const Http = {
  getText(url, options) {
    return { kind: Symbol.for("task/http/get-text"), url, options };
  },
  getJson(url, options) {
    return { kind: Symbol.for("task/http/get-json"), url, options };
  }
};

export const Decoder = {
  string: primitiveDecoder("String", (value) => typeof value === "string"),
  number: primitiveDecoder("Number", (value) => typeof value === "number" && Number.isFinite(value)),
  bool: primitiveDecoder("Bool", (value) => typeof value === "boolean"),
  keyword: primitiveDecoder("Keyword", (value) => typeof value === "symbol"),
  literal(expected) {
    return {
      decode(value, path = "value") {
        return decoderValueEqual(value, expected)
          ? { ok: true, value }
          : { ok: false, error: `${path} expected ${decoderFormatValue(expected)}` };
      }
    };
  },
  optional(decoder) {
    return {
      optional: true,
      decode(value, path = "value") {
        if (value == null) return { ok: true, value: null };
        return runDecoder(decoder, value, path);
      }
    };
  },
  vector(decoder) {
    return {
      decode(value, path = "value") {
        if (!Array.isArray(value)) return decoderTypeError(path, "Vector");
        const output = [];
        for (let index = 0; index < value.length; index += 1) {
          const decoded = runDecoder(decoder, value[index], `${path}[${index}]`);
          if (!decoded.ok) return decoded;
          output.push(decoded.value);
        }
        return { ok: true, value: output };
      }
    };
  },
  record(spec) {
    const entries = decoderSpecEntries(spec);
    return {
      decode(value, path = "value") {
        if (!plainObject(value)) return decoderTypeError(path, "Record");
        const output = {};
        for (const [field, decoder] of entries) {
          const fieldPath = decoderFieldPath(path, field);
          if (!hasOwn(value, field)) {
            if (decoder?.optional) {
              output[field] = null;
              continue;
            }
            return { ok: false, error: `${fieldPath} is required` };
          }
          const decoded = runDecoder(decoder, value[field], fieldPath);
          if (!decoded.ok) return decoded;
          output[field] = decoded.value;
        }
        return { ok: true, value: output };
      }
    };
  }
};

export function decode(decoder, value) {
  return runDecoder(decoder, value, "value");
}

export function describe(name, ...entries) {
  return {
    __closkellTestGroup: true,
    name: String(name ?? ""),
    tests: flattenTestEntries(entries)
  };
}

export function test(name, ...assertions) {
  return {
    __closkellTest: true,
    name: String(name ?? ""),
    assertions: flattenAssertions(assertions)
  };
}

export function expect_(actual, expected) {
  return { __closkellAssert: "equal", actual, expected };
}

export function expect_not_(actual, expected) {
  return { __closkellAssert: "not-equal", actual, expected };
}

export function expect_ok(actual) {
  return { __closkellAssert: "ok", actual };
}

export function expect_err(actual) {
  return { __closkellAssert: "err", actual };
}

export function expect_some(actual) {
  return { __closkellAssert: "some", actual };
}

export function expect_nil(actual) {
  return { __closkellAssert: "nil", actual };
}

export function expect_match(actual, pattern) {
  return { __closkellAssert: "match", actual, pattern };
}

export function expect_throws(thunk, expected) {
  return { __closkellAssert: "throws", thunk, expected };
}

export function collectCloskellTests(moduleExports) {
  const tests = "tests" in (moduleExports || {})
    ? flattenModuleTestEntries(moduleExports.tests, [], true)
    : Object.keys(moduleExports || {})
      .sort()
      .flatMap((name) => flattenModuleTestEntries(moduleExports[name], [], false));
  return { tests };
}

export function runCloskellTest(testValue, index = 0) {
  const name = closkellTestName(testValue, index);
  if (!testValue || typeof testValue !== "object") {
    return { name, ok: false, error: "expected a test record or closkell/test value" };
  }

  const assertions = closkellTestAssertions(testValue);
  if (assertions.length === 0) {
    return { name, ok: false, error: "expected at least one assertion" };
  }

  for (const assertion of assertions) {
    const result = runCloskellAssertion(assertion);
    if (!result.ok) return { name, ...result };
  }
  return { name, ok: true };
}

export function runCloskellAssertion(assertion) {
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
        : { ok: false, expected: "true", actual: formatTestValue(assertion.actual) };
    case "err":
      return assertion.actual?.ok === false
        ? { ok: true }
        : { ok: false, expected: "err", actual: formatTestValue(assertion.actual) };
    case "some":
      return assertion.actual != null
        ? { ok: true }
        : { ok: false, expected: "some value", actual: formatTestValue(assertion.actual) };
    case "nil":
      return assertion.actual == null
        ? { ok: true }
        : { ok: false, expected: "nil", actual: formatTestValue(assertion.actual) };
    case "match":
      return runMatchAssertion(assertion.actual, assertion.pattern);
    case "throws":
      return runThrowsAssertion(assertion.thunk, assertion.expected);
    default:
      return { ok: false, error: `unknown assertion kind \`${kind}\`` };
  }
}

export function registerVitestTests(moduleExports, api = {}) {
  const describeFn = api.describe || globalThis.describe;
  const testFn = api.test || api.it || globalThis.test || globalThis.it;
  if (typeof describeFn !== "function" || typeof testFn !== "function") {
    throw new Error("registerVitestTests requires Vitest describe and test functions.");
  }

  let index = 0;
  for (const entry of moduleTestEntries(moduleExports)) {
    index = registerVitestEntry(entry, describeFn, testFn, index);
  }
}

export function render(component) {
  const documentRef = ensureRuntimeDocument();
  const root = documentRef.createElement("main");
  root.setAttribute("data-closkell-test-root", "");
  const harness = {
    __closkellHarness: true,
    kind: "component",
    root,
    component,
    messages: [],
    events: [],
    frames: [],
    disposed: false
  };
  const dispatch = testDispatchForHarness(harness);
  harness.dispatch = dispatch;
  component?.mount?.(root, dispatch);
  return harness;
}

export function renderToString(viewOrComponent, state) {
  const documentRef = ensureRuntimeDocument();
  const root = documentRef.createElement("main");
  const component = typeof viewOrComponent === "function" && arguments.length >= 2
    ? viewOrComponent(state)
    : viewOrComponent;
  const dispatch = serverRenderDispatch();
  component?.mount?.(root, dispatch);
  annotateServerRenderedComponent(component);
  const html = root.innerHTML ?? (root.children || []).map(serializeTestNode).join("");
  component?.dispose?.();
  root.replaceChildren?.();
  return html;
}

export const render_to_string = renderToString;

export function rerender(harness, component) {
  if (!harness?.__closkellHarness) return harness;
  if (harness.disposed) return harness;
  if (harness.component?.root?.parentNode) harness.component.root.parentNode.removeChild(harness.component.root);
  harness.component?.dispose?.();
  harness.component = component;
  component?.mount?.(harness.root, harness.dispatch || testDispatchForHarness(harness));
  return harness;
}

export function dispose(harness) {
  if (!harness || harness.disposed) return null;
  if (harness.kind === "app") {
    harness.app?.dispose?.();
  } else {
    harness.component?.dispose?.();
  }
  harness.root?.replaceChildren?.();
  harness.disposed = true;
  return null;
}

export function find(harness, selector) {
  return harnessRoot(harness)?.querySelector?.(selector) ?? null;
}

export function find_all(harness, selector) {
  return Array.from(harnessRoot(harness)?.querySelectorAll?.(selector) ?? []);
}

export function text(harness, selector) {
  const node = selector == null ? harnessRoot(harness) : find(harness, selector);
  return node?.textContent ?? "";
}

export function html(harness, selector) {
  const node = selector == null ? harnessRoot(harness) : find(harness, selector);
  return node?.innerHTML ?? "";
}

export function attr(harness, selector, name) {
  const node = find(harness, selector);
  return node?.getAttribute?.(name) ?? null;
}

export function class_(harness, selector, name) {
  const node = find(harness, selector);
  const className = node?.getAttribute?.("class") ?? "";
  return className.split(/\s+/).filter(Boolean).includes(String(name));
}

export function style(harness, selector, name) {
  const node = find(harness, selector);
  const styleName = cssStylePropertyName(name);
  return node?.style?.getPropertyValue?.(styleName) ?? node?.style?.[jsStylePropertyName(name)] ?? "";
}

export function messages(harness) {
  return Array.from(harness?.messages ?? []);
}

export function commands(harness) {
  if (harness?.kind === "app") {
    return Array.from(harness.app?.commands ?? []).map((entry) => entry?.command ?? entry);
  }
  return Array.from(harness?.commands ?? []);
}

export function subscriptions(harness) {
  if (harness?.kind === "app") {
    return Array.from(harness.app?.subscriptions ?? []).map(testVisibleSubscription);
  }
  return Array.from(harness?.subscriptions ?? []);
}

function testVisibleSubscription(subscription) {
  if (!subscription || typeof subscription !== "object") return subscription;
  const kind = testVisibleSubscriptionKind(commandValueName(subscription.kind));
  if (!kind) return subscription;
  const { s: _stopKind, ...visible } = subscription;
  return { ...visible, kind };
}

function testVisibleSubscriptionKind(kind) {
  switch (kind) {
    case "timer/every":
      return "sub/timer/every";
    case "dom-ref/resize-watch":
    case "dom-ref/resize-watch/direct":
      return "sub/dom-ref/resize";
    case "window/event-watch":
    case "window/event-watch/direct":
      return "sub/window/event";
    case "media-query/watch":
    case "media-query/watch/direct":
      return "sub/media-query";
    case "simulation/heart-rate":
      return "sub/simulation/heart-rate";
    case "bluetooth/connect-heart-rate":
      return "sub/bluetooth/connect-heart-rate";
    default:
      return null;
  }
}

export function dispatch(harness, message) {
  return harness?.dispatch?.(message);
}

export function mount_app(appSpec = {}, options = {}) {
  const documentRef = ensureRuntimeDocument();
  const root = testOption(options, "root") || documentRef.createElement("main");
  root.setAttribute?.("data-closkell-app-test-root", "");
  const harness = {
    __closkellHarness: true,
    kind: "app",
    root,
    app: null,
    frames: [],
    subscriptionEvents: [],
    messages: [],
    disposed: false
  };
  const testDevtools = testOption(options, "devtools") || {
    emit(event) {
      if (event?.type === "template/update") harness.frames.push(event);
    }
  };
  const handlers =
    normalizeTestHandlers(testOption(options, "handlers") || testOption(options, "commandHandlers"))
    || createCommandHandlers(normalizeTestCommandEnv(options));
  const subscriptionHandlers =
    normalizeTestHandlers(testOption(options, "subscriptionHandlers"))
    || createSubscriptionHandlers({ ...normalizeTestCommandEnv(options), commandHandlers: handlers });
  const app = startApp({
    root,
    init: testOption(appSpec, "init"),
    update: testOption(appSpec, "update"),
    view: testOption(appSpec, "view"),
    subscriptions: testOption(appSpec, "subscriptions"),
    handlers,
    subscriptionHandlers,
    devtools: testDevtools,
    onSubscription(event) {
      harness.subscriptionEvents.push(event);
      testOption(options, "onSubscription")?.(event);
    },
    onCommand(event) {
      testOption(options, "onCommand")?.(event);
    }
  });
  harness.app = app;
  harness.dispatch = (message) => app.dispatch(message);
  Object.defineProperties(harness, {
    state: {
      get() {
        return app.state;
      }
    },
    activeSubscriptions: {
      get() {
        return app.subscriptions.map(testVisibleSubscription);
      }
    },
    commandLog: {
      get() {
        return app.commands;
      }
    }
  });
  return harness;
}

export const fire = {
  event(harness, selector, type, init = {}) {
    const node = find(harness, selector);
    if (!node) return null;
    return dispatchTestEvent(harness, node, type, init);
  },
  click(harness, selector, init = {}) {
    const node = find(harness, selector);
    if (!node) return null;
    return dispatchTestEvent(harness, node, "click", init);
  },
  input(harness, selector, valueOrInit = {}) {
    const node = find(harness, selector);
    if (!node) return null;
    const init = testEventInit(valueOrInit);
    applyTestInputValue(node, init);
    return dispatchTestEvent(harness, node, "input", init);
  },
  change(harness, selector, valueOrInit = {}) {
    const node = find(harness, selector);
    if (!node) return null;
    const init = testEventInit(valueOrInit);
    applyTestInputValue(node, init);
    return dispatchTestEvent(harness, node, "change", init);
  },
  keydown(harness, selector, init = {}) {
    const node = find(harness, selector);
    if (!node) return null;
    return dispatchTestEvent(harness, node, "keydown", init);
  },
  pointerdown(harness, selector, init = {}) {
    const node = find(harness, selector);
    if (!node) return null;
    return dispatchTestEvent(harness, node, "pointerdown", init);
  }
};

export function scopeUpdate(parentState, field, childMessage, update, tag) {
  const fieldName = scopeKey(field);
  const [childState, childCommand] = normalizeUpdateResult(update(parentState?.[fieldName], childMessage));
  return [
    { ...(parentState || {}), [fieldName]: childState },
    mapScopedCommand(childCommand, tag)
  ];
}

export function scopeSubscriptions(childState, subscriptions, tag) {
  if (typeof subscriptions !== "function") return { kind: "none" };
  return mapScopedSubscription(subscriptions(childState), tag);
}

export function scopeView(tag, view, childState) {
  let currentTag = tag;
  let currentView = view;
  let currentState = childState;
  let child = currentView(currentState);
  let parentDispatch = null;

  const renderChild = () => {
    child = currentView(currentState);
    return child;
  };

  const scopedDispatch = () => scopedMessageDispatch(parentDispatch, currentTag);

  return {
    mount(parent, dispatch, hydrateNode = null) {
      parentDispatch = dispatch;
      return child.mount(parent, scopedDispatch(), hydrateNode);
    },
    update(nextTag, nextView, nextState, dispatch, updateContext) {
      const previousState = currentState;
      parentDispatch = dispatch || parentDispatch;
      const viewChanged = nextView && nextView !== currentView;
      currentTag = nextTag;
      currentView = nextView || currentView;
      currentState = nextState;

      if (viewChanged) {
        const parent = child.root?.parentNode;
        const anchor = child.root?.nextSibling || null;
        if (child.root?.parentNode) child.root.parentNode.removeChild(child.root);
        child.dispose?.();
        renderChild();
        child.update?.(currentState, scopedDispatch(), forceUpdateContext(updateContext));
        if (parent && child.root?.parentNode !== parent) parent.insertBefore(child.root, anchor);
      } else {
        child.update?.(currentState, scopedDispatch(), scopedViewUpdateContext(updateContext, previousState, currentState));
      }
      return child.root;
    },
    dispose() {
      child?.dispose?.();
    },
    get root() {
      return child?.root;
    },
    definition: {
      name: "scope-view",
      params: ["tag", "view", "state"],
      scoped: true,
      get child() {
        return child?.definition;
      }
    }
  };
}

export function createCommandHandlers(env = {}) {
  const host = env.host || globalThis;
  const timers = env.timers || host;
  const bluetooth = env.bluetooth || host.navigator?.bluetooth;
  const storage = env.storage || host.localStorage;
  const fetchImpl = env.fetch || host.fetch?.bind(host);
  const random = env.random || Math.random;
  const now = env.now || (() => Date.now());
  const animation = env.animation || host;
  const clipboard = env.clipboard || host.navigator?.clipboard;
  const sessionStorage = env.sessionStorage || host.sessionStorage;
  const documentRef = env.document || host.document;
  const FormDataCtor = env.FormData || host.FormData || globalThis.FormData;
  const requestAnimationFrameImpl = env.requestAnimationFrame || animation.requestAnimationFrame?.bind(animation);
  const cancelAnimationFrameImpl = env.cancelAnimationFrame || animation.cancelAnimationFrame?.bind(animation);
  const matchMedia = env.matchMedia || host.matchMedia?.bind(host);
  const ResizeObserverCtor = env.ResizeObserver || host.ResizeObserver;
  const resizeTarget = env.resizeTarget || host;
  const eventTarget = env.eventTarget || env.window || host;
  const download = env.download || ((payload) => downloadWithBrowser(payload, env, host));
  const importFile = env.importFile || ((payload) => importWithBrowser(payload, env, host));
  const intervals = new Map();
  const timeouts = new Map();
  const animationFrames = new Map();
  const bluetoothConnections = new Map();
  const simulations = new Map();
  const mediaQueries = new Map();
  const resizeObservers = new Map();
  const windowEvents = new Map();

  const cleanupBluetoothConnection = (connection) => {
    if (!connection) return;
    const { device, characteristic, readingListener, disconnectListener } = connection;
    characteristic?.removeEventListener?.("characteristicvaluechanged", readingListener);
    try {
      const stopped = characteristic?.stopNotifications?.();
      stopped?.catch?.(() => {});
    } catch {
      // Browser implementations can throw if notifications are already stopped.
    }
    device?.removeEventListener?.("gattserverdisconnected", disconnectListener);
    device?.gatt?.disconnect?.();
  };

  const cleanupAll = () => {
    for (const interval of intervals.values()) timers.clearInterval?.(interval);
    intervals.clear();

    for (const timeout of timeouts.values()) timers.clearTimeout?.(timeout);
    timeouts.clear();

    for (const entry of animationFrames.values()) cancelAnimationFrameEntry(entry);
    animationFrames.clear();

    for (const connection of bluetoothConnections.values()) cleanupBluetoothConnection(connection);
    bluetoothConnections.clear();

    for (const simulation of simulations.values()) timers.clearInterval?.(simulation.interval);
    simulations.clear();

    for (const entry of mediaQueries.values()) removeMediaQueryListener(entry);
    mediaQueries.clear();

    for (const entry of resizeObservers.values()) removeResizeObserver(entry);
    resizeObservers.clear();

    for (const entry of windowEvents.values()) removeWindowEventListener(entry);
    windowEvents.clear();
  };

  const handlers = {
    "bluetooth/request-device": async function(command) {
      if (!bluetooth?.requestDevice) {
        return commandErrorMessage(command, new Error("Web Bluetooth unavailable."));
      }

      try {
        const options = bluetoothRequestOptions(command);
        const device = await bluetooth.requestDevice(options);
        return commandMessage(command, device);
      } catch (error) {
        return commandErrorMessage(command, error);
      }
    },
    "bluetooth/connect-heart-rate": async function(command, dispatch) {
      if (!bluetooth?.requestDevice) {
        return commandErrorMessage(command, new Error("Web Bluetooth unavailable."));
      }

      try {
        const id = command.id || "heart-rate";
        const options = bluetoothRequestOptions(command);
        const serviceName = command.service || "heart_rate";
        const characteristicName = command.characteristic || "heart_rate_measurement";
        const device = await bluetooth.requestDevice(options);
        const server = await device.gatt?.connect();
        if (!server) throw new Error("Bluetooth device did not expose a GATT server.");

        const service = await server.getPrimaryService(serviceName);
        const characteristic = await service.getCharacteristic(characteristicName);
        await characteristic.startNotifications();

        const readingListener = (event) => {
          const value = event.target?.value;
          if (!value) return;
          const message = namedCommandMessage(command.onReading, {
            bpm: parseHeartRateMeasurement(value)
          });
          if (message !== undefined) dispatch(message);
        };
        characteristic.addEventListener("characteristicvaluechanged", readingListener);

        const disconnectListener = () => {
          bluetoothConnections.delete(id);
          const message = namedCommandMessage(command.onDisconnected);
          if (message !== undefined) dispatch(message);
        };
        device.addEventListener?.("gattserverdisconnected", disconnectListener);

        bluetoothConnections.set(id, {
          device,
          characteristic,
          readingListener,
          disconnectListener
        });

        return commandMessage(command, {
          id,
          device,
          deviceName: device.name || "",
          connected: Boolean(device.gatt?.connected)
        });
      } catch (error) {
        return commandErrorMessage(command, error);
      }
    },
    "bluetooth/disconnect": async function(command) {
      const id = command.id || "heart-rate";
      const connection = bluetoothConnections.get(id);
      if (connection) {
        cleanupBluetoothConnection(connection);
        bluetoothConnections.delete(id);
      }
      return commandMessage(command);
    },
    "timer/after"(command) {
      return new Promise((resolve) => {
        const id = command.id ?? `timeout:${timeouts.size + 1}`;
        let fired = false;
        const timeout = timers.setTimeout(() => {
          fired = true;
          timeouts.delete(id);
          resolve(commandMessage(command));
        }, command.ms ?? 0);
        if (!fired) timeouts.set(id, timeout);
      });
    },
    "timer/every"(command, dispatch) {
      const id = command.id || `timer:${intervals.size + 1}`;
      const existing = intervals.get(id);
      if (existing !== undefined) timers.clearInterval(existing);
      const interval = timers.setInterval(() => {
        const message = commandMessage(command);
        if (message !== undefined) dispatch(message);
      }, command.ms ?? 0);
      intervals.set(id, interval);
    },
    "timer/cancel"(command) {
      const interval = intervals.get(command.id);
      if (interval !== undefined) {
        timers.clearInterval(interval);
        intervals.delete(command.id);
      }
      const timeout = timeouts.get(command.id);
      if (timeout !== undefined) {
        timers.clearTimeout?.(timeout);
        timeouts.delete(command.id);
      }
      return commandMessage(command);
    },
    "animation/frame"(command, dispatch) {
      const id = command.id || `frame:${animationFrames.size + 1}`;
      const existing = animationFrames.get(id);
      if (existing) cancelAnimationFrameEntry(existing);

      const callback = (timestamp) => {
        animationFrames.delete(id);
        const message = animationFrameMessage(command, id, timestamp);
        if (message !== undefined) dispatch(message);
      };

      let entry;
      if (requestAnimationFrameImpl) {
        entry = {
          kind: "animation",
          handle: requestAnimationFrameImpl(callback),
          cancel: cancelAnimationFrameImpl
        };
      } else if (timers.setTimeout) {
        entry = {
          kind: "timeout",
          handle: timers.setTimeout(() => callback(now()), 16),
          cancel: timers.clearTimeout?.bind(timers)
        };
      } else {
        return commandErrorMessage(command, new Error("requestAnimationFrame is unavailable."));
      }

      animationFrames.set(id, entry);
      return commandMessage(command, { id });
    },
    "animation/cancel"(command) {
      const existing = animationFrames.get(command.id);
      if (existing) {
        cancelAnimationFrameEntry(existing);
        animationFrames.delete(command.id);
      }
      return commandMessage(command, { id: command.id });
    },
    "time/now"(command) {
      return commandMessage(command, now());
    },
    "storage/get"(command) {
      try {
        const raw = storage?.getItem(command.key);
        return commandMessage(command, raw == null ? null : parseStoredValue(raw, command.format || command.parse));
      } catch (error) {
        return commandErrorMessage(command, error);
      }
    },
    "storage/set"(command) {
      try {
        storage?.setItem(command.key, serializeStoredValue(command.value));
        return commandMessage(command, command.value);
      } catch (error) {
        return commandErrorMessage(command, error);
      }
    },
    "storage/remove"(command) {
      try {
        storage?.removeItem(command.key);
        return commandMessage(command, { key: command.key });
      } catch (error) {
        return commandErrorMessage(command, error);
      }
    },
    "browser/history-replace-search-param"(command) {
      replaceBrowserSearchParam(host, command.name, command.value);
      return commandMessage(command);
    },
    "browser/history-write-route"(command) {
      writeBrowserRoute(host, command.url, command.op, command.definition);
      return commandMessage(command);
    },
    "browser/theme-load"(command) {
      const theme = loadBrowserTheme(host, storage, sessionStorage, command.key);
      return commandMessage(command, theme);
    },
    "browser/theme-apply"(command) {
      applyBrowserTheme(host, storage, sessionStorage, command.key, command.theme);
      return commandMessage(command, command.theme);
    },
    "browser/clipboard-write"(command) {
      void clipboard?.writeText?.(String(command.text ?? ""));
      return commandMessage(command);
    },
    "browser/set-cookie"(command) {
      setBrowserCookie(host, command.name, command.value);
      return commandMessage(command);
    },
    "auth-storage/persist"(command) {
      persistAuthStorage(storage, command.sourceUrl, command.entries);
      return commandMessage(command);
    },
    "auth-storage/load"(command) {
      return commandMessage(command, loadAuthStorage(storage, sessionStorage, command.sourceUrl));
    },
    "random/number"(command) {
      const min = command.min ?? 0;
      const max = command.max ?? 1;
      return commandMessage(command, min + random() * (max - min));
    },
    "simulation/heart-rate"(command, dispatch) {
      const id = command.id || "simulated-heart-rate";
      const existing = simulations.get(id);
      if (existing) timers.clearInterval?.(existing.interval);

      const min = numberOr(command.min, 90);
      const max = Math.max(min, numberOr(command.max, 160));
      const jitter = command.jitter == null ? null : Math.max(0, numberOr(command.jitter, 0));
      const ms = Math.max(1, numberOr(command.ms, 1000));
      let bpm = simulationHeartRateBpm(random, command.start, min, max, null);

      const emitReading = () => {
        bpm = simulationHeartRateBpm(random, undefined, min, max, jitter, bpm);
        const message = namedCommandMessage(command.onReading, { bpm });
        if (message !== undefined) dispatch(message);
      };

      const interval = timers.setInterval(emitReading, ms);
      simulations.set(id, { interval, command });

      return commandMessage(command, {
        id,
        deviceName: command.deviceName || "Simulated monitor",
        connected: true,
        simulated: true
      });
    },
    "simulation/stop"(command, dispatch) {
      const id = command.id || "simulated-heart-rate";
      const existing = simulations.get(id);
      if (existing) {
        timers.clearInterval?.(existing.interval);
        simulations.delete(id);
        const disconnected = namedCommandMessage(existing.command?.onDisconnected);
        if (disconnected !== undefined) dispatch(disconnected);
      }
      return commandMessage(command, { id });
    },
    "task/perform": async function(command) {
      try {
        const value = await runTask(command.task, { fetch: fetchImpl });
        return taskSuccessMessage(command, value);
      } catch (error) {
        return taskErrorMessage(command, error);
      }
    },
    "http/request": async function(command) {
      if (!fetchImpl) return commandErrorMessage(command, new Error("No fetch implementation is available for http/request"));

      try {
        const { url, options } = httpRequestFetchArgs(command, {
          document: documentRef,
          FormData: FormDataCtor
        });
        const fetchUrl = proxiedHttpUrl(url, command, host);
        const started = nowMs();
        const response = await fetchImpl(fetchUrl, options);
        const responseFormat = commandValueName(command.response || command.format || "json");
        const payload = await httpResponsePayload(response, responseFormat, url, env, host);
        return commandMessage(command, {
          status: response.status,
          statusText: response.statusText,
          ok: response.ok,
          ...payload,
          headers: headersToObject(response.headers),
          url: response.url || url,
          requestUrl: url,
          durationMs: Math.max(0, Math.round(nowMs() - started))
        });
      } catch (error) {
        return commandErrorMessage(command, error);
      }
    },
    "file/download"(command) {
      const payload = {
        name: command.name || "download",
        content: command.content ?? "",
        mime: command.mime || "application/octet-stream",
        blob: command.blob
      };
      const result = download(payload);
      return commandMessage(command, result ?? payload);
    },
    "file/import": async function(command) {
      const payload = {
        accept: command.accept || "",
        multiple: Boolean(command.multiple),
        format: commandValueName(command.format || command.parse || "text")
      };

      try {
        const value = await importFile(payload);
        if (value === undefined) return commandCancelMessage(command);
        return commandMessage(command, value);
      } catch (error) {
        return commandErrorMessage(command, error);
      }
    },
    "file/read-selected": async function(command, dispatch) {
      const input = resolveRef(command.ref, dispatch);
      if (!input) return commandErrorMessage(command, new Error(`File input ref ${String(command.ref)} was not found.`));

      const format = commandValueName(command.format || command.parse || "text");
      const shouldClear = command.clear !== false;
      try {
        const files = Array.from(input.files || []);
        if (!files.length) {
          if (shouldClear) clearFileInput(input);
          return commandCancelMessage(command);
        }

        const imported = command.multiple
          ? await Promise.all(files.map((file) => readImportedFile(file, format)))
          : await readImportedFile(files[0], format);
        if (shouldClear) clearFileInput(input);
        return commandMessage(command, imported);
      } catch (error) {
        if (shouldClear) clearFileInput(input);
        return commandErrorMessage(command, error);
      }
    },
    "canvas/draw"(command, dispatch) {
      const canvas = resolveRef(command.ref, dispatch);
      if (!canvas) return commandErrorMessage(command, new Error(`Canvas ref ${String(command.ref)} was not found.`));
      const ctx = canvas.getContext?.("2d");
      if (!ctx) return commandErrorMessage(command, new Error("Canvas 2D context is unavailable."));

      const sizing = canvasDrawSizing(command, canvas, env, host);
      if (sizing.width !== undefined) canvas.width = sizing.width;
      if (sizing.height !== undefined) canvas.height = sizing.height;
      if (sizing.cssWidth !== undefined && command.setCssSize === true) setCanvasCssSize(canvas, "width", sizing.cssWidth);
      if (sizing.cssHeight !== undefined && command.setCssSize === true) setCanvasCssSize(canvas, "height", sizing.cssHeight);
      setCanvasTransform(ctx, sizing.pixelRatio);

      for (const op of command.ops || []) applyCanvasOp(ctx, canvas, op);
      return commandMessage(command, {
        ref: refName(command.ref),
        width: canvas.width,
        height: canvas.height,
        cssWidth: sizing.cssWidth ?? canvas.width,
        cssHeight: sizing.cssHeight ?? canvas.height,
        pixelRatio: sizing.pixelRatio
      });
    },
    "canvas/measure-text"(command, dispatch) {
      const canvas = resolveRef(command.ref, dispatch);
      if (!canvas) return commandErrorMessage(command, new Error(`Canvas ref ${String(command.ref)} was not found.`));
      const ctx = canvas.getContext?.("2d");
      if (!ctx?.measureText) return commandErrorMessage(command, new Error("Canvas text measurement is unavailable."));

      applyCanvasState(ctx, command, "fill");
      const texts = canvasMeasureTexts(command);
      const measurements = texts.map((text) => {
        const metrics = ctx.measureText(text);
        return {
          text,
          width: numberOrZero(metrics?.width),
          actualBoundingBoxLeft: numberOrZero(metrics?.actualBoundingBoxLeft),
          actualBoundingBoxRight: numberOrZero(metrics?.actualBoundingBoxRight),
          actualBoundingBoxAscent: numberOrZero(metrics?.actualBoundingBoxAscent),
          actualBoundingBoxDescent: numberOrZero(metrics?.actualBoundingBoxDescent)
        };
      });
      return commandMessage(command, {
        ref: refName(command.ref),
        font: ctx.font || "",
        texts,
        widths: measurements.map((item) => item.width),
        measurements
      });
    },
    "dom-ref/focus"(command, dispatch) {
      const node = resolveRef(command.ref, dispatch);
      if (!node?.focus) return commandErrorMessage(command, new Error(`DOM ref ${String(command.ref)} cannot be focused.`));
      node.focus();
      return commandMessage(command, { ref: refName(command.ref) });
    },
    "dom-ref/click"(command, dispatch) {
      const node = resolveRef(command.ref, dispatch);
      if (!node?.click) return commandErrorMessage(command, new Error(`DOM ref ${String(command.ref)} cannot be clicked.`));
      node.click();
      return commandMessage(command, { ref: refName(command.ref) });
    },
    "dom-ref/measure"(command, dispatch) {
      const node = resolveRef(command.ref, dispatch);
      if (!node) return commandErrorMessage(command, new Error(`DOM ref ${String(command.ref)} was not found.`));
      const rect = measureNode(node);
      return commandMessage(command, { ref: refName(command.ref), ...rect });
    },
    "dom/scroll-into-view"(command) {
      queueScrollIntoView(command, {
        document: documentRef,
        host,
        requestAnimationFrame: requestAnimationFrameImpl
      });
      return commandMessage(command, {
        id: command.id || "",
        ref: command.ref || "",
        selector: command.selector || "",
        testId: command.testId || ""
      });
    },
    "dom-ref/resize-watch"(command, dispatch) {
      const node = resolveRef(command.ref, dispatch);
      if (!node) return commandErrorMessage(command, new Error(`DOM ref ${String(command.ref)} was not found.`));
      const id = command.id || refName(command.ref);
      const existing = resizeObservers.get(id);
      if (existing) removeResizeObserver(existing);

      if (ResizeObserverCtor) {
        const observer = new ResizeObserverCtor((entries) => {
          const entry = entries?.find((item) => item.target === node) || entries?.[0];
          const message = resizeMessage(command, id, node, rectFromResizeEntry(entry, node));
          if (message !== undefined) dispatch(message);
        });
        observer.observe(node);
        resizeObservers.set(id, { observer, node });
      } else if (resizeTarget?.addEventListener) {
        const listener = () => {
          const message = resizeMessage(command, id, node, measureNode(node));
          if (message !== undefined) dispatch(message);
        };
        resizeTarget.addEventListener("resize", listener);
        resizeObservers.set(id, { target: resizeTarget, listener });
      } else {
        return commandErrorMessage(command, new Error("ResizeObserver or resize events are unavailable."));
      }

      return resizeMessage(command, id, node, measureNode(node));
    },
    "dom-ref/resize-unwatch"(command) {
      const id = command.id || command.ref;
      const existing = resizeObservers.get(id);
      if (existing) {
        removeResizeObserver(existing);
        resizeObservers.delete(id);
      }
      return commandMessage(command, { id });
    },
    "window/event-watch"(command, dispatch) {
      if (!eventTarget?.addEventListener) return commandErrorMessage(command, new Error("Window event target is unavailable."));
      const type = String(command.type || command.event || "");
      if (!type) return commandErrorMessage(command, new Error("window/event-watch requires an event type."));
      const id = command.id || type;
      const existing = windowEvents.get(id);
      if (existing) removeWindowEventListener(existing);

      const options = eventListenerOptions(command.options);
      const listener = (event) => {
        applyWindowEventControls(command, event);
        const message = windowEventMessage(command, event, id, host);
        if (message !== undefined) dispatch(message);
      };
      eventTarget.addEventListener(type, listener, options);
      windowEvents.set(id, { target: eventTarget, type, listener, options });
      return commandMessage(command, { id, type });
    },
    "window/event-unwatch"(command) {
      const id = command.id || command.type || command.event;
      const existing = windowEvents.get(id);
      if (existing) {
        removeWindowEventListener(existing);
        windowEvents.delete(id);
      }
      return commandMessage(command, { id });
    },
    "media-query/watch"(command, dispatch) {
      if (!matchMedia) return commandErrorMessage(command, new Error("matchMedia is unavailable."));
      const query = String(command.query || "");
      const id = command.id || query;
      const mediaQuery = matchMedia(query);
      const existing = mediaQueries.get(id);
      if (existing) removeMediaQueryListener(existing);

      const listener = (event) => {
        const message = mediaQueryMessage(command, event, id);
        if (message !== undefined) dispatch(message);
      };
      addMediaQueryListener(mediaQuery, listener);
      mediaQueries.set(id, { mediaQuery, listener });
      return mediaQueryMessage(command, mediaQuery, id);
    },
    "media-query/unwatch"(command) {
      const id = command.id || command.query;
      const existing = mediaQueries.get(id);
      if (existing) {
        removeMediaQueryListener(existing);
        mediaQueries.delete(id);
      }
      return commandMessage(command, { id });
    }
  };

  Object.defineProperty(handlers, "dispose", {
    value: cleanupAll
  });

  return handlers;
}

export function createSelectedCommandHandlers(env = {}, registrations = []) {
  const context = {
    env,
    host: env.host || globalThis,
    disposers: []
  };
  const handlers = {};
  for (const register of registrations) {
    register(handlers, context);
  }
  Object.defineProperty(handlers, "dispose", {
    value() {
      for (const dispose of context.disposers.splice(0)) dispose();
    }
  });
  return handlers;
}

export function createCompiledCommandHandlers(registrations = []) {
  const context = {
    env: {},
    host: globalThis,
    disposers: []
  };
  const handlers = {};
  for (const register of registrations) {
    register(handlers, context);
  }
  Object.defineProperty(handlers, "dispose", {
    value() {
      for (const dispose of context.disposers.splice(0)) dispose();
    }
  });
  return handlers;
}

function addCommandDisposer(context, dispose) {
  context.disposers.push(dispose);
}

export function registerBluetoothCommandHandlers(handlers, context) {
  const { env, host } = context;
  const bluetooth = env.bluetooth || host.navigator?.bluetooth;
  const bluetoothConnections = new Map();

  const cleanupBluetoothConnection = (connection) => {
    if (!connection) return;
    const { device, characteristic, readingListener, disconnectListener } = connection;
    characteristic?.removeEventListener?.("characteristicvaluechanged", readingListener);
    try {
      const stopped = characteristic?.stopNotifications?.();
      stopped?.catch?.(() => {});
    } catch {
      // Browser implementations can throw if notifications are already stopped.
    }
    device?.removeEventListener?.("gattserverdisconnected", disconnectListener);
    device?.gatt?.disconnect?.();
  };

  addCommandDisposer(context, () => {
    for (const connection of bluetoothConnections.values()) cleanupBluetoothConnection(connection);
    bluetoothConnections.clear();
  });

  handlers["bluetooth/request-device"] = async function(command) {
    if (!bluetooth?.requestDevice) {
      return commandErrorMessage(command, new Error("Bluetooth unavailable"));
    }

    try {
      const options = bluetoothRequestOptions(command);
      const device = await bluetooth.requestDevice(options);
      return commandMessage(command, device);
    } catch (error) {
      return commandErrorMessage(command, error);
    }
  };

  handlers["bluetooth/connect-heart-rate"] = async function(command, dispatch) {
    if (!bluetooth?.requestDevice) {
      return commandErrorMessage(command, new Error("Bluetooth unavailable"));
    }

    try {
      const id = command.id || "heart-rate";
      const options = bluetoothRequestOptions(command);
      const serviceName = command.service || "heart_rate";
      const characteristicName = command.characteristic || "heart_rate_measurement";
      const device = await bluetooth.requestDevice(options);
      const server = await device.gatt?.connect();
      if (!server) throw new Error("Bluetooth GATT unavailable");

      const service = await server.getPrimaryService(serviceName);
      const characteristic = await service.getCharacteristic(characteristicName);
      await characteristic.startNotifications();

      const readingListener = (event) => {
        const value = event.target?.value;
        if (!value) return;
        const message = namedCommandMessage(command.onReading, {
          bpm: parseHeartRateMeasurement(value)
        });
        if (message !== undefined) dispatch(message);
      };
      characteristic.addEventListener("characteristicvaluechanged", readingListener);

      const disconnectListener = () => {
        bluetoothConnections.delete(id);
        const message = namedCommandMessage(command.onDisconnected);
        if (message !== undefined) dispatch(message);
      };
      device.addEventListener?.("gattserverdisconnected", disconnectListener);

      bluetoothConnections.set(id, {
        device,
        characteristic,
        readingListener,
        disconnectListener
      });

      return commandMessage(command, {
        id,
        device,
        deviceName: device.name || "",
        connected: Boolean(device.gatt?.connected)
      });
    } catch (error) {
      return commandErrorMessage(command, error);
    }
  };

  handlers["bluetooth/disconnect"] = async function(command) {
    const id = command.id || "heart-rate";
    const connection = bluetoothConnections.get(id);
    if (connection) {
      cleanupBluetoothConnection(connection);
      bluetoothConnections.delete(id);
    }
    return commandMessage(command);
  };
}

export function registerCompiledBluetoothHeartRateCommandHandlers(handlers, context) {
  const bluetooth = globalThis.navigator?.bluetooth;
  const bluetoothConnections = new Map();

  const cleanupBluetoothConnection = (connection) => {
    if (!connection) return;
    const { device, characteristic, readingListener, disconnectListener } = connection;
    characteristic?.removeEventListener?.("characteristicvaluechanged", readingListener);
    try {
      characteristic?.stopNotifications?.()?.catch?.(() => {});
    } catch {}
    device?.removeEventListener?.("gattserverdisconnected", disconnectListener);
    device?.gatt?.disconnect?.();
  };

  addCommandDisposer(context, () => {
    for (const connection of bluetoothConnections.values()) cleanupBluetoothConnection(connection);
    bluetoothConnections.clear();
  });

  handlers["bluetooth/connect-heart-rate"] = async function(command, dispatch) {
    if (!bluetooth?.requestDevice) return compiledCommandErrorMessage(command, new Error("Bluetooth unavailable"));
    try {
      const id = command.id || "heart-rate";
      const device = await bluetooth.requestDevice(compiledBluetoothRequestOptions(command));
      const server = await device.gatt?.connect();
      if (!server) throw new Error("Bluetooth GATT unavailable");

      const service = await server.getPrimaryService(command.service || "heart_rate");
      const characteristic = await service.getCharacteristic(command.characteristic || "heart_rate_measurement");
      await characteristic.startNotifications();

      const readingListener = (event) => {
        const value = event.target?.value;
        if (!value) return;
        const message = compiledNamedCommandMessage(command.onReading, {
          bpm: parseHeartRateMeasurement(value)
        });
        if (message !== undefined) dispatch(message);
      };
      characteristic.addEventListener("characteristicvaluechanged", readingListener);

      const disconnectListener = () => {
        bluetoothConnections.delete(id);
        const message = compiledNamedCommandMessage(command.onDisconnected);
        if (message !== undefined) dispatch(message);
      };
      device.addEventListener?.("gattserverdisconnected", disconnectListener);
      bluetoothConnections.set(id, { device, characteristic, readingListener, disconnectListener });

      return compiledCommandMessage(command, {
        id,
        device,
        deviceName: device.name || "",
        connected: Boolean(device.gatt?.connected)
      });
    } catch (error) {
      return compiledCommandErrorMessage(command, error);
    }
  };

  handlers["bluetooth/disconnect"] = async function(command) {
    const id = command.id || "heart-rate";
    const connection = bluetoothConnections.get(id);
    if (connection) {
      cleanupBluetoothConnection(connection);
      bluetoothConnections.delete(id);
    }
    return compiledCommandMessage(command);
  };
}

export function registerTimerCommandHandlers(handlers, context) {
  const timers = context.env.timers || context.host;
  const intervals = new Map();
  const timeouts = new Map();

  addCommandDisposer(context, () => {
    for (const interval of intervals.values()) timers.clearInterval?.(interval);
    intervals.clear();
    for (const timeout of timeouts.values()) timers.clearTimeout?.(timeout);
    timeouts.clear();
  });

  handlers["timer/after"] = function(command) {
    return new Promise((resolve) => {
      const id = command.id ?? `timeout:${timeouts.size + 1}`;
      let fired = false;
      const timeout = timers.setTimeout(() => {
        fired = true;
        timeouts.delete(id);
        resolve(commandMessage(command));
      }, command.ms ?? 0);
      if (!fired) timeouts.set(id, timeout);
    });
  };

  handlers["timer/every"] = function(command, dispatch) {
    const id = command.id || `timer:${intervals.size + 1}`;
    const existing = intervals.get(id);
    if (existing !== undefined) timers.clearInterval(existing);
    const interval = timers.setInterval(() => {
      const message = commandMessage(command);
      if (message !== undefined) dispatch(message);
    }, command.ms ?? 0);
    intervals.set(id, interval);
  };

  handlers["timer/cancel"] = function(command) {
    const interval = intervals.get(command.id);
    if (interval !== undefined) {
      timers.clearInterval(interval);
      intervals.delete(command.id);
    }
    const timeout = timeouts.get(command.id);
    if (timeout !== undefined) {
      timers.clearTimeout?.(timeout);
      timeouts.delete(command.id);
    }
    return commandMessage(command);
  };
}

export function registerCompiledTimerCommandHandlers(handlers, context) {
  const timers = globalThis;
  const intervals = new Map();
  const timeouts = new Map();

  addCommandDisposer(context, () => {
    for (const interval of intervals.values()) timers.clearInterval(interval);
    intervals.clear();
    for (const timeout of timeouts.values()) timers.clearTimeout?.(timeout);
    timeouts.clear();
  });

  handlers["timer/after"] = function(command) {
    return new Promise((resolve) => {
      const id = command.id;
      const timeout = timers.setTimeout(() => {
        timeouts.delete(id);
        resolve(compiledCommandMessage(command));
      }, command.ms ?? 0);
      timeouts.set(id, timeout);
    });
  };

  handlers["timer/every"] = function(command, dispatch) {
    const id = command.id;
    const existing = intervals.get(id);
    if (existing !== undefined) timers.clearInterval(existing);
    intervals.set(id, timers.setInterval(() => {
      const message = compiledCommandMessage(command);
      if (message !== undefined) dispatch(message);
    }, command.ms ?? 0));
  };

  handlers["timer/cancel"] = function(command) {
    const interval = intervals.get(command.id);
    if (interval !== undefined) {
      timers.clearInterval(interval);
      intervals.delete(command.id);
    }
    const timeout = timeouts.get(command.id);
    if (timeout !== undefined) {
      timers.clearTimeout?.(timeout);
      timeouts.delete(command.id);
    }
    return compiledCommandMessage(command);
  };
}

export function registerAnimationCommandHandlers(handlers, context) {
  const { env, host } = context;
  const timers = env.timers || host;
  const now = env.now || (() => Date.now());
  const animation = env.animation || host;
  const requestAnimationFrameImpl = env.requestAnimationFrame || animation.requestAnimationFrame?.bind(animation);
  const cancelAnimationFrameImpl = env.cancelAnimationFrame || animation.cancelAnimationFrame?.bind(animation);
  const animationFrames = new Map();

  addCommandDisposer(context, () => {
    for (const entry of animationFrames.values()) cancelAnimationFrameEntry(entry);
    animationFrames.clear();
  });

  handlers["animation/frame"] = function(command, dispatch) {
    const id = command.id;
    const existing = animationFrames.get(id);
    if (existing) cancelAnimationFrameEntry(existing);

    const callback = (timestamp) => {
      animationFrames.delete(id);
      const message = animationFrameMessage(command, id, timestamp);
      if (message !== undefined) dispatch(message);
    };

    let entry;
    if (requestAnimationFrameImpl) {
      entry = {
        kind: "animation",
        handle: requestAnimationFrameImpl(callback),
        cancel: cancelAnimationFrameImpl
      };
    } else if (timers.setTimeout) {
      entry = {
        kind: "timeout",
        handle: timers.setTimeout(() => callback(now()), 16),
        cancel: timers.clearTimeout?.bind(timers)
      };
    } else {
      return commandErrorMessage(command, new Error("RAF unavailable"));
    }

    animationFrames.set(id, entry);
    return commandMessage(command, { id });
  };

  handlers["animation/cancel"] = function(command) {
    const existing = animationFrames.get(command.id);
    if (existing) {
      cancelAnimationFrameEntry(existing);
      animationFrames.delete(command.id);
    }
    return commandMessage(command, { id: command.id });
  };
}

export function registerCompiledAnimationCommandHandlers(handlers, context) {
  const timers = globalThis;
  const requestAnimationFrameImpl = globalThis.requestAnimationFrame?.bind(globalThis);
  const cancelAnimationFrameImpl = globalThis.cancelAnimationFrame?.bind(globalThis);
  const animationFrames = new Map();

  addCommandDisposer(context, () => {
    for (const entry of animationFrames.values()) cancelAnimationFrameEntry(entry);
    animationFrames.clear();
  });

  handlers["animation/frame"] = function(command, dispatch) {
    const id = command.id || `frame:${animationFrames.size + 1}`;
    const existing = animationFrames.get(id);
    if (existing) cancelAnimationFrameEntry(existing);

    const callback = (timestamp) => {
      animationFrames.delete(id);
      const message = compiledAnimationFrameMessage(command, id, timestamp);
      if (message !== undefined) dispatch(message);
    };

    if (requestAnimationFrameImpl) {
      animationFrames.set(id, {
        kind: "animation",
        handle: requestAnimationFrameImpl(callback),
        cancel: cancelAnimationFrameImpl
      });
    } else if (timers.setTimeout) {
      animationFrames.set(id, {
        kind: "timeout",
        handle: timers.setTimeout(() => callback(Date.now()), 16),
        cancel: timers.clearTimeout?.bind(timers)
      });
    } else {
      return compiledCommandErrorMessage(command, new Error("RAF unavailable"));
    }

    return compiledCommandMessage(command, { id });
  };

  handlers["animation/cancel"] = function(command) {
    const existing = animationFrames.get(command.id);
    if (existing) {
      cancelAnimationFrameEntry(existing);
      animationFrames.delete(command.id);
    }
    return compiledCommandMessage(command, { id: command.id });
  };
}

export function registerTimeCommandHandlers(handlers, context) {
  const now = context.env.now || (() => Date.now());
  handlers["time/now"] = function(command) {
    return commandMessage(command, now());
  };
}

export function registerCompiledTimeCommandHandlers(handlers) {
  handlers["time/now"] = function(command) {
    return compiledCommandMessage(command, Date.now());
  };
}

export function registerStorageCommandHandlers(handlers, context) {
  const storage = context.env.storage || context.host.localStorage;

  handlers["storage/get"] = function(command) {
    try {
      const raw = storage?.getItem(command.key);
      return commandMessage(command, raw == null ? null : parseStoredValue(raw, command.format || command.parse));
    } catch (error) {
      return commandErrorMessage(command, error);
    }
  };

  handlers["storage/set"] = function(command) {
    try {
      storage?.setItem(command.key, serializeStoredValue(command.value));
      return commandMessage(command, command.value);
    } catch (error) {
      return commandErrorMessage(command, error);
    }
  };

  handlers["storage/remove"] = function(command) {
    try {
      storage?.removeItem(command.key);
      return commandMessage(command, { key: command.key });
    } catch (error) {
      return commandErrorMessage(command, error);
    }
  };
}

export function registerCompiledStorageCommandHandlers(handlers) {
  registerCompiledStorageReadWriteCommandHandlers(handlers);
  registerCompiledStorageRemoveCommandHandlers(handlers);
}

export function registerCompiledStorageReadWriteCommandHandlers(handlers) {
  const storage = globalThis.localStorage;

  handlers["storage/get"] = function(command) {
    try {
      const raw = storage?.getItem(command.key);
      return compiledCommandMessage(command, raw == null ? null : parseCompiledStoredValue(raw, command.format));
    } catch (error) {
      return compiledCommandErrorMessage(command, error);
    }
  };

  handlers["storage/set"] = function(command) {
    try {
      storage?.setItem(command.key, serializeStoredValue(command.value));
      return compiledCommandMessage(command, command.value);
    } catch (error) {
      return compiledCommandErrorMessage(command, error);
    }
  };
}

export function registerCompiledStorageRemoveCommandHandlers(handlers) {
  const storage = globalThis.localStorage;

  handlers["storage/remove"] = function(command) {
    try {
      storage?.removeItem(command.key);
      return compiledCommandMessage(command, { key: command.key });
    } catch (error) {
      return compiledCommandErrorMessage(command, error);
    }
  };
}

export function registerBrowserCommandHandlers(handlers, context) {
  const { env, host } = context;
  const storage = env.storage || host.localStorage;
  const sessionStorage = env.sessionStorage || host.sessionStorage;
  const clipboard = env.clipboard || host.navigator?.clipboard;

  handlers["browser/history-replace-search-param"] = function(command) {
    replaceBrowserSearchParam(host, command.name, command.value);
    return commandMessage(command);
  };

  handlers["browser/history-write-route"] = function(command) {
    writeBrowserRoute(host, command.url, command.op, command.definition);
    return commandMessage(command);
  };

  handlers["browser/theme-load"] = function(command) {
    const theme = loadBrowserTheme(host, storage, sessionStorage, command.key);
    return commandMessage(command, theme);
  };

  handlers["browser/theme-apply"] = function(command) {
    applyBrowserTheme(host, storage, sessionStorage, command.key, command.theme);
    return commandMessage(command, command.theme);
  };

  handlers["browser/clipboard-write"] = function(command) {
    void clipboard?.writeText?.(String(command.text ?? ""));
    return commandMessage(command);
  };

  handlers["browser/set-cookie"] = function(command) {
    setBrowserCookie(host, command.name, command.value);
    return commandMessage(command);
  };
}

export function registerAuthStorageCommandHandlers(handlers, context) {
  const storage = context.env.storage || context.host.localStorage;
  const sessionStorage = context.env.sessionStorage || context.host.sessionStorage;

  handlers["auth-storage/persist"] = function(command) {
    persistAuthStorage(storage, command.sourceUrl, command.entries);
    return commandMessage(command);
  };

  handlers["auth-storage/load"] = function(command) {
    return commandMessage(command, loadAuthStorage(storage, sessionStorage, command.sourceUrl));
  };
}

export function registerRandomCommandHandlers(handlers, context) {
  const random = context.env.random || Math.random;
  handlers["random/number"] = function(command) {
    const min = command.min ?? 0;
    const max = command.max ?? 1;
    return commandMessage(command, min + random() * (max - min));
  };
}

export function registerCompiledRandomCommandHandlers(handlers) {
  handlers["random/number"] = function(command) {
    const min = command.min ?? 0;
    const max = command.max ?? 1;
    return compiledCommandMessage(command, min + Math.random() * (max - min));
  };
}

export function registerSimulationCommandHandlers(handlers, context) {
  const { env, host } = context;
  const timers = env.timers || host;
  const random = env.random || Math.random;
  const simulations = new Map();

  addCommandDisposer(context, () => {
    for (const simulation of simulations.values()) timers.clearInterval?.(simulation.interval);
    simulations.clear();
  });

  handlers["simulation/heart-rate"] = function(command, dispatch) {
    const id = command.id || "simulated-heart-rate";
    const existing = simulations.get(id);
    if (existing) timers.clearInterval?.(existing.interval);

    const min = numberOr(command.min, 90);
    const max = Math.max(min, numberOr(command.max, 160));
    const jitter = command.jitter == null ? null : Math.max(0, numberOr(command.jitter, 0));
    const ms = Math.max(1, numberOr(command.ms, 1000));
    let bpm = simulationHeartRateBpm(random, command.start, min, max, null);

    const emitReading = () => {
      bpm = simulationHeartRateBpm(random, undefined, min, max, jitter, bpm);
      const message = namedCommandMessage(command.onReading, { bpm });
      if (message !== undefined) dispatch(message);
    };

    const interval = timers.setInterval(emitReading, ms);
    simulations.set(id, { interval, command });

    return commandMessage(command, {
      id,
      deviceName: command.deviceName || "Simulated monitor",
      connected: true,
      simulated: true
    });
  };

  handlers["simulation/stop"] = function(command, dispatch) {
    const id = command.id || "simulated-heart-rate";
    const existing = simulations.get(id);
    if (existing) {
      timers.clearInterval?.(existing.interval);
      simulations.delete(id);
      const disconnected = namedCommandMessage(existing.command?.onDisconnected);
      if (disconnected !== undefined) dispatch(disconnected);
    }
    return commandMessage(command, { id });
  };
}

export function registerCompiledSimulationCommandHandlers(handlers, context) {
  const timers = globalThis;
  const simulations = new Map();

  addCommandDisposer(context, () => {
    for (const simulation of simulations.values()) timers.clearInterval?.(simulation.interval);
    simulations.clear();
  });

  handlers["simulation/heart-rate"] = function(command, dispatch) {
    const id = command.id || "simulated-heart-rate";
    const existing = simulations.get(id);
    if (existing) timers.clearInterval?.(existing.interval);

    const min = numberOr(command.min, 90);
    const max = Math.max(min, numberOr(command.max, 160));
    const jitter = command.jitter == null ? null : Math.max(0, numberOr(command.jitter, 0));
    const ms = Math.max(1, numberOr(command.ms, 1000));
    let bpm = simulationHeartRateBpm(Math.random, command.start, min, max, null);

    const interval = timers.setInterval(() => {
      bpm = simulationHeartRateBpm(Math.random, undefined, min, max, jitter, bpm);
      const message = compiledNamedCommandMessage(command.onReading, { bpm });
      if (message !== undefined) dispatch(message);
    }, ms);
    simulations.set(id, { interval, command });

    return compiledCommandMessage(command, {
      id,
      deviceName: command.deviceName || "Simulated monitor",
      connected: true,
      simulated: true
    });
  };

  handlers["simulation/stop"] = function(command, dispatch) {
    const id = command.id || "simulated-heart-rate";
    const existing = simulations.get(id);
    if (existing) {
      timers.clearInterval?.(existing.interval);
      simulations.delete(id);
      const disconnected = compiledNamedCommandMessage(existing.command?.onDisconnected);
      if (disconnected !== undefined) dispatch(disconnected);
    }
    return compiledCommandMessage(command, { id });
  };
}

export function registerTaskCommandHandlers(handlers, context) {
  const fetchImpl = context.env.fetch || context.host.fetch?.bind(context.host);
  handlers["task/perform"] = async function(command) {
    try {
      const value = await runTask(command.task, { fetch: fetchImpl });
      return taskSuccessMessage(command, value);
    } catch (error) {
      return taskErrorMessage(command, error);
    }
  };
}

export function registerHttpCommandHandlers(handlers, context) {
  const { env, host } = context;
  const fetchImpl = env.fetch || host.fetch?.bind(host);
  const documentRef = env.document || host.document;
  const FormDataCtor = env.FormData || host.FormData || globalThis.FormData;

  handlers["http/request"] = async function(command) {
    if (!fetchImpl) return commandErrorMessage(command, new Error("No fetch implementation is available for http/request"));

    try {
      const { url, options } = httpRequestFetchArgs(command, {
        document: documentRef,
        FormData: FormDataCtor
      });
      const fetchUrl = proxiedHttpUrl(url, command, host);
      const started = nowMs();
      const response = await fetchImpl(fetchUrl, options);
      const responseFormat = commandValueName(command.response || command.format || "json");
      const payload = await httpResponsePayload(response, responseFormat, url, env, host);
      return commandMessage(command, {
        status: response.status,
        statusText: response.statusText,
        ok: response.ok,
        ...payload,
        headers: headersToObject(response.headers),
        url: response.url || url,
        requestUrl: url,
        durationMs: Math.max(0, Math.round(nowMs() - started))
      });
    } catch (error) {
      return commandErrorMessage(command, error);
    }
  };
}

export function registerCompiledSimulationStopCommandHandlers(handlers) {
  handlers["simulation/stop"] = function(command) {
    return compiledCommandMessage(command, { id: command.id || "simulated-heart-rate" });
  };
}

export function registerFileDownloadCommandHandlers(handlers, context) {
  const { env, host } = context;
  const download = env.download || ((payload) => downloadWithBrowser(payload, env, host));

  handlers["file/download"] = function(command) {
    const payload = {
      name: command.name || "download",
      content: command.content ?? "",
      mime: command.mime || "application/octet-stream",
      blob: command.blob
    };
    const result = download(payload);
    return commandMessage(command, result ?? payload);
  };
}

export function registerCompiledFileDownloadCommandHandlers(handlers) {
  handlers["file/download"] = function(command) {
    const payload = {
      name: command.name || "download",
      content: command.content ?? "",
      mime: command.mime || "application/octet-stream",
      blob: command.blob
    };
    const documentRef = globalThis.document;
    const URLRef = globalThis.URL;
    const BlobCtor = globalThis.Blob;
    if (!documentRef || !URLRef?.createObjectURL || !BlobCtor) {
      throw new Error("No browser download implementation is available for file/download");
    }

    const blob = payload.blob || new BlobCtor([payload.content], { type: payload.mime });
    const href = URLRef.createObjectURL(blob);
    const link = documentRef.createElement("a");
    link.href = href;
    link.download = payload.name;
    link.style ||= {};
    link.style.display = "none";
    documentRef.body?.appendChild?.(link);
    link.click();
    link.parentNode?.removeChild?.(link);
    URLRef.revokeObjectURL?.(href);
    return compiledCommandMessage(command, { ...payload, href, size: blob.size });
  };
}

export function registerFileImportCommandHandlers(handlers, context) {
  const { env, host } = context;
  const importFile = env.importFile || ((payload) => importWithBrowser(payload, env, host));

  handlers["file/import"] = async function(command) {
    const payload = {
      accept: command.accept || "",
      multiple: Boolean(command.multiple),
      format: commandValueName(command.format || command.parse || "text")
    };

    try {
      const value = await importFile(payload);
      if (value === undefined) return commandCancelMessage(command);
      return commandMessage(command, value);
    } catch (error) {
      return commandErrorMessage(command, error);
    }
  };
}

export function registerFileReadSelectedCommandHandlers(handlers) {
  handlers["file/read-selected"] = async function(command, dispatch) {
    const input = resolveRef(command.ref, dispatch);
    if (!input) return commandErrorMessage(command, new Error(`Missing file ref ${String(command.ref)}`));

    const format = commandValueName(command.format || command.parse || "text");
    const shouldClear = command.clear !== false;
    try {
      const files = Array.from(input.files || []);
      if (!files.length) {
        if (shouldClear) clearFileInput(input);
        return commandCancelMessage(command);
      }

      const imported = command.multiple
        ? await Promise.all(files.map((file) => readImportedFile(file, format)))
        : await readImportedFile(files[0], format);
      if (shouldClear) clearFileInput(input);
      return commandMessage(command, imported);
    } catch (error) {
      if (shouldClear) clearFileInput(input);
      return commandErrorMessage(command, error);
    }
  };
}

export function registerCompiledFileReadSelectedCommandHandlers(handlers) {
  handlers["file/read-selected"] = async function(command, dispatch) {
    const input = resolveCompiledRef(command.ref, dispatch);
    if (!input) return compiledCommandErrorMessage(command, new Error(`Missing file ref ${String(command.ref)}`));

    const format = command.format;
    const shouldClear = command.clear !== false;
    try {
      const files = Array.from(input.files || []);
      if (!files.length) {
        if (shouldClear) clearFileInput(input);
        return compiledCommandCancelMessage(command);
      }

      const imported = command.multiple
        ? await Promise.all(files.map((file) => readImportedFile(file, format)))
        : await readImportedFile(files[0], format);
      if (shouldClear) clearFileInput(input);
      return compiledCommandMessage(command, imported);
    } catch (error) {
      if (shouldClear) clearFileInput(input);
      return compiledCommandErrorMessage(command, error);
    }
  };
}

export function registerCanvasDrawCommandHandlers(handlers, context) {
  const { env, host } = context;
  handlers["canvas/draw"] = function(command, dispatch) {
    const canvas = resolveRef(command.ref, dispatch);
    if (!canvas) return commandErrorMessage(command, new Error(`Missing canvas ref ${String(command.ref)}`));
    const ctx = canvas.getContext?.("2d");
    if (!ctx) return commandErrorMessage(command, new Error("Canvas 2D unavailable"));

    const sizing = canvasDrawSizing(command, canvas, env, host);
    if (sizing.width !== undefined) canvas.width = sizing.width;
    if (sizing.height !== undefined) canvas.height = sizing.height;
    if (sizing.cssWidth !== undefined && command.setCssSize === true) setCanvasCssSize(canvas, "width", sizing.cssWidth);
    if (sizing.cssHeight !== undefined && command.setCssSize === true) setCanvasCssSize(canvas, "height", sizing.cssHeight);
    setCanvasTransform(ctx, sizing.pixelRatio);

    for (const op of command.ops || []) applyCanvasOp(ctx, canvas, op);
    return commandMessage(command, {
      ref: refName(command.ref),
      width: canvas.width,
      height: canvas.height,
      cssWidth: sizing.cssWidth ?? canvas.width,
      cssHeight: sizing.cssHeight ?? canvas.height,
      pixelRatio: sizing.pixelRatio
    });
  };
}

export function registerCompiledCanvasDrawCommandHandlers(handlers) {
  handlers["canvas/draw"] = function(command, dispatch) {
    const canvas = resolveCompiledRef(command.ref, dispatch);
    if (!canvas) return compiledCommandErrorMessage(command, new Error(`Missing canvas ref ${String(command.ref)}`));
    const ctx = canvas.getContext?.("2d");
    if (!ctx) return compiledCommandErrorMessage(command, new Error("Canvas 2D unavailable"));

    const cssWidth = command.cssWidth;
    const cssHeight = command.cssHeight;
    const pixelRatio = Math.max(1, numberOrZero(globalThis.devicePixelRatio ?? 1));
    if (cssWidth !== undefined) canvas.width = Math.round(cssWidth * pixelRatio);
    if (cssHeight !== undefined) canvas.height = Math.round(cssHeight * pixelRatio);
    setCanvasTransform(ctx, pixelRatio);

    for (const op of command.ops || []) applyCompiledCanvasOp(ctx, canvas, op);
    return compiledCommandMessage(command, {
      ref: compiledRefName(command.ref),
      width: canvas.width,
      height: canvas.height,
      cssWidth: cssWidth ?? canvas.width,
      cssHeight: cssHeight ?? canvas.height,
      pixelRatio
    });
  };
}

export function registerCanvasMeasureTextCommandHandlers(handlers) {
  handlers["canvas/measure-text"] = function(command, dispatch) {
    const canvas = resolveRef(command.ref, dispatch);
    if (!canvas) return commandErrorMessage(command, new Error(`Missing canvas ref ${String(command.ref)}`));
    const ctx = canvas.getContext?.("2d");
    if (!ctx?.measureText) return commandErrorMessage(command, new Error("Canvas text unavailable"));

    applyCanvasState(ctx, command, "fill");
    const texts = canvasMeasureTexts(command);
    const measurements = texts.map((text) => {
      const metrics = ctx.measureText(text);
      return {
        text,
        width: numberOrZero(metrics?.width),
        actualBoundingBoxLeft: numberOrZero(metrics?.actualBoundingBoxLeft),
        actualBoundingBoxRight: numberOrZero(metrics?.actualBoundingBoxRight),
        actualBoundingBoxAscent: numberOrZero(metrics?.actualBoundingBoxAscent),
        actualBoundingBoxDescent: numberOrZero(metrics?.actualBoundingBoxDescent)
      };
    });
    return commandMessage(command, {
      ref: refName(command.ref),
      font: ctx.font || "",
      texts,
      widths: measurements.map((item) => item.width),
      measurements
    });
  };
}

export function registerDomRefCommandHandlers(handlers) {
  handlers["dom-ref/focus"] = function(command, dispatch) {
    const node = resolveRef(command.ref, dispatch);
    if (!node?.focus) return commandErrorMessage(command, new Error(`Cannot focus ${String(command.ref)}`));
    node.focus();
    return commandMessage(command, { ref: refName(command.ref) });
  };

  handlers["dom-ref/click"] = function(command, dispatch) {
    const node = resolveRef(command.ref, dispatch);
    if (!node?.click) return commandErrorMessage(command, new Error(`Cannot click ${String(command.ref)}`));
    node.click();
    return commandMessage(command, { ref: refName(command.ref) });
  };

  handlers["dom-ref/measure"] = function(command, dispatch) {
    const node = resolveRef(command.ref, dispatch);
    if (!node) return commandErrorMessage(command, new Error(`Missing DOM ref ${String(command.ref)}`));
    const rect = measureNode(node);
    return commandMessage(command, { ref: refName(command.ref), ...rect });
  };
}

export function registerCompiledDomRefCommandHandlers(handlers) {
  handlers["dom-ref/focus"] = function(command, dispatch) {
    const node = resolveCompiledRef(command.ref, dispatch);
    if (!node?.focus) return compiledCommandErrorMessage(command, new Error(`Cannot focus ${String(command.ref)}`));
    node.focus();
    return compiledCommandMessage(command, { ref: compiledRefName(command.ref) });
  };

  handlers["dom-ref/click"] = function(command, dispatch) {
    const node = resolveCompiledRef(command.ref, dispatch);
    if (!node?.click) return compiledCommandErrorMessage(command, new Error(`Cannot click ${String(command.ref)}`));
    node.click();
    return compiledCommandMessage(command, { ref: compiledRefName(command.ref) });
  };

  handlers["dom-ref/measure"] = function(command, dispatch) {
    const node = resolveCompiledRef(command.ref, dispatch);
    if (!node) return compiledCommandErrorMessage(command, new Error("Missing DOM ref"));
    return compiledCommandMessage(command, { ref: compiledRefName(command.ref), ...measureNode(node) });
  };
}

export function registerDomScrollCommandHandlers(handlers, context) {
  const { env, host } = context;
  const documentRef = env.document || host.document;
  const animation = env.animation || host;
  const requestAnimationFrameImpl = env.requestAnimationFrame || animation.requestAnimationFrame?.bind(animation);

  handlers["dom/scroll-into-view"] = function(command) {
    queueScrollIntoView(command, {
      document: documentRef,
      host,
      requestAnimationFrame: requestAnimationFrameImpl
    });
    return commandMessage(command, {
      id: command.id || "",
      ref: command.ref || "",
      selector: command.selector || "",
      testId: command.testId || ""
    });
  };
}

export function registerDomResizeCommandHandlers(handlers, context) {
  const { env, host } = context;
  const ResizeObserverCtor = env.ResizeObserver || host.ResizeObserver;
  const resizeTarget = env.resizeTarget || host;
  const resizeObservers = new Map();

  addCommandDisposer(context, () => {
    for (const entry of resizeObservers.values()) removeResizeObserver(entry);
    resizeObservers.clear();
  });

  handlers["dom-ref/resize-watch"] = function(command, dispatch) {
    const node = resolveRef(command.ref, dispatch);
    if (!node) return commandErrorMessage(command, new Error(`Missing DOM ref ${String(command.ref)}`));
    const id = command.id || refName(command.ref);
    const existing = resizeObservers.get(id);
    if (existing) removeResizeObserver(existing);

    if (ResizeObserverCtor) {
      const observer = new ResizeObserverCtor((entries) => {
        const entry = entries?.find((item) => item.target === node) || entries?.[0];
        const message = resizeMessage(command, id, node, rectFromResizeEntry(entry, node));
        if (message !== undefined) dispatch(message);
      });
      observer.observe(node);
      resizeObservers.set(id, { observer, node });
    } else if (resizeTarget?.addEventListener) {
      const listener = () => {
        const message = resizeMessage(command, id, node, measureNode(node));
        if (message !== undefined) dispatch(message);
      };
      resizeTarget.addEventListener("resize", listener);
      resizeObservers.set(id, { target: resizeTarget, listener });
    } else {
      return commandErrorMessage(command, new Error("Resize unavailable"));
    }

    return resizeMessage(command, id, node, measureNode(node));
  };

  handlers["dom-ref/resize-unwatch"] = function(command) {
    const id = command.id || command.ref;
    const existing = resizeObservers.get(id);
    if (existing) {
      removeResizeObserver(existing);
      resizeObservers.delete(id);
    }
    return commandMessage(command, { id });
  };
}

export function registerCompiledDomResizeCommandHandlers(handlers, context) {
  const ResizeObserverCtor = globalThis.ResizeObserver;
  const resizeObservers = new Map();

  addCommandDisposer(context, () => {
    for (const entry of resizeObservers.values()) removeResizeObserver(entry);
    resizeObservers.clear();
  });

  handlers["dom-ref/resize-watch"] = function(command, dispatch) {
    const node = resolveCompiledRef(command.ref, dispatch);
    if (!node) return compiledCommandErrorMessage(command, new Error(`Missing DOM ref ${String(command.ref)}`));
    const id = command.id;
    const existing = resizeObservers.get(id);
    if (existing) removeResizeObserver(existing);

    if (ResizeObserverCtor) {
      const observer = new ResizeObserverCtor((entries) => {
        const entry = entries?.find((item) => item.target === node) || entries?.[0];
        const message = compiledResizeMessage(command, id, node, rectFromResizeEntry(entry, node));
        if (message !== undefined) dispatch(message);
      });
      observer.observe(node);
      resizeObservers.set(id, { observer, node });
    } else if (globalThis.addEventListener) {
      const listener = () => {
        const message = compiledResizeMessage(command, id, node, measureNode(node));
        if (message !== undefined) dispatch(message);
      };
      globalThis.addEventListener("resize", listener);
      resizeObservers.set(id, { target: globalThis, listener });
    } else {
      return compiledCommandErrorMessage(command, new Error("Resize unavailable"));
    }

    return compiledResizeMessage(command, id, node, measureNode(node));
  };

  handlers["dom-ref/resize-unwatch"] = function(command) {
    const id = command.id;
    const existing = resizeObservers.get(id);
    if (existing) {
      removeResizeObserver(existing);
      resizeObservers.delete(id);
    }
    return compiledCommandMessage(command, { id });
  };
}

export function registerCompiledDirectDomResizeCommandHandlers(handlers, context) {
  const resizeObservers = new Map();

  addCommandDisposer(context, () => {
    for (const entry of resizeObservers.values()) removeResizeObserver(entry);
    resizeObservers.clear();
  });

  handlers["dom-ref/resize-watch/direct"] = function(command, dispatch) {
    const node = resolveCompiledRef(command.ref, dispatch);
    if (!node) return compiledCommandErrorMessage(command, new Error(`Missing DOM ref ${String(command.ref)}`));
    const id = command.id;
    const existing = resizeObservers.get(id);
    if (existing) removeResizeObserver(existing);

    const emit = (entry) => dispatch(command.m(entry, node, id));
    const observer = new ResizeObserver((entries) => emit(entries[0]));
    observer.observe(node);
    resizeObservers.set(id, { observer, node });
    return command.m(null, node, id);
  };
}

export function registerWindowEventCommandHandlers(handlers, context) {
  const { env, host } = context;
  const eventTarget = env.eventTarget || env.window || host;
  const windowEvents = new Map();

  addCommandDisposer(context, () => {
    for (const entry of windowEvents.values()) removeWindowEventListener(entry);
    windowEvents.clear();
  });

  handlers["window/event-watch"] = function(command, dispatch) {
    if (!eventTarget?.addEventListener) return commandErrorMessage(command, new Error("Events unavailable"));
    const type = String(command.type || command.event || "");
    if (!type) return commandErrorMessage(command, new Error("Missing event type"));
    const id = command.id || type;
    const existing = windowEvents.get(id);
    if (existing) removeWindowEventListener(existing);

    const options = eventListenerOptions(command.options);
    const listener = (event) => {
      applyWindowEventControls(command, event);
      const message = windowEventMessage(command, event, id, host);
      if (message !== undefined) dispatch(message);
    };
    eventTarget.addEventListener(type, listener, options);
    windowEvents.set(id, { target: eventTarget, type, listener, options });
    return commandMessage(command, { id, type });
  };

  handlers["window/event-unwatch"] = function(command) {
    const id = command.id || command.type || command.event;
    const existing = windowEvents.get(id);
    if (existing) {
      removeWindowEventListener(existing);
      windowEvents.delete(id);
    }
    return commandMessage(command, { id });
  };
}

export function registerCompiledWindowEventCommandHandlers(handlers, context) {
  const eventTarget = globalThis;
  const windowEvents = new Map();

  addCommandDisposer(context, () => {
    for (const entry of windowEvents.values()) removeWindowEventListener(entry);
    windowEvents.clear();
  });

  handlers["window/event-watch"] = function(command, dispatch) {
    if (!eventTarget?.addEventListener) return compiledCommandErrorMessage(command, new Error("Events unavailable"));
    const type = command.type;
    if (!type) return compiledCommandErrorMessage(command, new Error("Missing event type"));
    const id = command.id;
    const existing = windowEvents.get(id);
    if (existing) removeWindowEventListener(existing);

    const options = command.options;
    const listener = (event) => {
      applyCompiledWindowEventControls(command, event);
      const message = compiledWindowEventMessage(command, event, id);
      if (message !== undefined) dispatch(message);
    };
    eventTarget.addEventListener(type, listener, options);
    windowEvents.set(id, { target: eventTarget, type, listener, options });
    return compiledCommandMessage(command, { id, type });
  };

  handlers["window/event-unwatch"] = function(command) {
    const id = command.id;
    const existing = windowEvents.get(id);
    if (existing) {
      removeWindowEventListener(existing);
      windowEvents.delete(id);
    }
    return compiledCommandMessage(command, { id });
  };
}

export function registerCompiledDirectWindowEventCommandHandlers(handlers, context) {
  const windowEvents = new Map();

  addCommandDisposer(context, () => {
    for (const entry of windowEvents.values()) removeWindowEventListener(entry);
    windowEvents.clear();
  });

  handlers["window/event-watch/direct"] = function(command, dispatch) {
    const type = command.type;
    const id = command.id;
    const existing = windowEvents.get(id);
    if (existing) removeWindowEventListener(existing);

    const options = command.options;
    const listener = (event) => {
      if (command.p) event.preventDefault();
      if (command.q) event.stopPropagation();
      dispatch(command.m(event));
    };
    globalThis.addEventListener(type, listener, options);
    windowEvents.set(id, { target: globalThis, type, listener, options });
  };

  handlers["window/event-unwatch/direct"] = function(command) {
    const id = command.id;
    const existing = windowEvents.get(id);
    if (existing) {
      removeWindowEventListener(existing);
      windowEvents.delete(id);
    }
  };
}

export function registerMediaQueryCommandHandlers(handlers, context) {
  const matchMedia = context.env.matchMedia || context.host.matchMedia?.bind(context.host);
  const mediaQueries = new Map();

  addCommandDisposer(context, () => {
    for (const entry of mediaQueries.values()) removeMediaQueryListener(entry);
    mediaQueries.clear();
  });

  handlers["media-query/watch"] = function(command, dispatch) {
    if (!matchMedia) return commandErrorMessage(command, new Error("matchMedia unavailable"));
    const query = String(command.query || "");
    const id = command.id || query;
    const mediaQuery = matchMedia(query);
    const existing = mediaQueries.get(id);
    if (existing) removeMediaQueryListener(existing);

    const listener = (event) => {
      const message = mediaQueryMessage(command, event, id);
      if (message !== undefined) dispatch(message);
    };
    addMediaQueryListener(mediaQuery, listener);
    mediaQueries.set(id, { mediaQuery, listener });
    return mediaQueryMessage(command, mediaQuery, id);
  };

  handlers["media-query/unwatch"] = function(command) {
    const id = command.id || command.query;
    const existing = mediaQueries.get(id);
    if (existing) {
      removeMediaQueryListener(existing);
      mediaQueries.delete(id);
    }
    return commandMessage(command, { id });
  };
}

export function registerCompiledMediaQueryCommandHandlers(handlers, context) {
  const matchMedia = globalThis.matchMedia?.bind(globalThis);
  const mediaQueries = new Map();

  addCommandDisposer(context, () => {
    for (const entry of mediaQueries.values()) removeMediaQueryListener(entry);
    mediaQueries.clear();
  });

  handlers["media-query/watch"] = function(command, dispatch) {
    if (!matchMedia) return compiledCommandErrorMessage(command, new Error("matchMedia unavailable"));
    const query = command.query;
    const id = command.id;
    const mediaQuery = matchMedia(query);
    const existing = mediaQueries.get(id);
    if (existing) removeMediaQueryListener(existing);

    const listener = (event) => {
      const message = compiledMediaQueryMessage(command, event, id);
      if (message !== undefined) dispatch(message);
    };
    addMediaQueryListener(mediaQuery, listener);
    mediaQueries.set(id, { mediaQuery, listener });
    return compiledMediaQueryMessage(command, mediaQuery, id);
  };

  handlers["media-query/unwatch"] = function(command) {
    const id = command.id;
    const existing = mediaQueries.get(id);
    if (existing) {
      removeMediaQueryListener(existing);
      mediaQueries.delete(id);
    }
    return compiledCommandMessage(command, { id });
  };
}

export function registerCompiledDirectMediaQueryCommandHandlers(handlers, context) {
  const mediaQueries = new Map();

  addCommandDisposer(context, () => {
    for (const entry of mediaQueries.values()) removeMediaQueryListener(entry);
    mediaQueries.clear();
  });

  handlers["media-query/watch/direct"] = function(command, dispatch) {
    const query = command.query;
    const id = command.id;
    const mediaQuery = matchMedia(query);
    const existing = mediaQueries.get(id);
    if (existing) removeMediaQueryListener(existing);

    const listener = (event) => {
      dispatch(command.m(event));
    };
    addMediaQueryListener(mediaQuery, listener);
    mediaQueries.set(id, { mediaQuery, listener });
    return command.m(mediaQuery);
  };

  handlers["media-query/unwatch/direct"] = function(command) {
    const id = command.id;
    const existing = mediaQueries.get(id);
    if (existing) {
      removeMediaQueryListener(existing);
      mediaQueries.delete(id);
    }
  };
}

export function createBrowserBootInput(env = {}) {
  const host = env.host || globalThis;
  return {
    currentUrl: host.location?.href ?? ""
  };
}

export function createSubscriptionHandlersFor(commandHandlers) {
  const handlers = {
    start(subscription, dispatch) {
      const command = startCommandForSubscription(subscription);
      const kind = commandKind(command);
      const handler = commandHandlers[kind];
      if (typeof handler !== "function") {
        if (subscription?.onError !== undefined) {
          return commandErrorMessage(subscription, new Error(`No handler registered for subscription kind ${subscriptionKind(subscription)}`));
        }
        return undefined;
      }
      return handler(command, dispatch);
    },
    stop(subscription, dispatch) {
      const command = stopCommandForSubscription(subscription);
      if (!command) return undefined;
      const kind = commandKind(command);
      const handler = commandHandlers[kind];
      if (typeof handler !== "function") return undefined;
      return handler(command, dispatch);
    },
    dispose() {
      commandHandlers.dispose?.();
    }
  };

  Object.defineProperty(handlers, "__closkellCommandHandlers", {
    value: commandHandlers
  });

  return handlers;
}

export function createSubscriptionHandlers(env = {}) {
  const commandHandlers = env.commandHandlers || createCommandHandlers(env);
  return createSubscriptionHandlersFor(commandHandlers);
}

export function startConfiguredApp(options = {}) {
  const handlers = options.handlers || {};
  const subscriptionHandlers = options.subscriptionHandlers || createSubscriptionHandlersFor(handlers);
  return startAppCore(options, subscriptionHandlers);
}

export function startApp(options = {}) {
  const handlers = options.handlers || {};
  const subscriptionHandlers = options.subscriptionHandlers || createSubscriptionHandlers({ commandHandlers: handlers });
  return startAppCore(options, subscriptionHandlers);
}

export function startCompiledApp(options = {}) {
  const {
    root,
    init,
    update,
    view,
    subscriptions = () => null,
    handlers = {},
    boot = undefined
  } = options;
  const [initialState, initialCommand] = boot === undefined ? init() : init(boot);
  let state = initialState;
  let component = view(state);
  let disposed = false;
  const activeSubscriptions = new Map();
  const refs = new Map();
  const isActive = () => !disposed;

  const dispatch = (message, event) => {
    if (disposed) return state;
    const result = update(state, message, event);
    state = result[0];
    component.update(state, dispatch, null);
    syncSubscriptions();
    run(result[1]);
    return state;
  };
  dispatch.__closkellRefs = refs;

  component.mount(root, dispatch);
  syncSubscriptions();
  run(initialCommand);

  function run(command) {
    if (disposed) return;
    const commandKind = command.kind;
    if (commandKind === "none") return;
    if (commandKind === "batch") {
      for (const item of command.commands) run(item);
      return;
    }
    const handler = handlers[commandKind];
    settle(command, () => handler(command, dispatch), isActive);
  }

  function settle(effect, invoke, active) {
    let result;
    try {
      result = invoke();
    } catch (error) {
      const message = compiledCommandErrorMessage(effect, error);
      if (message !== undefined && active()) dispatch(message);
      return;
    }
    if (result && typeof result.then === "function") {
      result
        .then((message) => {
          if (message != null && active()) dispatch(message);
        })
        .catch((error) => {
          const message = compiledCommandErrorMessage(effect, error);
          if (message != null && active()) dispatch(message);
        });
    } else if (result != null && active()) {
      dispatch(result);
    }
  }

  function flatten(subscription, output) {
    if (!subscription) return output;
    const subscriptionKind = subscription.kind;
    if (subscriptionKind === "none") return output;
    if (subscriptionKind === "batch") {
      for (const item of subscription.subscriptions) {
        flatten(item, output);
      }
    } else {
      output.push(subscription);
    }
    return output;
  }

  function subscriptionKey(subscription) {
    return `${subscription.kind}:${subscription.id}`;
  }

  function stopCommand(subscription) {
    return compiledStopCommandForSubscription(subscription);
  }

  function runSubscription(subscription, active) {
    const command = compiledStartCommandForSubscription(subscription);
    const handler = handlers[command.kind];
    settle(command, () => handler(command, dispatch), active);
  }

  function syncSubscriptions() {
    if (disposed) return;
    const nextByKey = new Map();
    for (const subscription of flatten(subscriptions(state), [])) {
      const key = subscriptionKey(subscription);
      nextByKey.set(key, { subscription, signature: JSON.stringify(subscription) });
    }
    for (const [key, active] of Array.from(activeSubscriptions.entries())) {
      const next = nextByKey.get(key);
      if (next && next.signature === active.signature) continue;
      runCommand(stopCommand(active.subscription), () => false);
      activeSubscriptions.delete(key);
    }
    for (const [key, next] of nextByKey.entries()) {
      if (activeSubscriptions.has(key)) continue;
      activeSubscriptions.set(key, next);
      runSubscription(next.subscription, isActive);
    }
  }

  function runCommand(command, active) {
    if (!command) return;
    const handler = handlers[command.kind];
    settle(command, () => handler(command, dispatch), active);
  }

  return {
    dispatch,
    refs,
    getRef(name) {
      return refs.get(compiledRefName(name));
    },
    get state() {
      return state;
    },
    get subscriptions() {
      return Array.from(activeSubscriptions.values()).map((entry) => entry.subscription);
    },
    get root() {
      return component.root;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      for (const active of activeSubscriptions.values()) {
        runCommand(stopCommand(active.subscription), () => false);
      }
      activeSubscriptions.clear();
      handlers.dispose?.();
      component?.dispose?.();
    }
  };
}

export function startCompiledAppWithoutSubscriptions(options = {}) {
  const {
    root,
    init,
    update,
    view,
    handlers = {},
    boot = undefined
  } = options;
  const [initialState, initialCommand] = boot === undefined ? init() : init(boot);
  let state = initialState;
  let component = view(state);
  let disposed = false;
  const refs = new Map();
  const isActive = () => !disposed;

  const dispatch = (message, event) => {
    if (disposed) return state;
    const result = update(state, message, event);
    state = result[0];
    component.update(state, dispatch, null);
    run(result[1]);
    return state;
  };
  dispatch.__closkellRefs = refs;

  component.mount(root, dispatch);
  run(initialCommand);

  function run(command) {
    if (disposed) return;
    const commandKind = command.kind;
    if (commandKind === "none") return;
    if (commandKind === "batch") {
      for (const item of command.commands) run(item);
      return;
    }
    const handler = handlers[commandKind];
    settle(command, () => handler(command, dispatch), isActive);
  }

  function settle(effect, invoke, active) {
    let result;
    try {
      result = invoke();
    } catch (error) {
      const message = compiledCommandErrorMessage(effect, error);
      if (message !== undefined && active()) dispatch(message);
      return;
    }
    if (result && typeof result.then === "function") {
      result
        .then((message) => {
          if (message != null && active()) dispatch(message);
        })
        .catch((error) => {
          const message = compiledCommandErrorMessage(effect, error);
          if (message != null && active()) dispatch(message);
        });
    } else if (result != null && active()) {
      dispatch(result);
    }
  }

  return {
    dispatch,
    refs,
    getRef(name) {
      return refs.get(compiledRefName(name));
    },
    get state() {
      return state;
    },
    get subscriptions() {
      return [];
    },
    get root() {
      return component.root;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      handlers.dispose?.();
      component?.dispose?.();
    }
  };
}

function createCompiledSubscriptionHandlersFor(commandHandlers) {
  const handlers = {
    start(subscription, dispatch) {
      const command = compiledStartCommandForSubscription(subscription);
      const handler = commandHandlers[compiledCommandKind(command)];
      if (typeof handler !== "function") {
        if (subscription?.onError !== undefined) {
          return compiledCommandErrorMessage(subscription, new Error(`No handler registered for subscription kind ${compiledCommandKind(subscription)}`));
        }
        return undefined;
      }
      return handler(command, dispatch);
    },
    stop(subscription, dispatch) {
      const command = compiledStopCommandForSubscription(subscription);
      if (!command) return undefined;
      const handler = commandHandlers[compiledCommandKind(command)];
      if (typeof handler !== "function") return undefined;
      return handler(command, dispatch);
    },
    dispose() {
      commandHandlers.dispose?.();
    }
  };

  Object.defineProperty(handlers, "__closkellCommandHandlers", {
    value: commandHandlers
  });

  return handlers;
}

function startCompiledAppCore({
  root,
  init,
  update,
  view,
  subscriptions = null,
  handlers = {},
  boot = undefined
}, subscriptionHandlers) {
  const initValue = typeof init === "function"
    ? (boot === undefined ? init() : init(boot))
    : init;
  const [initialState, initialCommand] = normalizeUpdateResult(initValue);
  let state = initialState;
  let component = null;
  let disposed = false;
  const activeSubscriptions = new Map();
  const refs = new Map();
  const isActive = () => !disposed;

  const dispatch = (message, event) => {
    if (disposed) return state;
    const [nextState, command] = normalizeUpdateResult(update(state, message, event));
    state = nextState;
    component.update(state, dispatch, null);
    syncSubscriptions();
    runCompiledCommand(command, dispatch, handlers, isActive);
    return state;
  };
  dispatch.__closkellRefs = refs;

  component = view(state);
  component.mount(root, dispatch);
  syncSubscriptions();
  runCompiledCommand(initialCommand, dispatch, handlers, isActive);

  function syncSubscriptions() {
    if (typeof subscriptions !== "function" || disposed) return;
    reconcileCompiledSubscriptions(
      activeSubscriptions,
      flattenSubscriptions(subscriptions(state)),
      dispatch,
      subscriptionHandlers,
      isActive
    );
  }

  return {
    dispatch,
    refs,
    getRef(name) {
      return refs.get(refName(name));
    },
    get state() {
      return state;
    },
    get subscriptions() {
      return Array.from(activeSubscriptions.values()).map((entry) => entry.subscription);
    },
    get root() {
      return component.root;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      stopAllCompiledSubscriptions(activeSubscriptions, dispatch, subscriptionHandlers);
      if (subscriptionHandlers.__closkellCommandHandlers !== handlers) {
        subscriptionHandlers.dispose?.();
      }
      handlers.dispose?.();
      component?.dispose?.();
    }
  };
}

function startAppCore({
  root,
  init,
  update,
  view,
  subscriptions = null,
  handlers = {},
  onCommand = () => {},
  onSubscription = () => {},
  devtools = null,
  hydrate = false,
  boot = undefined
}, resolvedSubscriptionHandlers) {
  const initValue = typeof init === "function"
    ? (boot === undefined ? init() : init(boot))
    : init;
  const [initialState, initialCommand] = normalizeUpdateResult(initValue);
  let state = initialState;
  let component = null;
  let disposed = false;
  const commandLog = [];
  const subscriptionLog = [];
  const activeSubscriptions = new Map();
  const refs = new Map();
  const isActive = () => !disposed;

  const dispatch = (message, event) => {
    if (disposed) return state;
    emitDispatchDevtools(dispatch, { type: "message/dispatch", message, event, state });
    const previousState = state;
    const [nextState, command] = normalizeUpdateResult(update(state, message, event));
    state = nextState;
    const changedPaths = changedStatePaths(previousState, state);
    const updateContext = { changedPaths, devtools, frames: [] };
    component.update(state, dispatch, updateContext);
    emitDispatchDevtools(dispatch, {
      type: "state/update",
      message,
      previousState,
      state,
      command,
      changedPaths
    });
    syncSubscriptions();
    runCommand(command, dispatch, handlers, commandLog, onCommand, isActive);
    return state;
  };
  dispatch.__closkellRefs = refs;
  dispatch.__closkellDevtools = devtools;

  emitDevtools(devtools, { type: "app/init", state });
  component = view(state);
  mountAppComponent(root, component, dispatch, hydrate);
  emitDevtools(devtools, { type: "app/mount", root: component.root, state });
  syncSubscriptions();
  runCommand(initialCommand, dispatch, handlers, commandLog, onCommand, isActive);

  function syncSubscriptions() {
    if (typeof subscriptions !== "function" || disposed) return;
    const nextSubscriptions = flattenSubscriptions(subscriptions(state));
    reconcileSubscriptions(
      activeSubscriptions,
      nextSubscriptions,
      dispatch,
      resolvedSubscriptionHandlers,
      subscriptionLog,
      onSubscription,
      isActive
    );
  }

  return {
    dispatch,
    commands: commandLog,
    subscriptionEvents: subscriptionLog,
    refs,
    getRef(name) {
      return refs.get(refName(name));
    },
    get state() {
      return state;
    },
    get subscriptions() {
      return Array.from(activeSubscriptions.values()).map((entry) => entry.subscription);
    },
    get root() {
      return component.root;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      stopAllSubscriptions(
        activeSubscriptions,
        dispatch,
        resolvedSubscriptionHandlers,
        subscriptionLog,
        onSubscription
      );
      if (resolvedSubscriptionHandlers.__closkellCommandHandlers !== handlers) {
        resolvedSubscriptionHandlers.dispose?.();
      }
      handlers.dispose?.();
      component?.dispose?.();
      emitDevtools(devtools, { type: "app/dispose", state });
    }
  };
}

export function hydrateApp(options = {}) {
  const root = resolveHydrationRoot(options.root);
  const initState = options.initState ?? options.state;
  return startApp({
    ...options,
    root,
    init: [initState, { kind: Symbol.for("none") }],
    hydrate: true
  });
}

function mountAppComponent(root, component, dispatch, hydrate) {
  const previousChildren = hydrate ? Array.from(root?.childNodes ?? root?.children ?? []) : [];
  const hydrateNode = hydrate ? hydrationCandidateForComponent(root, component) : null;
  component.mount(root, dispatch, hydrateNode);
  if (!hydrate) return;
  for (const child of previousChildren) {
    if (child !== component.root && child?.parentNode === root) {
      root.removeChild?.(child);
    }
  }
  component.root?.setAttribute?.("data-closkell-hydrated", "");
}

function hydrationCandidateForComponent(root, component) {
  const templateName = component?.definition?.name;
  if (!templateName) return null;
  return Array.from(root?.childNodes ?? root?.children ?? []).find(
    (child) => child?.nodeType === 1 && child.getAttribute?.("data-closkell-template") === templateName
  ) || null;
}

function resolveHydrationRoot(root) {
  if (typeof root === "string") {
    const resolved = globalThis.document?.querySelector?.(root);
    if (resolved) return resolved;
    throw new Error(`hydrateApp root selector did not match: ${root}`);
  }
  const resolved = root ?? globalThis.document?.getElementById?.("app");
  if (!resolved) throw new Error("hydrateApp requires a root element or selector.");
  return resolved;
}

function flattenSubscriptions(subscription) {
  if (subscription == null || subscription === false) return [];
  if (Array.isArray(subscription)) return subscription.flatMap(flattenSubscriptions);

  const kind = subscriptionKind(subscription);
  if (!kind || kind === "none") return [];
  if (kind === "batch") {
    return flattenSubscriptions(subscription.subscriptions ?? subscription.subs ?? subscription.commands);
  }
  return [subscription];
}

function reconcileSubscriptions(
  activeSubscriptions,
  nextSubscriptions,
  dispatch,
  handlers,
  subscriptionLog,
  onSubscription,
  isActive
) {
  const nextByKey = new Map();
  for (const subscription of nextSubscriptions) {
    const key = subscriptionKey(subscription);
    if (!key) continue;
    nextByKey.set(key, {
      subscription,
      signature: subscriptionSignature(subscription)
    });
  }

  for (const [key, active] of Array.from(activeSubscriptions.entries())) {
    const next = nextByKey.get(key);
    if (next && next.signature === active.signature) continue;
    stopSubscription(active.subscription, dispatch, handlers, subscriptionLog, onSubscription);
    activeSubscriptions.delete(key);
  }

  for (const [key, next] of nextByKey.entries()) {
    if (activeSubscriptions.has(key)) continue;
    activeSubscriptions.set(key, next);
    startSubscription(next.subscription, dispatch, handlers, subscriptionLog, onSubscription, isActive);
  }
}

function reconcileCompiledSubscriptions(activeSubscriptions, nextSubscriptions, dispatch, handlers, isActive) {
  const nextByKey = new Map();
  for (const subscription of nextSubscriptions) {
    const key = compiledSubscriptionKey(subscription);
    if (!key) continue;
    nextByKey.set(key, {
      subscription,
      signature: compiledSubscriptionSignature(subscription)
    });
  }

  for (const [key, active] of Array.from(activeSubscriptions.entries())) {
    const next = nextByKey.get(key);
    if (next && next.signature === active.signature) continue;
    runCompiledSubscriptionHandler("stop", active.subscription, dispatch, handlers, () => false);
    activeSubscriptions.delete(key);
  }

  for (const [key, next] of nextByKey.entries()) {
    if (activeSubscriptions.has(key)) continue;
    activeSubscriptions.set(key, next);
    runCompiledSubscriptionHandler("start", next.subscription, dispatch, handlers, isActive);
  }
}

function stopAllSubscriptions(activeSubscriptions, dispatch, handlers, subscriptionLog, onSubscription) {
  for (const active of activeSubscriptions.values()) {
    stopSubscription(active.subscription, dispatch, handlers, subscriptionLog, onSubscription);
  }
  activeSubscriptions.clear();
}

function stopAllCompiledSubscriptions(activeSubscriptions, dispatch, handlers) {
  for (const active of activeSubscriptions.values()) {
    runCompiledSubscriptionHandler("stop", active.subscription, dispatch, handlers, () => false);
  }
  activeSubscriptions.clear();
}

function startSubscription(subscription, dispatch, handlers, subscriptionLog, onSubscription, isActive) {
  const kind = subscriptionKind(subscription);
  const event = { type: "subscription/start", kind, subscription };
  subscriptionLog.push(event);
  onSubscription(event);
  emitDispatchDevtools(dispatch, event);
  runSubscriptionHandler("start", subscription, dispatch, handlers, isActive);
}

function stopSubscription(subscription, dispatch, handlers, subscriptionLog, onSubscription) {
  const kind = subscriptionKind(subscription);
  const event = { type: "subscription/stop", kind, subscription };
  subscriptionLog.push(event);
  onSubscription(event);
  emitDispatchDevtools(dispatch, event);
  runSubscriptionHandler("stop", subscription, dispatch, handlers, () => false);
}

function runSubscriptionHandler(action, subscription, dispatch, handlers, isActive) {
  let result;
  try {
    result = callSubscriptionHandler(action, subscription, dispatch, handlers);
  } catch (error) {
    handleSubscriptionError(subscription, error, dispatch, isActive);
    return;
  }

  if (result && typeof result.then === "function") {
    result
      .then((message) => {
        if (message !== undefined && message !== null && isActive()) dispatch(message);
      })
      .catch((error) => handleSubscriptionError(subscription, error, dispatch, isActive));
  } else if (result !== undefined && result !== null && isActive()) {
    dispatch(result);
  }
}

function runCompiledSubscriptionHandler(action, subscription, dispatch, handlers, isActive) {
  let result;
  try {
    result = callCompiledSubscriptionHandler(action, subscription, dispatch, handlers);
  } catch (error) {
    dispatchCompiledCommandError(subscription, error, dispatch, isActive);
    return;
  }

  if (result && typeof result.then === "function") {
    result
      .then((message) => {
        if (message !== undefined && message !== null && isActive()) dispatch(message);
      })
      .catch((error) => dispatchCompiledCommandError(subscription, error, dispatch, isActive));
  } else if (result !== undefined && result !== null && isActive()) {
    dispatch(result);
  }
}

function callSubscriptionHandler(action, subscription, dispatch, handlers) {
  if (typeof handlers?.[action] === "function") {
    return handlers[action](subscription, dispatch);
  }

  const byKind = handlers?.[subscriptionKind(subscription)];
  if (typeof byKind === "function" && action === "start") {
    return byKind(subscription, dispatch);
  }
  if (typeof byKind?.[action] === "function") {
    return byKind[action](subscription, dispatch);
  }
  return undefined;
}

function callCompiledSubscriptionHandler(action, subscription, dispatch, handlers) {
  if (typeof handlers?.[action] === "function") {
    return handlers[action](subscription, dispatch);
  }

  const byKind = handlers?.[compiledCommandKind(subscription)];
  if (typeof byKind === "function" && action === "start") {
    return byKind(subscription, dispatch);
  }
  if (typeof byKind?.[action] === "function") {
    return byKind[action](subscription, dispatch);
  }
  return undefined;
}

function handleSubscriptionError(subscription, error, dispatch, isActive) {
  emitDispatchDevtools(dispatch, {
    type: "subscription/error",
    kind: subscriptionKind(subscription),
    subscription,
    error: errorMessage(error)
  });
  if (!isActive()) return;
  const message = commandErrorMessage(subscription, error);
  if (message !== undefined && isActive()) dispatch(message);
}

function normalizeUpdateResult(result) {
  if (Array.isArray(result) && result.length === 2) {
    return result;
  }
  return [result, { kind: "none" }];
}

function runCommand(command, dispatch, handlers, commandLog, onCommand, isActive = () => true) {
  if (!command) return;
  if (!isActive()) return;

  if (Array.isArray(command)) {
    command.forEach((item) => runCommand(item, dispatch, handlers, commandLog, onCommand, isActive));
    return;
  }

  const kind = compiledCommandKind(command);
  if (kind === "none") return;

  if (kind === "batch") {
    (command.commands || []).forEach((item) => runCommand(item, dispatch, handlers, commandLog, onCommand, isActive));
    return;
  }

  commandLog.push({ kind, command });
  onCommand({ kind, command });
  emitDispatchDevtools(dispatch, { type: "command/run", kind, command });

  const handler = handlers[kind];
  if (typeof handler !== "function") {
    if (command.onError !== undefined) {
      handleCommandError(command, new Error(`No handler registered for command kind ${kind}`), dispatch, isActive);
    }
    return;
  }

  let result;
  try {
    result = handler(command, dispatch);
  } catch (error) {
    handleCommandError(command, error, dispatch, isActive);
    return;
  }

  if (result && typeof result.then === "function") {
    result
      .then((message) => {
        if (message != null && isActive()) dispatch(message);
      })
      .catch((error) => {
        handleCommandError(command, error, dispatch, isActive);
      });
  } else if (result != null && isActive()) {
    dispatch(result);
  }
}

function runCompiledCommand(command, dispatch, handlers, isActive = () => true) {
  if (!command || !isActive()) return;

  if (Array.isArray(command)) {
    command.forEach((item) => runCompiledCommand(item, dispatch, handlers, isActive));
    return;
  }

  const kind = compiledCommandKind(command);
  if (kind === "none") return;

  if (kind === "batch") {
    (command.commands || []).forEach((item) => runCompiledCommand(item, dispatch, handlers, isActive));
    return;
  }

  const handler = handlers[kind];
  if (typeof handler !== "function") {
    if (command.onError !== undefined) {
      dispatchCompiledCommandError(command, new Error(`No handler ${kind}`), dispatch, isActive);
    }
    return;
  }

  let result;
  try {
    result = handler(command, dispatch);
  } catch (error) {
    dispatchCompiledCommandError(command, error, dispatch, isActive);
    return;
  }

  if (result && typeof result.then === "function") {
    result
      .then((message) => {
        if (message != null && isActive()) dispatch(message);
      })
      .catch((error) => dispatchCompiledCommandError(command, error, dispatch, isActive));
  } else if (result != null && isActive()) {
    dispatch(result);
  }
}

function dispatchCompiledCommandError(command, error, dispatch, isActive) {
  if (!isActive()) return;
  const message = compiledCommandErrorMessage(command, error);
  if (message !== undefined && isActive()) dispatch(message);
}

async function runTask(task, context = {}) {
  if (task == null) return task;
  if (typeof task === "function") return task();
  if (typeof task?.then === "function") return task;

  const kind = commandKind(task);
  switch (kind) {
    case "task/succeed":
      return task.value;
    case "task/fail":
      throw task.error;
    case "task/map": {
      const value = await runTask(task.task, context);
      return task.mapper(value);
    }
    case "task/map-error": {
      try {
        return await runTask(task.task, context);
      } catch (error) {
        throw task.mapper(error);
      }
    }
    case "task/and-then": {
      const value = await runTask(task.task, context);
      return runTask(task.next(value), context);
    }
    case "task/http/get-text":
      return runHttpTask(task, context, "text");
    case "task/http/get-json":
      return runHttpTask(task, context, "json");
    default:
      throw new Error(`Unknown task kind ${kind || String(task.kind || "")}`);
  }
}

async function runHttpTask(task, context, format) {
  const fetchImpl = context.fetch || globalThis.fetch;
  if (!fetchImpl) throw "No fetch implementation is available for HTTP tasks.";

  try {
    const response = await fetchImpl(task.url, task.options || {});
    if (!response?.ok) {
      const status = response?.status ?? 0;
      const statusText = response?.statusText || "HTTP request failed";
      throw `HTTP ${status} ${statusText}`;
    }
    return format === "json" ? response.json() : response.text();
  } catch (error) {
    throw typeof error === "string" ? error : errorMessage(error);
  }
}

function taskSuccessMessage(command, value) {
  if (typeof command.onSuccess === "function") return command.onSuccess(value);
  return commandMessage(command, value);
}

function taskErrorMessage(command, error) {
  if (typeof command.onError === "function") return command.onError(error);
  return commandErrorMessage(command, error);
}

function flattenTestEntries(entries) {
  return entries.flatMap((entry) => {
    if (entry == null || entry === false) return [];
    if (Array.isArray(entry)) return flattenTestEntries(entry);
    return [entry];
  });
}

function flattenAssertions(assertions) {
  return assertions.flatMap((assertion) => {
    if (assertion == null || assertion === false) return [];
    if (Array.isArray(assertion)) return flattenAssertions(assertion);
    return [assertion];
  });
}

function moduleTestEntries(moduleExports) {
  if ("tests" in (moduleExports || {})) {
    return normalizeModuleTestEntries(moduleExports.tests, true);
  }
  return Object.keys(moduleExports || {})
    .sort()
    .flatMap((name) => normalizeModuleTestEntries(moduleExports[name], false));
}

function normalizeModuleTestEntries(value, strict) {
  if (value == null || value === false) return [];
  if (Array.isArray(value)) return value.flatMap((entry) => normalizeModuleTestEntries(entry, strict));
  if (isTestGroup(value) || isTestCase(value) || isLegacyTestRecord(value)) return [value];
  return strict ? [value] : [];
}

function flattenModuleTestEntries(value, prefix = [], strict = true) {
  if (value == null || value === false) return [];
  if (Array.isArray(value)) {
    return value.flatMap((entry) => flattenModuleTestEntries(entry, prefix, strict));
  }
  if (isTestGroup(value)) {
    const nextPrefix = [...prefix, String(value.name ?? "")];
    return flattenModuleTestEntries(value.tests ?? [], nextPrefix, strict);
  }
  if (isTestCase(value)) {
    return [{ ...value, name: fullTestName(prefix, closkellTestName(value, 0)) }];
  }
  if (isLegacyTestRecord(value)) {
    const name = value.name ? fullTestName(prefix, String(value.name)) : fullTestName(prefix, "");
    return [{ ...value, name }];
  }
  return strict ? [value] : [];
}

function registerVitestEntry(entry, describeFn, testFn, index) {
  if (Array.isArray(entry)) {
    for (const item of entry) index = registerVitestEntry(item, describeFn, testFn, index);
    return index;
  }
  if (entry == null || entry === false) return index;
  if (isTestGroup(entry)) {
    describeFn(String(entry.name ?? ""), () => {
      let groupIndex = 0;
      for (const item of normalizeModuleTestEntries(entry.tests ?? [], true)) {
        groupIndex = registerVitestEntry(item, describeFn, testFn, groupIndex);
      }
    });
    return index;
  }
  if (isTestCase(entry) || isLegacyTestRecord(entry)) {
    const testIndex = index;
    testFn(closkellTestName(entry, testIndex), () => {
      const result = runCloskellTest(entry, testIndex);
      if (!result.ok) throw new Error(formatTestFailure(result));
    });
    return index + 1;
  }
  return index;
}

function isTestGroup(value) {
  return value && typeof value === "object" && value.__closkellTestGroup;
}

function isTestCase(value) {
  return value && typeof value === "object" && value.__closkellTest;
}

function isLegacyTestRecord(value) {
  return value && typeof value === "object" && ("actual" in value || "expected" in value);
}

function assertionKind(assertion) {
  const kind = assertion?.__closkellAssert;
  if (typeof kind === "symbol") return symbolKey(kind);
  return String(kind ?? "");
}

function closkellTestAssertions(testValue) {
  if (isTestCase(testValue)) return testValue.assertions ?? [];
  return [testValue];
}

function closkellTestName(testValue, index) {
  if (testValue && typeof testValue.name === "string" && testValue.name.length > 0) {
    return testValue.name;
  }
  return `test ${index + 1}`;
}

function fullTestName(prefix, name) {
  return [...prefix, name].filter((part) => part && part.length > 0).join(" / ");
}

function runEqualAssertion(actual, expected, negated) {
  const equal = deepEqual(actual, expected);
  if (negated ? !equal : equal) return { ok: true };
  return {
    ok: false,
    expected: negated ? `not ${formatTestValue(expected)}` : formatTestValue(expected),
    actual: formatTestValue(actual)
  };
}

function runMatchAssertion(actual, pattern) {
  if (deepMatch(actual, pattern)) return { ok: true };
  return {
    ok: false,
    expected: `match ${formatTestValue(pattern)}`,
    actual: formatTestValue(actual)
  };
}

function runThrowsAssertion(thunk, expected) {
  if (typeof thunk !== "function") {
    return { ok: false, expected: "function that throws", actual: formatTestValue(thunk) };
  }
  try {
    thunk();
  } catch (error) {
    const message = errorMessage(error);
    if (expected === undefined || String(message).includes(String(expected))) return { ok: true };
    return {
      ok: false,
      expected: `throw containing ${formatTestValue(expected)}`,
      actual: formatTestValue(message)
    };
  }
  return { ok: false, expected: "throw", actual: "no throw" };
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
  if (left instanceof Set || right instanceof Set) {
    if (!(left instanceof Set) || !(right instanceof Set) || left.size !== right.size) return false;
    return Array.from(left).every((leftItem) =>
      Array.from(right).some((rightItem) => deepEqual(leftItem, rightItem))
    );
  }
  if (left instanceof Map || right instanceof Map) {
    if (!(left instanceof Map) || !(right instanceof Map) || left.size !== right.size) return false;
    return Array.from(left).every(([key, value]) => right.has(key) && deepEqual(value, right.get(key)));
  }
  if (isPlainObject(left) || isPlainObject(right)) {
    if (!isPlainObject(left) || !isPlainObject(right)) return false;
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
    return Array.from(pattern).every(([key, value]) => actual.has(key) && deepMatch(actual.get(key), value));
  }
  if (isPlainObject(pattern)) {
    if (!isPlainObject(actual)) return false;
    return Object.keys(pattern).every((key) => deepMatch(actual[key], pattern[key]));
  }
  return false;
}

function isPlainObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function symbolKey(value) {
  if (typeof value !== "symbol") return null;
  return Symbol.keyFor(value) ?? value.description ?? "";
}

function formatTestValue(value) {
  if (typeof value === "symbol") return `:${symbolKey(value)}`;
  if (typeof value === "undefined") return "undefined";
  if (value instanceof Set) return `#{${Array.from(value).map(formatTestValue).join(" ")}}`;
  if (value instanceof Map) {
    const entries = Array.from(value.entries()).map(([key, entry]) => `${formatTestValue(key)} ${formatTestValue(entry)}`);
    return `(hash-map ${entries.join(" ")})`;
  }
  return JSON.stringify(
    value,
    (_key, next) => (typeof next === "symbol" ? `:${symbolKey(next)}` : next)
  );
}

function formatTestFailure(result) {
  if (result.error) return result.error;
  return `expected ${result.expected}, actual ${result.actual}`;
}

function harnessRoot(harness) {
  if (harness?.__closkellHarness) return harness.root;
  return harness?.root ?? harness;
}

function testDispatchForHarness(harness) {
  const dispatch = (message, event) => {
    harness.messages.push(message);
    if (event) harness.events.push(testEventSnapshot(event));
  };
  dispatch.__closkellRefs = new Map();
  dispatch.__closkellDevtools = {
    emit(event) {
      if (event?.type === "template/update") harness.frames.push(event);
    }
  };
  return dispatch;
}

function serverRenderDispatch() {
  const dispatch = () => {};
  dispatch.__closkellRefs = new Map();
  dispatch.__closkellDevtools = { emit() {} };
  return dispatch;
}

function annotateServerRenderedComponent(component, seen = new Set()) {
  if (!component || seen.has(component)) return;
  seen.add(component);

  const root = component.root;
  const name = component.definition?.name;
  if (root?.nodeType === 1 && name) {
    root.setAttribute?.("data-closkell-template", name);
    const slots = serverSlotMetadata(component.definition);
    if (slots.length) root.setAttribute?.("data-closkell-slots", JSON.stringify(slots));
  }

  const instance = component.__closkellInstance;
  if (!instance) return;
  for (const slot of instance.componentSlots || []) {
    annotateServerRenderedComponent(slot?.component, seen);
  }
  for (const slot of instance.conditionalSlots || []) {
    annotateServerRenderedComponent(slot?.component, seen);
  }
  for (const slot of instance.keyedSlots || []) {
    for (const entry of slot?.byKey?.values?.() || []) {
      annotateServerRenderedComponent(entry.component, seen);
    }
  }
}

function serverSlotMetadata(definition) {
  return (definition?.slots || [])
    .filter((slot) => slot?.kind?.event || slot?.kind?.ref)
    .map((slot) => ({
      id: slot.id,
      node: slot.node,
      kind: slot.kind
    }));
}

function dispatchTestEvent(harness, node, type, init = {}) {
  const event = createTestEvent(type, { ...init, target: node, currentTarget: node });
  node.dispatchEvent?.(event);
  harness?.events?.push?.(testEventSnapshot(event));
  return event;
}

function createTestEvent(type, init = {}) {
  const event = {
    type: String(type),
    bubbles: init.bubbles !== false,
    cancelable: init.cancelable !== false,
    defaultPrevented: false,
    propagationStopped: false,
    target: init.target ?? null,
    currentTarget: init.currentTarget ?? init.target ?? null,
    key: init.key ?? "",
    altKey: Boolean(init.altKey),
    ctrlKey: Boolean(init.ctrlKey),
    metaKey: Boolean(init.metaKey),
    shiftKey: Boolean(init.shiftKey),
    clientX: Number(init.clientX ?? 0),
    clientY: Number(init.clientY ?? 0),
    preventDefault() {
      this.defaultPrevented = true;
    },
    stopPropagation() {
      this.propagationStopped = true;
    }
  };
  return event;
}

function testEventSnapshot(event) {
  return {
    type: event.type,
    defaultPrevented: Boolean(event.defaultPrevented),
    propagationStopped: Boolean(event.propagationStopped)
  };
}

function testEventInit(valueOrInit) {
  if (valueOrInit && typeof valueOrInit === "object" && !Array.isArray(valueOrInit)) {
    return valueOrInit;
  }
  return { value: valueOrInit };
}

function applyTestInputValue(node, init) {
  if ("value" in init) {
    node.value = String(init.value ?? "");
    node.setAttribute?.("value", node.value);
  }
  if ("checked" in init) {
    node.checked = Boolean(init.checked);
    if (node.checked) node.setAttribute?.("checked", "");
    else node.removeAttribute?.("checked");
  }
}

function testOption(value, name) {
  if (value instanceof Map) {
    if (value.has(name)) return value.get(name);
    const symbol = Symbol.for(name);
    if (value.has(symbol)) return value.get(symbol);
  }
  return value?.[name];
}

function normalizeTestHandlers(handlers) {
  if (!handlers) return null;
  if (!(handlers instanceof Map)) return handlers;
  const normalized = {};
  for (const [key, value] of handlers.entries()) normalized[handlerKey(key)] = value;
  return normalized;
}

function handlerKey(key) {
  if (typeof key === "symbol") return Symbol.keyFor(key) || key.description || String(key);
  return String(key);
}

function normalizeTestCommandEnv(options = {}) {
  const env = options instanceof Map ? normalizeTestHandlers(options) : { ...(options || {}) };
  if (env.now !== undefined && typeof env.now !== "function") {
    const value = Number(env.now);
    env.now = () => value;
  }
  if (env.random !== undefined && typeof env.random !== "function") {
    const value = Number(env.random);
    env.random = () => value;
  }
  if (env.storage instanceof Map) env.storage = testStorageFromMap(env.storage);
  return env;
}

function testStorageFromMap(values) {
  return {
    values: new Map(values),
    getItem(key) {
      return this.values.has(key) ? this.values.get(key) : null;
    },
    setItem(key, value) {
      this.values.set(key, String(value));
    },
    removeItem(key) {
      this.values.delete(key);
    }
  };
}

function siblingForNode(node, offset) {
  const siblings = node?.parentNode?.children;
  if (!siblings) return null;
  const index = siblings.indexOf(node);
  return index === -1 ? null : siblings[index + offset] || null;
}

function querySelectorAllFrom(root, selector) {
  const selectors = String(selector || "").split(",").map((part) => part.trim()).filter(Boolean);
  const matches = [];
  for (const node of descendantsOf(root)) {
    if (selectors.some((candidate) => selectorMatchesNode(node, candidate))) matches.push(node);
  }
  return matches;
}

function descendantsOf(root) {
  const nodes = [];
  for (const child of root?.children || []) {
    nodes.push(child);
    nodes.push(...descendantsOf(child));
  }
  return nodes;
}

function selectorMatchesNode(node, selector) {
  if (!node || node.nodeType !== 1) return false;
  const parts = String(selector || "").trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return false;
  return selectorChainMatches(node, parts.length - 1, parts);
}

function selectorChainMatches(node, index, parts) {
  if (!simpleSelectorMatches(node, parts[index])) return false;
  if (index === 0) return true;
  let parent = node.parentNode;
  while (parent) {
    if (selectorChainMatches(parent, index - 1, parts)) return true;
    parent = parent.parentNode;
  }
  return false;
}

function simpleSelectorMatches(node, selector) {
  if (selector === "*") return true;
  const tagMatch = selector.match(/^[A-Za-z][A-Za-z0-9_-]*/);
  if (tagMatch && node.tagName !== tagMatch[0].toLowerCase()) return false;

  const idMatches = selector.match(/#[A-Za-z0-9_-]+/g) || [];
  for (const id of idMatches) {
    if (node.getAttribute?.("id") !== id.slice(1)) return false;
  }

  const classMatches = selector.match(/\.[A-Za-z0-9_-]+/g) || [];
  const classes = (node.getAttribute?.("class") || "").split(/\s+/).filter(Boolean);
  for (const className of classMatches) {
    if (!classes.includes(className.slice(1))) return false;
  }

  const attrMatches = selector.match(/\[[^\]]+\]/g) || [];
  for (const raw of attrMatches) {
    const content = raw.slice(1, -1).trim();
    const match = content.match(/^([^=\s]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s]+)))?$/);
    if (!match) return false;
    const name = match[1];
    const expected = match[2] ?? match[3] ?? match[4];
    if (!node.hasAttribute?.(name)) return false;
    if (expected !== undefined && node.getAttribute?.(name) !== expected) return false;
  }

  return true;
}

function serializeTestNode(node) {
  if (node?.nodeType === 3) return escapeHtmlText(node.nodeValue);
  if (node?.nodeType === 11) return node.children.map(serializeTestNode).join("");
  const attrs = serializableAttributes(node)
    .map(([name, value]) => value === "" ? ` ${name}` : ` ${name}="${escapeHtmlText(value)}"`)
    .join("");
  const children = (node?.children || []).map(serializeTestNode).join("");
  return `<${node.tagName}${attrs}>${children}</${node.tagName}>`;
}

function serializableAttributes(node) {
  const attrs = Object.entries(node?.attributes || {});
  const hasStyleAttr = attrs.some(([name]) => name === "style");
  const styleText = String(node?.style?.cssText || "");
  if (styleText && !hasStyleAttr) attrs.push(["style", styleText]);
  return attrs;
}

function escapeHtmlText(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function handleCommandError(command, error, dispatch, isActive) {
  if (!isActive()) return;
  emitDispatchDevtools(dispatch, {
    type: "command/error",
    kind: commandKind(command),
    command,
    error: errorMessage(error)
  });
  const message = commandErrorMessage(command, error);
  if (message !== undefined && isActive()) dispatch(message);
}

function emitDispatchDevtools(dispatch, event) {
  emitDevtools(dispatch?.__closkellDevtools, event);
}

function emitDevtools(devtools, event) {
  if (!devtools) return;
  if (typeof devtools === "function") {
    devtools(event);
    return;
  }
  if (typeof devtools.emit === "function") {
    devtools.emit(event);
    return;
  }
  if (Array.isArray(devtools.events)) {
    devtools.events.push(event);
  }
}

function changedStatePaths(previous, next, prefix = "state", seen = new WeakMap()) {
  if (Object.is(previous, next)) return [];
  if (previous instanceof Set || next instanceof Set) {
    return setsEqual(previous, next) ? [] : [prefix];
  }
  if (previous instanceof Map || next instanceof Map) {
    return mapsEqual(previous, next) ? [] : [prefix];
  }
  if (!isChangeObject(previous) || !isChangeObject(next)) return [prefix];

  if (seen.get(previous) === next) return [prefix];
  seen.set(previous, next);

  const keys = new Set([...Object.keys(previous), ...Object.keys(next)]);
  const paths = [];
  for (const key of keys) {
    paths.push(...changedStatePaths(previous[key], next[key], `${prefix}.${key}`, seen));
  }
  return paths;
}

function setsEqual(previous, next) {
  if (!(previous instanceof Set) || !(next instanceof Set)) return false;
  if (previous.size !== next.size) return false;
  for (const value of previous) {
    if (!next.has(value)) return false;
  }
  return true;
}

function mapsEqual(previous, next) {
  if (!(previous instanceof Map) || !(next instanceof Map)) return false;
  if (previous.size !== next.size) return false;
  for (const [key, value] of previous) {
    if (!next.has(key) || !Object.is(value, next.get(key))) return false;
  }
  return true;
}

function isChangeObject(value) {
  return value !== null && typeof value === "object";
}

function isStatePath(path) {
  return path === "state" || path.startsWith("state.");
}

function isLocalReadPath(path, prefixes) {
  return prefixes.some((prefix) => path === prefix || path.startsWith(`${prefix}.`));
}

function changedPathsForUpdate(updateContext) {
  const changedPaths = updateContext.changedPaths || [];
  const localChangedPaths = updateContext.localChangedPaths || [];
  return localChangedPaths.length ? [...changedPaths, ...localChangedPaths] : changedPaths;
}

function pathsOverlap(read, changed) {
  return read === changed || read.startsWith(`${changed}.`) || changed.startsWith(`${read}.`);
}

function forceUpdateContext(updateContext) {
  if (!updateContext) return null;
  return { ...updateContext, force: true, frames: updateContext.frames };
}

function commandKind(command) {
  const kind = command.kind;
  if (typeof kind === "symbol") {
    return Symbol.keyFor(kind) || kind.description || "";
  }
  return String(kind || "");
}

function compiledCommandKind(command) {
  return command?.kind || "";
}

function scopeKey(value) {
  if (typeof value === "symbol") return Symbol.keyFor(value) || value.description || String(value);
  return String(value || "");
}

function wrapScopedMessage(message, tag) {
  if (message === undefined || message === null) return message;
  return { kind: tag, msg: message };
}

function scopedMessageDispatch(parentDispatch, tag) {
  const dispatch = (message, event) => parentDispatch?.(wrapScopedMessage(message, tag), event);
  dispatch.__closkellRefs = parentDispatch?.__closkellRefs;
  dispatch.__closkellDevtools = parentDispatch?.__closkellDevtools;
  return dispatch;
}

function scopedViewUpdateContext(updateContext, previousState, nextState) {
  if (!updateContext) return null;
  return {
    ...updateContext,
    localChangedPaths: [
      ...(updateContext.localChangedPaths || []),
      ...changedStatePaths(previousState, nextState)
    ],
    localReadPrefixes: [...(updateContext.localReadPrefixes || []), "state"],
    frames: updateContext.frames
  };
}

function mapScopedCommand(command, tag) {
  if (!command) return command;
  if (Array.isArray(command)) return command.map((item) => mapScopedCommand(item, tag));

  const kind = commandKind(command);
  if (kind === "none") return command;
  if (kind === "batch") {
    return {
      ...command,
      commands: (command.commands || []).map((item) => mapScopedCommand(item, tag))
    };
  }

  return mapScopedContinuations(command, tag);
}

function mapScopedSubscription(subscription, tag) {
  if (!subscription) return subscription;
  if (Array.isArray(subscription)) return subscription.map((item) => mapScopedSubscription(item, tag));

  const kind = subscriptionKind(subscription);
  if (kind === "none") return subscription;
  if (kind === "batch") {
    const subscriptions = subscription.subscriptions ?? subscription.subs ?? subscription.commands ?? [];
    return {
      ...subscription,
      subscriptions: subscriptions.map((item) => mapScopedSubscription(item, tag))
    };
  }

  return mapScopedContinuations(subscription, tag);
}

function mapScopedContinuations(effect, tag) {
  const mapped = { ...effect };
  if (effect.msg !== undefined) mapped.msg = wrapScopedMessage(effect.msg, tag);
  if (effect.toMessage !== undefined) {
    mapped.toMessage = (value) => wrapScopedMessage(effect.toMessage(value), tag);
  }
  mapScopedPayloadContinuation(mapped, effect, "onSuccess", tag, (value) => ({ value }));
  mapScopedPayloadContinuation(mapped, effect, "onError", tag, (error) => ({ error: errorMessage(error) }));
  mapScopedPayloadContinuation(mapped, effect, "onFrame", tag, (value) => value);
  mapScopedPayloadContinuation(mapped, effect, "onChange", tag, (value) => value);
  mapScopedPayloadContinuation(mapped, effect, "onReading", tag, (value) => value);
  mapScopedPayloadContinuation(mapped, effect, "onEvent", tag, (value) => value);
  mapScopedPayloadContinuation(mapped, effect, "onCancel", tag, () => ({}));
  mapScopedPayloadContinuation(mapped, effect, "onDisconnected", tag, () => ({}));
  return mapped;
}

function mapScopedPayloadContinuation(target, source, field, tag, payloadForValue) {
  if (source[field] === undefined) return;
  const continuation = source[field];
  target[field] = (value) => wrapScopedMessage(namedCommandMessage(continuation, payloadForValue(value)), tag);
}

function subscriptionKind(subscription) {
  return commandKind(subscription);
}

function subscriptionKey(subscription) {
  const kind = subscriptionKind(subscription);
  if (!kind || kind === "none" || kind === "batch") return "";
  const id =
    subscription.id ??
    subscription.ref ??
    subscription.query ??
    subscription.type ??
    subscription.event ??
    kind;
  return `${kind}:${subscriptionIdentityPart(id)}`;
}

function subscriptionIdentityPart(value) {
  if (typeof value === "symbol") return Symbol.keyFor(value) || value.description || String(value);
  if (value === null || value === undefined) return "";
  return String(value);
}

function subscriptionSignature(subscription) {
  const seen = new WeakSet();
  const normalize = (value) => {
    if (typeof value === "symbol") return `:${Symbol.keyFor(value) || value.description || String(value)}`;
    if (typeof value === "function") return "[Function]";
    if (value && typeof value === "object") {
      if (seen.has(value)) return "[Circular]";
      seen.add(value);
      if (Array.isArray(value)) return value.map(normalize);
      const sorted = {};
      for (const key of Object.keys(value).sort()) {
        sorted[key] = normalize(value[key]);
      }
      return sorted;
    }
    return value;
  };
  return JSON.stringify(normalize(subscription));
}

function compiledSubscriptionKey(subscription) {
  const kind = compiledCommandKind(subscription);
  if (!kind || kind === "none" || kind === "batch") return "";
  const id =
    subscription.id ??
    subscription.ref ??
    subscription.query ??
    subscription.type ??
    subscription.event ??
    kind;
  return `${kind}:${id == null ? "" : String(id)}`;
}

function compiledSubscriptionSignature(subscription) {
  return JSON.stringify(subscription);
}

function startCommandForSubscription(subscription) {
  const kind = subscriptionKind(subscription);
  switch (kind) {
    case "sub/timer/every":
      return { ...subscription, kind: Symbol.for("timer/every") };
    case "sub/dom-ref/resize":
      return { ...subscription, kind: Symbol.for("dom-ref/resize-watch") };
    case "sub/window/event":
      return { ...subscription, kind: Symbol.for("window/event-watch") };
    case "sub/media-query":
      return { ...subscription, kind: Symbol.for("media-query/watch") };
    case "sub/simulation/heart-rate":
      return { ...subscription, kind: Symbol.for("simulation/heart-rate") };
    case "sub/bluetooth/connect-heart-rate":
      return { ...subscription, kind: Symbol.for("bluetooth/connect-heart-rate") };
    default:
      return subscription;
  }
}

function compiledStartCommandForSubscription(subscription) {
  switch (compiledCommandKind(subscription)) {
    case "sub/timer/every":
      return { ...subscription, kind: "timer/every" };
    case "sub/dom-ref/resize":
      return { ...subscription, kind: "dom-ref/resize-watch" };
    case "sub/window/event":
      return { ...subscription, kind: "window/event-watch" };
    case "sub/media-query":
      return { ...subscription, kind: "media-query/watch" };
    case "sub/simulation/heart-rate":
      return { ...subscription, kind: "simulation/heart-rate" };
    case "sub/bluetooth/connect-heart-rate":
      return { ...subscription, kind: "bluetooth/connect-heart-rate" };
    default:
      return subscription;
  }
}

function stopCommandForSubscription(subscription) {
  const kind = subscriptionKind(subscription);
  switch (kind) {
    case "sub/timer/every":
      return { kind: Symbol.for("timer/cancel"), id: subscription.id };
    case "sub/dom-ref/resize":
      return { kind: Symbol.for("dom-ref/resize-unwatch"), id: subscription.id || subscription.ref };
    case "sub/window/event":
      return { kind: Symbol.for("window/event-unwatch"), id: subscription.id || subscription.type || subscription.event };
    case "sub/media-query":
      return { kind: Symbol.for("media-query/unwatch"), id: subscription.id || subscription.query };
    case "sub/simulation/heart-rate":
      return { kind: Symbol.for("simulation/stop"), id: subscription.id };
    case "sub/bluetooth/connect-heart-rate":
      return { kind: Symbol.for("bluetooth/disconnect"), id: subscription.id };
    default:
      return undefined;
  }
}

function compiledStopCommandForSubscription(subscription) {
  switch (compiledCommandKind(subscription)) {
    case "sub/timer/every":
      return { kind: "timer/cancel", id: subscription.id };
    case "sub/dom-ref/resize":
      return { kind: "dom-ref/resize-unwatch", id: subscription.id || subscription.ref };
    case "sub/window/event":
      return { kind: "window/event-unwatch", id: subscription.id || subscription.type || subscription.event };
    case "sub/media-query":
      return { kind: "media-query/unwatch", id: subscription.id || subscription.query };
    case "sub/simulation/heart-rate":
      return { kind: "simulation/stop", id: subscription.id };
    case "sub/bluetooth/connect-heart-rate":
      return { kind: "bluetooth/disconnect", id: subscription.id };
    default:
      return undefined;
  }
}

function commandMessage(command, value) {
  if (typeof command.toMessage === "function") {
    return command.toMessage(value);
  }
  if (command.msg !== undefined) {
    return command.msg;
  }
  if (command.onSuccess !== undefined) {
    if (typeof command.onSuccess === "function") return command.onSuccess(value);
    return { kind: command.onSuccess, value };
  }
  return undefined;
}

function compiledCommandMessage(command, value) {
  if (command.toMessage !== undefined) {
    return command.toMessage(value);
  }
  if (command.msg !== undefined) {
    return command.msg;
  }
  return undefined;
}

function namedCommandMessage(kind, fields = {}) {
  if (kind === undefined) return undefined;
  if (typeof kind === "function") return kind(fields);
  return { kind, ...fields };
}

function compiledNamedCommandMessage(kind, fields = {}) {
  if (kind === undefined) return undefined;
  return { kind, ...fields };
}

function commandErrorMessage(command, error) {
  if (command.onError !== undefined) {
    if (typeof command.onError === "function") return command.onError(errorMessage(error));
    return {
      kind: command.onError,
      error: errorMessage(error)
    };
  }
  throw error;
}

function compiledCommandErrorMessage(command, error) {
  if (command.onError !== undefined) {
    return {
      kind: command.onError,
      error: errorMessage(error)
    };
  }
  throw error;
}

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

function commandCancelMessage(command) {
  if (command.onCancel !== undefined) {
    if (typeof command.onCancel === "function") return command.onCancel();
    return { kind: command.onCancel };
  }
  return undefined;
}

function compiledCommandCancelMessage(command) {
  if (command.onCancel !== undefined) {
    return { kind: command.onCancel };
  }
  return undefined;
}

function commandValueName(value) {
  if (typeof value === "symbol") {
    return Symbol.keyFor(value) || value.description || "";
  }
  return String(value || "");
}

function nowMs() {
  return globalThis.performance?.now?.() ?? Date.now();
}

function headersToObject(headers) {
  const result = {};
  if (!headers?.forEach) return result;
  headers.forEach((value, key) => {
    result[String(key).toLowerCase()] = String(value);
  });
  return result;
}

function simulationHeartRateBpm(random, explicitStart, min, max, jitter, previous) {
  if (Number.isFinite(Number(explicitStart))) {
    return clampNumber(Math.round(Number(explicitStart)), min, max);
  }
  if (Number.isFinite(Number(previous)) && jitter != null) {
    const delta = Math.round((random() * 2 - 1) * jitter);
    return clampNumber(Math.round(Number(previous)) + delta, min, max);
  }
  return clampNumber(Math.round(min + random() * (max - min)), min, max);
}

function clampNumber(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

const HTTP_REQUEST_OPTION_FIELDS = [
  "method",
  "headers",
  "body",
  "mode",
  "credentials",
  "cache",
  "redirect",
  "referrer",
  "referrerPolicy",
  "integrity",
  "keepalive",
  "signal"
];

function httpRequestFetchArgs(command, env = {}) {
  const request = plainObject(command.request) ? command.request : {};
  const options = plainObject(command.options) ? command.options : {};
  const merged = { ...request, ...options };
  for (const field of HTTP_REQUEST_OPTION_FIELDS) {
    if (command[field] !== undefined) merged[field] = command[field];
  }
  if (merged.body !== undefined) {
    merged.body = resolveHttpRequestBody(merged.body, env);
  }
  const url = request.url ?? command.url;
  delete merged.url;
  return {
    url,
    options: Object.keys(merged).length ? merged : undefined
  };
}

function proxiedHttpUrl(url, command, host) {
  if (!command.proxy) return url;
  try {
    const location = host.location;
    const absolute = new URL(url, location?.href || "http://localhost/");
    const currentOrigin = location?.origin;
    if (currentOrigin && absolute.origin === currentOrigin) return url;
    return `/__proxy?url=${encodeURIComponent(absolute.href)}`;
  } catch {
    return url;
  }
}

async function httpResponsePayload(response, responseFormat, requestUrl, env = {}, host = globalThis) {
  if (responseFormat === "text") {
    return { body: await response.text() };
  }
  if (responseFormat === "blob" || responseFormat === "file") {
    const blob = await response.blob();
    const contentType = headerValue(response.headers, "content-type") || blob.type || null;
    const contentDisposition = headerValue(response.headers, "content-disposition");
    const fileName = resolveHttpFileName(contentDisposition, requestUrl, contentType);
    return {
      body: null,
      blob,
      fileName,
      contentDisposition,
      isFile: true,
      copyText: textMime(baseMime(contentType)) ? await blob.text() : "",
      contentType,
      previewUrl: await blobPreviewUrl(blob, contentType, env, host)
    };
  }
  if (responseFormat !== "auto") {
    return { body: await response.json() };
  }

  const headerContentType = headerValue(response.headers, "content-type");
  const contentDisposition = headerValue(response.headers, "content-disposition");
  const fileName = resolveHttpFileName(contentDisposition, requestUrl, headerContentType);

  if (isLikelyFileResponse(headerContentType, contentDisposition)) {
    const blob = await response.blob();
    const mime = baseMime(headerContentType) || blob.type || "";
    return {
      body: null,
      blob,
      fileName,
      contentDisposition,
      isFile: true,
      copyText: textMime(mime) || mime.includes("csv") ? await blob.text() : "",
      contentType: headerContentType,
      previewUrl: await blobPreviewUrl(blob, headerContentType, env, host)
    };
  }

  const raw = await response.text();
  const mime = baseMime(headerContentType);
  if (mime.includes("json") || mime.endsWith("+json")) {
    try {
      const parsed = JSON.parse(raw);
      return {
        body: parsed,
        fileName,
        contentDisposition,
        isFile: false,
        copyText: JSON.stringify(parsed, null, 2),
        contentType: headerContentType
      };
    } catch {
      if (looksLikeCsv(raw)) {
        return textFileResponse(raw, "text/csv", fileName, contentDisposition);
      }
      if (looksLikeBinary(raw)) {
        const BlobCtor = env.Blob || host.Blob || globalThis.Blob;
        const blob = new BlobCtor([raw], { type: "application/octet-stream" });
        return {
          body: null,
          blob,
          fileName,
          contentDisposition,
          isFile: true,
          copyText: "",
          contentType: "application/octet-stream"
        };
      }
      return {
        body: raw,
        fileName,
        contentDisposition,
        isFile: false,
        copyText: raw,
        contentType: "text/plain"
      };
    }
  }

  if (mime.includes("csv") || looksLikeCsv(raw)) {
    return textFileResponse(raw, "text/csv", fileName, contentDisposition);
  }

  return {
    body: raw,
    fileName,
    contentDisposition,
    isFile: false,
    copyText: raw,
    contentType: headerContentType
  };
}

function headerValue(headers, name) {
  if (!headers) return null;
  for (const [key, value] of headers.entries?.() || []) {
    if (String(key).toLowerCase() === name) return value;
  }
  return headers.get?.(name) ?? null;
}

function baseMime(contentType) {
  return String(contentType || "").split(";")[0].trim().toLowerCase();
}

function textMime(mime) {
  return String(mime || "").startsWith("text/");
}

function httpFileNameFromDisposition(header) {
  if (!String(header || "").trim()) return null;
  const value = String(header).trim();
  const star = value.match(/filename\*\s*=\s*(?:UTF-8''|utf-8'')([^;\n]+)/i);
  if (star?.[1]) {
    try {
      return decodeURIComponent(star[1].trim());
    } catch {
      return star[1].trim();
    }
  }
  const quoted = value.match(/filename\s*=\s*"([^"]*)"/i) || value.match(/filename\s*=\s*'([^']*)'/i);
  if (quoted?.[1]) return quoted[1];
  const plain = value.match(/filename\s*=\s*([^;\n]+)/i);
  return plain?.[1]?.trim().replace(/^["']|["']$/g, "") || null;
}

function httpExtensionFromContentType(contentType) {
  const mime = baseMime(contentType);
  if (mime.includes("csv")) return "csv";
  if (mime === "application/pdf") return "pdf";
  if (mime.includes("json")) return "json";
  if (mime.includes("xml")) return "xml";
  if (mime.startsWith("image/")) return mime.split("/")[1] || "bin";
  if (mime.includes("zip")) return "zip";
  if (mime.includes("excel") || mime.includes("spreadsheet")) return "xlsx";
  if (mime.includes("word") || mime.includes("msword")) return "docx";
  return "bin";
}

function inferHttpFileNameFromUrl(url, contentType) {
  try {
    const parts = new URL(url).pathname.split("/").filter(Boolean);
    if (!parts.length) return null;
    const last = parts[parts.length - 1];
    const lastLower = last.toLowerCase();
    const ext = httpExtensionFromContentType(contentType);
    if (lastLower.includes(".")) return last;
    if (lastLower === "csv" && parts.length >= 2) return `${parts[parts.length - 2]}.csv`;
    if (lastLower === "csv") return "download.csv";
    if (ext !== "bin") return `${last}.${ext}`;
    return null;
  } catch {
    return null;
  }
}

function resolveHttpFileName(contentDisposition, requestUrl, contentType) {
  return httpFileNameFromDisposition(contentDisposition) || (requestUrl ? inferHttpFileNameFromUrl(requestUrl, contentType) : null);
}

function looksLikeCsv(text) {
  const sample = String(text || "").trim().slice(0, 4096);
  if (!sample) return false;
  const lines = sample.split(/\r?\n/).filter(Boolean).slice(0, 5);
  if (!lines.length) return false;
  return lines.every((line) => /"[^"]*"[;,]/.test(line) || /^[^;\n]+;[^;\n]+/.test(line));
}

function looksLikeBinary(text) {
  return /[\x00-\x08\x0e-\x1f]/.test(String(text || "").slice(0, 512));
}

function isLikelyFileResponse(contentType, disposition) {
  const mime = baseMime(contentType);
  if (/attachment/i.test(String(disposition || ""))) return true;
  if (!mime) return false;
  if (mime === "application/json" || mime === "application/problem+json" || mime.endsWith("+json") || mime === "application/xml" || mime === "text/xml") {
    return false;
  }
  if (mime.startsWith("text/") && !mime.includes("csv")) return false;
  return mime === "application/octet-stream" ||
    mime.includes("csv") ||
    mime.startsWith("image/") ||
    mime.startsWith("audio/") ||
    mime.startsWith("video/") ||
    mime === "application/pdf" ||
    mime.includes("zip") ||
    mime.includes("excel") ||
    mime.includes("spreadsheet") ||
    mime.includes("msword") ||
    mime.includes("officedocument");
}

function textFileResponse(raw, mime, fileName, contentDisposition) {
  const BlobCtor = globalThis.Blob;
  return {
    body: null,
    blob: typeof BlobCtor === "function" ? new BlobCtor([raw], { type: `${mime};charset=utf-8` }) : undefined,
    fileName,
    contentDisposition,
    isFile: true,
    copyText: raw,
    contentType: mime
  };
}

function blobPreviewUrl(blob, contentType, env = {}, host = globalThis) {
  const mime = baseMime(contentType || blob?.type);
  if (!mime.startsWith("image/")) return "";
  const FileReaderCtor = env.FileReader || host.FileReader || globalThis.FileReader;
  if (typeof FileReaderCtor === "function") {
    return new Promise((resolve) => {
      const reader = new FileReaderCtor();
      reader.onerror = () => resolve("");
      reader.onload = () => resolve(typeof reader.result === "string" ? reader.result : "");
      reader.readAsDataURL(blob);
    });
  }
  const URLRef = env.URL || host.URL || globalThis.URL;
  return URLRef?.createObjectURL ? URLRef.createObjectURL(blob) : "";
}

function resolveHttpRequestBody(body, env = {}) {
  const kind = commandValueName(body?.kind);
  if (kind === "browser/selected-file") {
    return selectedFileByTestId(env.document, body.testId);
  }
  if (kind === "browser/multipart-form") {
    return multipartFormBody(env, body.fields, body.values);
  }
  return body;
}

function selectedFileByTestId(documentRef, testId) {
  const selector = `[data-testid="${cssAttr(String(testId ?? ""))}"]`;
  const input = documentRef?.querySelector?.(selector);
  return input?.files?.[0] ?? null;
}

function multipartFormBody(env, fields, values) {
  const FormDataCtor = env.FormData || globalThis.FormData;
  if (typeof FormDataCtor !== "function") {
    throw new Error("FormData is not available for multipart http/request body");
  }

  const form = new FormDataCtor();
  for (const field of Array.isArray(fields) ? fields : []) {
    const name = String(field?.name ?? "");
    if (!name) continue;

    if (field.kind === "file") {
      const file = selectedFileByTestId(env.document, `request-body-multipart-${name}`);
      if (file) form.append(name, file, file.name);
    } else {
      const value = values && hasOwn(values, name) ? values[name] : "";
      const text = String(value ?? "").trim();
      if (text) form.append(name, text);
    }
  }
  return form;
}

function cssAttr(value) {
  return String(value).replace(/\\/g, "\\\\").replace(/"/g, "\\\"");
}

function primitiveDecoder(name, predicate) {
  return {
    decode(value, path = "value") {
      return predicate(value) ? { ok: true, value } : decoderTypeError(path, name);
    }
  };
}

function runDecoder(decoder, value, path = "value") {
  if (!decoder || typeof decoder.decode !== "function") {
    return { ok: false, error: `${path} expected Decoder` };
  }
  try {
    const result = decoder.decode(value, path);
    if (result?.ok === true) return { ok: true, value: result.value };
    return { ok: false, error: String(result?.error ?? `${path} did not match decoder`) };
  } catch (error) {
    return { ok: false, error: error?.message ?? String(error) };
  }
}

function decoderSpecEntries(spec) {
  if (spec instanceof Map) {
    return Array.from(spec.entries()).map(([key, value]) => [decoderFieldName(key), value]);
  }
  if (plainObject(spec)) return Object.entries(spec);
  return [];
}

function decoderFieldName(key) {
  if (typeof key === "symbol") return Symbol.keyFor(key) || key.description || String(key);
  return String(key);
}

function decoderFieldPath(path, field) {
  return /^[A-Za-z_$][\w$]*$/.test(field) ? `${path}.${field}` : `${path}[${JSON.stringify(field)}]`;
}

function decoderTypeError(path, expected) {
  return { ok: false, error: `${path} expected ${expected}` };
}

function decoderValueEqual(left, right) {
  if (typeof left === "symbol" || typeof right === "symbol") {
    return typeof left === "symbol"
      && typeof right === "symbol"
      && (Symbol.keyFor(left) || left.description) === (Symbol.keyFor(right) || right.description);
  }
  return Object.is(left, right);
}

function decoderFormatValue(value) {
  if (typeof value === "symbol") return `:${Symbol.keyFor(value) || value.description || String(value)}`;
  if (value === null) return "nil";
  if (typeof value === "string") return JSON.stringify(value);
  return String(value);
}

function plainObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function commandOptions(value) {
  return (
    plainObject(value) &&
    (hasOwn(value, "msg") ||
      hasOwn(value, "onSuccess") ||
      hasOwn(value, "toMessage") ||
      hasOwn(value, "onError"))
  );
}

function hasOwn(value, key) {
  return Object.prototype.hasOwnProperty.call(value, key);
}

function resolveRef(value, dispatch) {
  const name = refName(value);
  if (!name) return null;
  return dispatch?.__closkellRefs?.get(name) || null;
}

function compiledRefName(value) {
  if (value === false || value == null) return null;
  return String(value);
}

function resolveCompiledRef(value, dispatch) {
  const name = compiledRefName(value);
  if (!name) return null;
  return dispatch?.__closkellRefs?.get(name) || null;
}

function measureNode(node) {
  const rect = node.getBoundingClientRect?.();
  if (rect) {
    return {
      x: numberOrZero(rect.x),
      y: numberOrZero(rect.y),
      width: numberOrZero(rect.width),
      height: numberOrZero(rect.height),
      top: numberOrZero(rect.top),
      right: numberOrZero(rect.right),
      bottom: numberOrZero(rect.bottom),
      left: numberOrZero(rect.left)
    };
  }

  const width = numberOrZero(node.clientWidth ?? node.offsetWidth ?? node.width);
  const height = numberOrZero(node.clientHeight ?? node.offsetHeight ?? node.height);
  return { x: 0, y: 0, width, height, top: 0, right: width, bottom: height, left: 0 };
}

function numberOrZero(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : 0;
}

function numberOr(value, fallback) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}

function rectFromResizeEntry(entry, node) {
  const measured = measureNode(node);
  const rect = entry?.contentRect;
  if (!rect) return measured;

  const x = numberOr(rect.x, measured.x);
  const y = numberOr(rect.y, measured.y);
  const width = numberOr(rect.width, measured.width);
  const height = numberOr(rect.height, measured.height);
  const top = numberOr(rect.top, y);
  const left = numberOr(rect.left, x);
  return {
    x,
    y,
    width,
    height,
    top,
    right: numberOr(rect.right, left + width),
    bottom: numberOr(rect.bottom, top + height),
    left
  };
}

function resizeMessage(command, id, node, rect) {
  return namedCommandMessage(command.onChange, {
    id,
    ref: refName(command.ref),
    value: rect,
    ...rect
  });
}

function compiledResizeMessage(command, id, node, rect) {
  return compiledNamedCommandMessage(command.onChange, {
    id,
    ref: compiledRefName(command.ref),
    value: rect,
    ...rect
  });
}

function windowEventMessage(command, event, id, host = globalThis) {
  const payload = windowEventPayload(event, host);
  return namedCommandMessage(command.onEvent, {
    id,
    ...payload,
    value: payload
  });
}

function compiledWindowEventMessage(command, event, id) {
  const payload = windowEventPayload(event);
  return compiledNamedCommandMessage(command.onEvent, {
    id,
    ...payload,
    value: payload
  });
}

function applyWindowEventControls(command, event) {
  if (eventControlMatches(command.preventDefault, event)) event?.preventDefault?.();
  if (eventControlMatches(command.stopPropagation, event)) event?.stopPropagation?.();
}

function applyCompiledWindowEventControls(command, event) {
  if (compiledEventControlMatches(command.preventDefault, event)) event?.preventDefault?.();
  if (compiledEventControlMatches(command.stopPropagation, event)) event?.stopPropagation?.();
}

function eventControlMatches(rule, event = {}) {
  if (rule === true) return true;
  if (!rule || typeof rule !== "object") return false;

  if (rule.type != null && String(event.type || "") !== String(rule.type)) return false;
  if (rule.key != null && String(event.key || "").toLowerCase() !== String(rule.key).toLowerCase()) return false;
  if (rule.code != null && String(event.code || "") !== String(rule.code)) return false;

  for (const name of ["altKey", "ctrlKey", "metaKey", "shiftKey"]) {
    if (Object.prototype.hasOwnProperty.call(rule, name) && Boolean(event[name]) !== Boolean(rule[name])) return false;
  }

  return true;
}

function compiledEventControlMatches(rule, event = {}) {
  if (rule === true) return true;
  if (!rule) return false;

  if (rule.type != null && String(event.type || "") !== String(rule.type)) return false;
  if (rule.key != null && String(event.key || "").toLowerCase() !== String(rule.key).toLowerCase()) return false;
  if (rule.code != null && String(event.code || "") !== String(rule.code)) return false;

  for (const name of ["altKey", "ctrlKey", "metaKey", "shiftKey"]) {
    if (Object.prototype.hasOwnProperty.call(rule, name) && Boolean(event[name]) !== Boolean(rule[name])) return false;
  }

  return true;
}

function windowEventPayload(event = {}, host = globalThis) {
  const href = String(host.location?.href || "");
  const path = String(host.location?.pathname || "");
  const search = String(host.location?.search || "");
  return {
    type: String(event.type || ""),
    href,
    path,
    search,
    clientX: event.clientX || 0,
    clientY: event.clientY || 0,
    pageX: event.pageX || 0,
    pageY: event.pageY || 0,
    screenX: event.screenX || 0,
    screenY: event.screenY || 0,
    movementX: event.movementX || 0,
    movementY: event.movementY || 0,
    button: event.button || 0,
    buttons: event.buttons || 0,
    pointerId: event.pointerId || 0,
    pointerType: event.pointerType == null ? "" : String(event.pointerType),
    isPrimary: Boolean(event.isPrimary),
    key: event.key == null ? "" : String(event.key),
    code: event.code == null ? "" : String(event.code),
    altKey: Boolean(event.altKey),
    ctrlKey: Boolean(event.ctrlKey),
    metaKey: Boolean(event.metaKey),
    shiftKey: Boolean(event.shiftKey)
  };
}

function queueScrollIntoView(command, env = {}) {
  const schedule = env.requestAnimationFrame || env.host?.requestAnimationFrame?.bind(env.host);
  const fallback = env.host?.setTimeout?.bind(env.host) || ((fn) => fn());
  const run = () => {
    const node = scrollTargetNode(command, env.document);
    if (!node?.scrollIntoView) return;
    if (command.skipIfVisible && nodeFullyVisible(node, env.host)) return;
    const behavior = command.behavior || (command.smooth ? "smooth" : "auto");
    node.scrollIntoView({
      behavior,
      block: command.block || "start",
      inline: command.inline || "nearest"
    });
  };
  if (schedule) {
    schedule(() => schedule(run));
  } else {
    fallback(run, 0);
  }
}

function scrollTargetNode(command, documentRef) {
  if (!documentRef?.querySelector) return null;
  if (command.selector) return documentRef.querySelector(String(command.selector));
  if (command.testId) return documentRef.querySelector(`[data-testid="${cssAttr(String(command.testId))}"]`);
  if (command.id) return documentRef.getElementById?.(String(command.id)) || null;
  return null;
}

function nodeFullyVisible(node, host = globalThis) {
  const rect = node.getBoundingClientRect?.();
  if (!rect) return false;
  const height = host.innerHeight || 0;
  const width = host.innerWidth || 0;
  return rect.top >= 0 && rect.left >= 0 && rect.bottom <= height && rect.right <= width;
}

function animationFrameMessage(command, id, timestamp) {
  const time = numberOrZero(timestamp);
  return namedCommandMessage(command.onFrame, {
    id,
    timestamp: time,
    value: time
  });
}

function compiledAnimationFrameMessage(command, id, timestamp) {
  const time = numberOrZero(timestamp);
  return compiledNamedCommandMessage(command.onFrame, {
    id,
    timestamp: time,
    value: time
  });
}

function canvasDrawSizing(command, canvas, env, host) {
  const pixelRatio = canvasPixelRatio(command, env, host);
  const cssWidth = numberOrUndefined(command.cssWidth ?? command.width);
  const cssHeight = numberOrUndefined(command.cssHeight ?? command.height);

  if (pixelRatio !== 1 || command.cssWidth !== undefined || command.cssHeight !== undefined) {
    return {
      cssWidth,
      cssHeight,
      width: cssWidth === undefined ? undefined : Math.round(cssWidth * pixelRatio),
      height: cssHeight === undefined ? undefined : Math.round(cssHeight * pixelRatio),
      pixelRatio
    };
  }

  return {
    cssWidth: command.width === undefined ? undefined : numberOrUndefined(command.width),
    cssHeight: command.height === undefined ? undefined : numberOrUndefined(command.height),
    width: command.width,
    height: command.height,
    pixelRatio
  };
}

function canvasPixelRatio(command, env, host) {
  const requested = command.pixelRatio ?? command.devicePixelRatio;
  if (requested === undefined || requested === false || requested == null) return 1;

  const requestedName = commandValueName(requested);
  if (requested === true || requestedName === "device") {
    return Math.max(1, numberOrZero(env.devicePixelRatio ?? host.devicePixelRatio ?? 1));
  }

  const ratio = Number(requested);
  return Number.isFinite(ratio) && ratio > 0 ? ratio : 1;
}

function numberOrUndefined(value) {
  if (value === undefined || value === null) return undefined;
  const number = Number(value);
  return Number.isFinite(number) ? number : undefined;
}

function setCanvasCssSize(canvas, name, value) {
  if (!canvas.style) canvas.style = {};
  canvas.style[name] = `${value}px`;
}

function setCanvasTransform(ctx, pixelRatio) {
  if (ctx.setTransform) {
    ctx.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
  } else if (ctx.resetTransform || ctx.scale) {
    ctx.resetTransform?.();
    if (ctx.scale && pixelRatio !== 1) ctx.scale(pixelRatio, pixelRatio);
  }
}

function eventListenerOptions(options) {
  if (options == null || typeof options !== "object") return options;
  return {
    capture: Boolean(options.capture),
    once: Boolean(options.once),
    passive: Boolean(options.passive)
  };
}

function mediaQueryMessage(command, mediaQuery, id) {
  return namedCommandMessage(command.onChange, {
    id,
    media: mediaQuery.media || command.query || "",
    matches: Boolean(mediaQuery.matches)
  });
}

function compiledMediaQueryMessage(command, mediaQuery, id) {
  return compiledNamedCommandMessage(command.onChange, {
    id,
    media: mediaQuery.media,
    matches: Boolean(mediaQuery.matches)
  });
}

function addMediaQueryListener(mediaQuery, listener) {
  if (mediaQuery.addEventListener) {
    mediaQuery.addEventListener("change", listener);
    return;
  }
  mediaQuery.addListener?.(listener);
}

function removeMediaQueryListener(entry) {
  const { mediaQuery, listener } = entry;
  if (mediaQuery.removeEventListener) {
    mediaQuery.removeEventListener("change", listener);
    return;
  }
  mediaQuery.removeListener?.(listener);
}

function removeResizeObserver(entry) {
  if (entry.observer?.disconnect) {
    entry.observer.disconnect();
  } else if (entry.observer?.unobserve && entry.node) {
    entry.observer.unobserve(entry.node);
  }

  if (entry.target?.removeEventListener && entry.listener) {
    entry.target.removeEventListener("resize", entry.listener);
  }
}

function removeWindowEventListener(entry) {
  entry.target?.removeEventListener?.(entry.type, entry.listener, entry.options);
}

function cancelAnimationFrameEntry(entry) {
  if (entry.cancel) {
    entry.cancel(entry.handle);
  }
}

function canvasMeasureTexts(command) {
  if (Array.isArray(command.texts)) return command.texts.map((text) => String(text ?? ""));
  if (command.texts !== undefined) return [String(command.texts ?? "")];
  return [String(command.text ?? "")];
}

function applyCanvasOp(ctx, canvas, op) {
  const name = commandValueName(op.op || op.kind);
  switch (name) {
    case "clear":
      ctx.clearRect(op.x ?? 0, op.y ?? 0, op.width ?? canvas.width ?? 0, op.height ?? canvas.height ?? 0);
      break;
    case "fill-rect":
      applyCanvasState(ctx, op, "fill");
      ctx.fillRect(op.x ?? 0, op.y ?? 0, op.width ?? 0, op.height ?? 0);
      break;
    case "stroke-rect":
      applyCanvasState(ctx, op, "stroke");
      ctx.strokeRect(op.x ?? 0, op.y ?? 0, op.width ?? 0, op.height ?? 0);
      break;
    case "begin-path":
      ctx.beginPath();
      break;
    case "move-to":
      ctx.moveTo(op.x ?? 0, op.y ?? 0);
      break;
    case "line-to":
      ctx.lineTo(op.x ?? 0, op.y ?? 0);
      break;
    case "arc":
      ctx.arc(op.x ?? 0, op.y ?? 0, op.radius ?? 0, op.start ?? 0, op.end ?? Math.PI * 2);
      break;
    case "stroke":
      applyCanvasState(ctx, op, "stroke");
      ctx.stroke();
      break;
    case "fill":
      applyCanvasState(ctx, op, "fill");
      ctx.fill();
      break;
    case "fill-text":
      applyCanvasState(ctx, op, "fill");
      ctx.fillText(String(op.text ?? ""), op.x ?? 0, op.y ?? 0);
      break;
    case "set":
      if (op.name) ctx[op.name] = op.value;
      break;
    default:
      throw new Error(`Unknown canvas op ${name}`);
  }
}

function applyCompiledCanvasOp(ctx, canvas, op) {
  switch (op.op || op.kind) {
    case "clear":
      ctx.clearRect(op.x ?? 0, op.y ?? 0, op.width ?? canvas.width ?? 0, op.height ?? canvas.height ?? 0);
      break;
    case "fill-rect":
      applyCompiledCanvasState(ctx, op, "fill");
      ctx.fillRect(op.x ?? 0, op.y ?? 0, op.width ?? 0, op.height ?? 0);
      break;
    case "stroke-rect":
      applyCompiledCanvasState(ctx, op, "stroke");
      ctx.strokeRect(op.x ?? 0, op.y ?? 0, op.width ?? 0, op.height ?? 0);
      break;
    case "begin-path":
      ctx.beginPath();
      break;
    case "move-to":
      ctx.moveTo(op.x ?? 0, op.y ?? 0);
      break;
    case "line-to":
      ctx.lineTo(op.x ?? 0, op.y ?? 0);
      break;
    case "arc":
      ctx.arc(op.x ?? 0, op.y ?? 0, op.radius ?? 0, op.start ?? 0, op.end ?? Math.PI * 2);
      break;
    case "stroke":
      applyCompiledCanvasState(ctx, op, "stroke");
      ctx.stroke();
      break;
    case "fill":
      applyCompiledCanvasState(ctx, op, "fill");
      ctx.fill();
      break;
    case "fill-text":
      applyCompiledCanvasState(ctx, op, "fill");
      ctx.fillText(String(op.text ?? ""), op.x ?? 0, op.y ?? 0);
      break;
    case "set":
      if (op.name) ctx[op.name] = op.value;
      break;
    default:
      throw new Error(`Bad canvas op ${op.op || op.kind}`);
  }
}

function applyCanvasState(ctx, op, paintMode) {
  if (paintMode !== "stroke" && (op.fillStyle !== undefined || op.color !== undefined)) {
    ctx.fillStyle = op.fillStyle ?? op.color;
  }
  if (paintMode !== "fill" && (op.strokeStyle !== undefined || op.color !== undefined)) {
    ctx.strokeStyle = op.strokeStyle ?? op.color;
  }
  if (op.lineWidth !== undefined) ctx.lineWidth = op.lineWidth;
  if (op.lineCap !== undefined) ctx.lineCap = op.lineCap;
  if (op.lineJoin !== undefined) ctx.lineJoin = op.lineJoin;
  if (op.globalAlpha !== undefined) ctx.globalAlpha = op.globalAlpha;
  if (op.font !== undefined) ctx.font = op.font;
  const textAlign = canvasTextStateValue(op.textAlign);
  const textBaseline = canvasTextStateValue(op.textBaseline);
  if (textAlign !== undefined) ctx.textAlign = textAlign;
  if (textBaseline !== undefined) ctx.textBaseline = textBaseline;
}

function applyCompiledCanvasState(ctx, op, paintMode) {
  if (paintMode !== "stroke" && (op.fillStyle !== undefined || op.color !== undefined)) {
    ctx.fillStyle = op.fillStyle ?? op.color;
  }
  if (paintMode !== "fill" && (op.strokeStyle !== undefined || op.color !== undefined)) {
    ctx.strokeStyle = op.strokeStyle ?? op.color;
  }
  if (op.lineWidth !== undefined) ctx.lineWidth = op.lineWidth;
  if (op.lineCap !== undefined) ctx.lineCap = op.lineCap;
  if (op.lineJoin !== undefined) ctx.lineJoin = op.lineJoin;
  if (op.globalAlpha !== undefined) ctx.globalAlpha = op.globalAlpha;
  if (op.font !== undefined) ctx.font = op.font;
  if (op.textAlign !== undefined) ctx.textAlign = String(op.textAlign || "");
  if (op.textBaseline !== undefined) ctx.textBaseline = String(op.textBaseline || "");
}

function canvasTextStateValue(value) {
  if (value === undefined || value === null || value === false) return undefined;
  const name = commandValueName(value);
  return name === "" ? undefined : name;
}

function bluetoothRequestOptions(command) {
  if (command.options && typeof command.options === "object") return command.options;

  const options = {};
  if (command.filters !== undefined) options.filters = command.filters;
  if (command.optionalServices !== undefined) options.optionalServices = command.optionalServices;
  if (command.acceptAllDevices !== undefined) options.acceptAllDevices = command.acceptAllDevices;
  return options;
}

function compiledBluetoothRequestOptions(command) {
  if (command.options) return command.options;

  const options = {};
  if (command.filters !== undefined) options.filters = command.filters;
  if (command.optionalServices !== undefined) options.optionalServices = command.optionalServices;
  if (command.acceptAllDevices !== undefined) options.acceptAllDevices = command.acceptAllDevices;
  return options;
}

function parseHeartRateMeasurement(dataView) {
  const flags = dataView.getUint8(0);
  const isUint16 = Boolean(flags & 0x01);
  return isUint16 ? dataView.getUint16(1, true) : dataView.getUint8(1);
}

function serializeStoredValue(value) {
  return typeof value === "string" ? value : JSON.stringify(value);
}

function parseStoredValue(value, format) {
  const mode = format == null ? "auto" : commandValueName(format);
  if (mode === "json") return JSON.parse(value);
  if (mode === "text" || mode === "string" || mode === "raw") return value;
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function parseCompiledStoredValue(value, format) {
  const mode = format == null ? "auto" : format;
  if (mode === "json") return JSON.parse(value);
  if (mode === "text" || mode === "string" || mode === "raw") return value;
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function replaceBrowserSearchParam(host, name, value) {
  try {
    const next = new URL(host.location?.href ?? "http://localhost/");
    if (value === null || value === undefined || String(value) === "") next.searchParams.delete(String(name));
    else next.searchParams.set(String(name), String(value));
    host.history?.replaceState?.(null, "", `${next.pathname}${next.search}${next.hash}`);
  } catch {}
}

function writeBrowserRoute(host, url, op, definition) {
  try {
    const params = new URLSearchParams();
    if (url !== null && url !== undefined && String(url) !== "") params.set("url", String(url));
    if (definition !== null && definition !== undefined && String(definition) !== "") {
      params.set("definition", String(definition));
    }
    if (op !== null && op !== undefined && String(op) !== "") params.set("op", String(op));
    const query = params.toString();
    const pathname = host.location?.pathname ?? "/";
    const next = query ? `${pathname}?${query}` : pathname;
    if (next !== `${pathname}${host.location?.search ?? ""}`) host.history?.replaceState?.(null, "", next);
  } catch {}
}

function loadBrowserTheme(host, storage, sessionStorage, key) {
  let stored = null;
  try {
    stored = sessionStorage?.getItem(String(key)) ?? storage?.getItem(String(key));
  } catch {}
  const theme = stored === "light" ? "light" : "dark";
  host.document?.documentElement?.classList?.toggle("dark", theme === "dark");
  return theme;
}

function applyBrowserTheme(host, storage, sessionStorage, key, theme) {
  const nextTheme = theme === "light" ? "light" : "dark";
  host.document?.documentElement?.classList?.toggle("dark", nextTheme === "dark");
  try {
    sessionStorage?.setItem(String(key), nextTheme);
    storage?.setItem(String(key), nextTheme);
  } catch {}
}

function setBrowserCookie(host, name, value) {
  const documentRef = host.document;
  if (!documentRef) return;
  documentRef.cookie = `${encodeURIComponent(String(name ?? ""))}=${encodeURIComponent(String(value ?? ""))}; path=/`;
}

function persistAuthStorage(storage, sourceUrl, entries) {
  const key = `better-swagger-auth:${String(sourceUrl ?? "")}`;
  try {
    storage?.setItem(key, JSON.stringify(Object.values(entries ?? {})));
  } catch {}
}

function loadAuthStorage(storage, sessionStorage, sourceUrl) {
  const key = `better-swagger-auth:${String(sourceUrl ?? "")}`;
  let raw = null;
  try {
    raw = storage?.getItem(key) ?? sessionStorage?.getItem(key);
  } catch {}
  if (!raw) return {};

  try {
    const parsed = JSON.parse(raw);
    const now = Date.now();
    const original = Array.isArray(parsed) ? parsed : [];
    const valid = original.filter((entry) => !entry?.expiresAt || entry.expiresAt > now);
    const entries = Object.fromEntries(valid.map((entry) => [entry.schemeId, entry]));
    if (valid.length !== original.length || sessionStorage?.getItem(key)) {
      storage?.setItem(key, JSON.stringify(Object.values(entries)));
      sessionStorage?.removeItem(key);
    }
    return entries;
  } catch {
    return {};
  }
}

function downloadWithBrowser(payload, env, host) {
  const documentRef = env.document || host.document;
  const URLRef = env.URL || host.URL;
  const BlobCtor = env.Blob || host.Blob;

  if (!documentRef || !URLRef?.createObjectURL || !BlobCtor) {
    throw new Error("No browser download implementation is available for file/download");
  }

  const blob = payload.blob || new BlobCtor([payload.content], { type: payload.mime });
  const href = URLRef.createObjectURL(blob);
  const link = documentRef.createElement("a");
  link.href = href;
  link.download = payload.name;
  link.style ||= {};
  link.style.display = "none";

  const body = documentRef.body;
  if (body?.appendChild) body.appendChild(link);
  link.click();
  if (link.parentNode?.removeChild) link.parentNode.removeChild(link);
  URLRef.revokeObjectURL?.(href);

  return { ...payload, href, size: blob.size };
}

function importWithBrowser(payload, env, host) {
  const documentRef = env.document || host.document;
  if (!documentRef?.createElement) {
    throw new Error("No browser file input implementation is available for file/import");
  }

  return new Promise((resolve, reject) => {
    const input = documentRef.createElement("input");
    input.type = "file";
    input.accept = payload.accept || "";
    input.multiple = Boolean(payload.multiple);
    input.style ||= {};
    input.style.display = "none";

    const cleanup = () => {
      if (input.parentNode?.removeChild) input.parentNode.removeChild(input);
    };

    input.addEventListener("change", async () => {
      try {
        const files = Array.from(input.files || []);
        if (!files.length) {
          cleanup();
          resolve(undefined);
          return;
        }

        const imported = payload.multiple
          ? await Promise.all(files.map((file) => readImportedFile(file, payload.format)))
          : await readImportedFile(files[0], payload.format);
        cleanup();
        resolve(imported);
      } catch (error) {
        cleanup();
        reject(error);
      }
    });

    const body = documentRef.body;
    if (body?.appendChild) body.appendChild(input);
    input.click();
  });
}

async function readImportedFile(file, format) {
  const text = await file.text();
  if (format === "json") return JSON.parse(text);
  if (format === "record") {
    return { name: file.name, type: file.type, text };
  }
  return text;
}

function clearFileInput(input) {
  try {
    input.value = "";
  } catch {
    // Some host shims or browsers may reject file value assignment.
  }
  try {
    if (Array.isArray(input.files)) input.files = [];
  } catch {
    // Browser FileList is read-only; clearing value is the portable path.
  }
}
