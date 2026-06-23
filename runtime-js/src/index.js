export function htmlTemplate(source) {
  const template = document.createElement("template");
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
      instance.definition = definition;
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
    for (const slot of current.keyedSlots || []) {
      for (const entry of slot?.byKey?.values?.() || []) disposeComponent(entry.component);
    }
    for (const slot of current.conditionalSlots || []) disposeComponent(slot?.component);
    for (const slot of current.componentSlots || []) disposeComponent(slot?.component);
    if (current.root?.parentNode?.removeChild) current.root.parentNode.removeChild(current.root);
    current.mounted = false;
  };

  return {
    definition,
    mount(parent, dispatch = lastDispatch) {
      lastDispatch = dispatch || lastDispatch;
      const current = ensureInstance();
      reportTemplateMount(current, lastDispatch, definition);
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
    }
  };
}

export function shouldUpdateSlot(instance, slot, updateContext) {
  const slotMetadata = instance.definition?.slots?.[slot] || { id: slot, reads: [] };
  const shouldUpdate = shouldUpdateSlotForReads(slotMetadata.reads || [], updateContext);
  recordTemplateSlot(updateContext, slotMetadata, shouldUpdate);
  return shouldUpdate;
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
  if (typeof value === "symbol") return Symbol.keyFor(value) ?? value.description ?? "";
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
  if (typeof name === "symbol") return Symbol.keyFor(name) ?? name.description ?? "";
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
      current.dispatch(current.messageForEvent(event), event);
    };
    node.addEventListener(eventName, current.listener);
  }

  instance.eventSlots[slot] = current;
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

export function setKeyedList(instance, slot, marker, items, keyForItem, renderItem, dispatch, updateContext) {
  const parent = marker.parentNode;
  if (!parent) return;

  const current = instance.keyedSlots[slot] || { byKey: new Map(), duplicateKeys: new Map() };
  if (!current.duplicateKeys) current.duplicateKeys = new Map();
  const slotMetadata = instance.definition?.slots?.[slot] || {};
  const keyedKind = slotMetadata.kind || {};
  const itemName = typeof keyedKind.keyed === "string" ? keyedKind.keyed : null;
  const indexName = typeof keyedKind.index === "string" ? keyedKind.index : null;
  const nextByKey = new Map();
  const nextDuplicateKeys = new Map();
  const seenKeys = new Map();
  const orderedEntries = [];

  let index = 0;
  for (const item of items || []) {
    const rawKey = keyForItem(item, index);
    const occurrence = seenKeys.get(rawKey) || 0;
    seenKeys.set(rawKey, occurrence + 1);
    const key = occurrence === 0 ? rawKey : duplicateStorageKey(current, nextDuplicateKeys, rawKey, occurrence);
    let entry = current.byKey.get(key);
    if (!entry) {
      const component = renderItem(item, index);
      updateKeyedComponent(component, item, index, dispatch, forceUpdateContext(updateContext));
      entry = { key, component, item, index };
    } else {
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

  let cursor = marker;
  for (let i = orderedEntries.length - 1; i >= 0; i -= 1) {
    const root = orderedEntries[i].component.root;
    if (root.parentNode !== parent || root.nextSibling !== cursor) {
      parent.insertBefore(root, cursor);
    }
    cursor = root;
  }

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

export function setComponent(instance, slot, marker, render, args, dispatch, updateContext) {
  const parent = marker.parentNode;
  if (!parent) return;

  const current = instance.componentSlots[slot] || {};
  const rendered = typeof render === "function" ? render() : render;
  const renderedKey = componentRenderKey(rendered);
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

function componentRenderKey(component) {
  return component?.definition?.name || null;
}

function componentParams(component) {
  const params = component?.definition?.params;
  return Array.isArray(params) ? params : [];
}

function disposeComponent(component) {
  if (component?.dispose) {
    component.dispose();
  }
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

function disposeEventSlots(instance) {
  for (const current of instance.eventSlots || []) {
    current?.node?.removeEventListener?.(current.eventName, current.listener);
  }
  instance.eventSlots = [];
}

function disposeRefs(instance) {
  for (const current of instance.refSlots || []) {
    unregisterRef(current?.registry, current?.name, current?.node);
  }
  instance.refSlots = [];
}

function registryForDispatch(dispatch) {
  if (!dispatch || (typeof dispatch !== "function" && typeof dispatch !== "object")) return new Map();
  if (!dispatch.__closkellRefs) dispatch.__closkellRefs = new Map();
  return dispatch.__closkellRefs;
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

export function createCommandHandlers(env = {}) {
  const host = env.host || globalThis;
  const timers = env.timers || host;
  const bluetooth = env.bluetooth || host.navigator?.bluetooth;
  const storage = env.storage || host.localStorage;
  const fetchImpl = env.fetch || host.fetch?.bind(host);
  const random = env.random || Math.random;
  const now = env.now || (() => Date.now());
  const animation = env.animation || host;
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
    "http/request": async function(command) {
      if (!fetchImpl) return commandErrorMessage(command, new Error("No fetch implementation is available for http/request"));

      try {
        const { url, options } = httpRequestFetchArgs(command);
        const started = nowMs();
        const response = await fetchImpl(url, options);
        const responseFormat = commandValueName(command.response || command.format || "json");
        const body = responseFormat === "text" ? await response.text() : await response.json();
        return commandMessage(command, {
          status: response.status,
          statusText: response.statusText,
          ok: response.ok,
          body,
          headers: headersToObject(response.headers),
          url: response.url || url,
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
        mime: command.mime || "application/octet-stream"
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
        const message = windowEventMessage(command, event, id);
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

export function startApp({ root, init, update, view, handlers = {}, onCommand = () => {}, devtools = null }) {
  const [initialState, initialCommand] = normalizeUpdateResult(typeof init === "function" ? init() : init);
  let state = initialState;
  let component = null;
  let disposed = false;
  const commandLog = [];
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
    runCommand(command, dispatch, handlers, commandLog, onCommand, isActive);
    return state;
  };
  dispatch.__closkellRefs = refs;
  dispatch.__closkellDevtools = devtools;

  emitDevtools(devtools, { type: "app/init", state });
  component = view(state);
  component.mount(root, dispatch);
  emitDevtools(devtools, { type: "app/mount", root: component.root, state });
  runCommand(initialCommand, dispatch, handlers, commandLog, onCommand, isActive);

  return {
    dispatch,
    commands: commandLog,
    refs,
    getRef(name) {
      return refs.get(refName(name));
    },
    get state() {
      return state;
    },
    get root() {
      return component.root;
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      handlers.dispose?.();
      component?.dispose?.();
      emitDevtools(devtools, { type: "app/dispose", state });
    }
  };
}

function normalizeUpdateResult(result) {
  if (Array.isArray(result) && result.length === 2) {
    return result;
  }
  return [result, Cmd.none()];
}

function runCommand(command, dispatch, handlers, commandLog, onCommand, isActive = () => true) {
  if (!command) return;
  if (!isActive()) return;

  if (Array.isArray(command)) {
    command.forEach((item) => runCommand(item, dispatch, handlers, commandLog, onCommand, isActive));
    return;
  }

  const kind = commandKind(command);
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
        if (message !== undefined && isActive()) dispatch(message);
      })
      .catch((error) => {
        handleCommandError(command, error, dispatch, isActive);
      });
  } else if (result !== undefined && isActive()) {
    dispatch(result);
  }
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

function commandMessage(command, value) {
  if (typeof command.toMessage === "function") {
    return command.toMessage(value);
  }
  if (command.msg !== undefined) {
    return command.msg;
  }
  if (command.onSuccess !== undefined) {
    return { kind: command.onSuccess, value };
  }
  return undefined;
}

function namedCommandMessage(kind, fields = {}) {
  if (kind === undefined) return undefined;
  return { kind, ...fields };
}

function commandErrorMessage(command, error) {
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

function httpRequestFetchArgs(command) {
  const request = plainObject(command.request) ? command.request : {};
  const options = plainObject(command.options) ? command.options : {};
  const merged = { ...request, ...options };
  for (const field of HTTP_REQUEST_OPTION_FIELDS) {
    if (command[field] !== undefined) merged[field] = command[field];
  }
  const url = request.url ?? command.url;
  delete merged.url;
  return {
    url,
    options: Object.keys(merged).length ? merged : undefined
  };
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

function windowEventMessage(command, event, id) {
  const payload = windowEventPayload(event);
  return namedCommandMessage(command.onEvent, {
    id,
    ...payload,
    value: payload
  });
}

function applyWindowEventControls(command, event) {
  if (eventControlMatches(command.preventDefault, event)) event?.preventDefault?.();
  if (eventControlMatches(command.stopPropagation, event)) event?.stopPropagation?.();
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

function windowEventPayload(event = {}) {
  return {
    type: String(event.type || ""),
    clientX: numberOrZero(event.clientX),
    clientY: numberOrZero(event.clientY),
    pageX: numberOrZero(event.pageX),
    pageY: numberOrZero(event.pageY),
    screenX: numberOrZero(event.screenX),
    screenY: numberOrZero(event.screenY),
    movementX: numberOrZero(event.movementX),
    movementY: numberOrZero(event.movementY),
    button: numberOrZero(event.button),
    buttons: numberOrZero(event.buttons),
    pointerId: numberOrZero(event.pointerId),
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

function animationFrameMessage(command, id, timestamp) {
  const time = numberOrZero(timestamp);
  return namedCommandMessage(command.onFrame, {
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

function downloadWithBrowser(payload, env, host) {
  const documentRef = env.document || host.document;
  const URLRef = env.URL || host.URL;
  const BlobCtor = env.Blob || host.Blob;

  if (!documentRef || !URLRef?.createObjectURL || !BlobCtor) {
    throw new Error("No browser download implementation is available for file/download");
  }

  const blob = new BlobCtor([payload.content], { type: payload.mime });
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
