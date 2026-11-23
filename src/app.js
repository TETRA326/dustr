// Dustr frontend. Plain JS, no framework, no bundler.
"use strict";

const { invoke, convertFileSrc } = window.__TAURI__.core;
const dialogApi = window.__TAURI__.dialog;
const appWindow = window.__TAURI__.window.getCurrentWindow();

const $ = (id) => document.getElementById(id);
const DIRS = ["up", "right", "down", "left"];
const ACTIONS = ["move", "keep", "recycle", "delete", "none"];
const ACTION_LABEL = { move: "Move", keep: "Keep", recycle: "Recycle", delete: "Delete", none: "Nothing" };
const ARROW = { up: "↑", right: "→", down: "↓", left: "←" };
const WASD = { up: "W", right: "D", down: "S", left: "A" };
const VEC = { up: [0, -1], right: [1, 0], down: [0, 1], left: [-1, 0] };

const EXT = {
  image: "png jpg jpeg jfif gif webp bmp svg ico avif apng",
  video: "mp4 webm mov m4v ogv",
  audio: "mp3 wav ogg oga flac m4a aac opus",
  pdf: "pdf",
  text: "txt md markdown log csv tsv json jsonc xml yml yaml toml ini cfg conf env js mjs cjs ts tsx jsx py rs c h cpp hpp cc cs java kt go rb php html htm css scss less sh bash zsh ps1 psm1 bat cmd sql lua r swift vb gitignore gitattributes editorconfig srt vtt properties gradle nfo reg",
};
const KIND_BY_EXT = {};
for (const [kind, list] of Object.entries(EXT)) for (const e of list.split(" ")) KIND_BY_EXT[e] = kind;

let settings = null;
let platform = "windows";

// ---------- helpers ----------
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const basename = (p) => p.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || p;
const samePath = (a, b) => a && b && a.replace(/[\\/]+$/, "").toLowerCase() === b.replace(/[\\/]+$/, "").toLowerCase();

function fmtSize(n) {
  if (n < 1024) return `${n} B`;
  const u = ["KB", "MB", "GB", "TB"];
  let i = -1;
  do { n /= 1024; i++; } while (n >= 1024 && i < u.length - 1);
  return `${n.toFixed(n < 10 ? 1 : 0)} ${u[i]}`;
}
function fmtDate(ms) {
  if (!ms) return "";
  return new Date(ms).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}
function el(tag, attrs = {}, ...children) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") e.className = v;
    else if (k === "html") e.innerHTML = v;
    else if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
    else if (v !== undefined && v !== null && v !== false) e.setAttribute(k, v);
  }
  for (const c of children) if (c != null) e.append(c);
  return e;
}
function showView(id) {
  for (const v of document.querySelectorAll(".view")) v.hidden = v.id !== id;
}
function animOn() {
  return settings.animations && !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
function anim(node, frames, opts) {
  if (!animOn()) return Promise.resolve();
  return node.animate(frames, { fill: "forwards", ...opts }).finished.catch(() => {});
}

// ---------- theme ----------
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
function applyTheme() {
  const t = settings.theme === "system" ? (darkQuery.matches ? "dark" : "light") : settings.theme;
  document.documentElement.dataset.theme = t;
  appWindow.setTheme(settings.theme === "system" ? null : settings.theme).catch(() => {});
}
darkQuery.addEventListener("change", () => settings && applyTheme());

async function saveSettings() {
  try { await invoke("save_settings", { settings }); } catch (e) { toast(`Could not save settings: ${e}`); }
}

// ---------- toast ----------
let toastTimer = null;
function toast(text, { ms = 2600, bar = 0 } = {}) {
  const t = $("toast");
  $("toast-text").textContent = text;
  const b = $("toast-bar");
  b.getAnimations().forEach((a) => a.cancel());
  b.hidden = !bar;
  t.hidden = false;
  if (bar) b.animate([{ transform: "scaleX(1)" }, { transform: "scaleX(0)" }], { duration: bar, fill: "forwards" });
  clearTimeout(toastTimer);
  toastTimer = setTimeout(hideToast, bar || ms);
}
function hideToast() { $("toast").hidden = true; }

// ---------- dialogs (modal, keyboard-driven) ----------
let modalKeyHandler = null;
function showDialog({ title, body, buttons, countdown, escapeValue }) {
  native.hide();
  return new Promise((resolve) => {
    const d = $("dialog");
    d.innerHTML = "";
    let timer = null;
    let closed = false;
    const done = (v) => {
      if (closed) return;
      closed = true;
      clearTimeout(timer);
      modalKeyHandler = null;
      session.cancelModal = null;
      $("overlay").hidden = true;
      resolve(v);
      native.reshow();
    };
    d.append(el("h3", {}, title));
    if (body) d.append(body);
    if (countdown) {
      const bar = el("div");
      d.append(el("div", { class: "countdown" }, bar));
      if (countdown.ms > 0) bar.animate([{ transform: "scaleX(1)" }, { transform: "scaleX(0)" }], { duration: countdown.ms, fill: "forwards" });
      timer = setTimeout(() => done(countdown.value), countdown.ms);
    }
    const actions = el("div", { class: "actions" });
    for (const b of buttons) {
      actions.append(el("button", { class: `btn ${b.primary ? "primary" : ""}`, onclick: () => done(b.value) },
        b.label, b.hint ? el("span", { class: "kbd" }, b.hint) : null));
    }
    d.append(actions);
    modalKeyHandler = (e) => {
      if (escapeValue && e.key === "Escape") { e.preventDefault(); done(escapeValue); return; }
      for (const b of buttons) if (b.keys && b.keys(e)) { e.preventDefault(); done(b.value); return; }
    };
    $("overlay").hidden = false;
    session.cancelModal = () => done("abort");
  });
}

// ---------- key mapping ----------
function keyToDir(e) {
  if (settings.arrowKeys) {
    const m = { ArrowUp: "up", ArrowRight: "right", ArrowDown: "down", ArrowLeft: "left" }[e.key];
    if (m) return m;
  }
  if (settings.wasdKeys && !e.ctrlKey && !e.altKey && !e.metaKey) {
    const m = { w: "up", d: "right", s: "down", a: "left" }[e.key.toLowerCase()];
    if (m) return m;
  }
  return null;
}
function keyLabel(dir) {
  const parts = [];
  if (settings.arrowKeys) parts.push(ARROW[dir]);
  if (settings.wasdKeys) parts.push(WASD[dir]);
  return parts.join(" / ");
}
function dirFor(action) { return DIRS.find((d) => settings[d] === action); }

// ---------- previews ----------
function kindOf(f) {
  const k = KIND_BY_EXT[f.ext];
  if (k) return k;
  if (platform === "windows" && settings.nativePreviews) return "native";
  return "other";
}
function previewRect() {
  const r = $("preview").getBoundingClientRect();
  return { x: r.left, y: r.top, w: r.width, h: r.height };
}

// Windows preview handlers, hosted in native popups by the backend.
const native = {
  prepared: new Map(), // path -> Promise<bool>
  current: null,       // path currently shown natively
  prepare(path) {
    if (!this.prepared.has(path)) {
      this.prepared.set(path, invoke("preview_prepare", { id: path, path, rect: previewRect() }).catch(() => false));
    }
    return this.prepared.get(path);
  },
  async show(path) {
    this.current = path;
    if (!$("overlay").hidden || $("session").hidden) return;
    await invoke("preview_show", { id: path, rect: previewRect() }).catch(() => {});
  },
  hide() {
    if (this.current) invoke("preview_hide", { id: this.current }).catch(() => {});
  },
  reshow() {
    if (this.current && $("overlay").hidden && !$("session").hidden && !session.busy) this.show(this.current);
  },
  // Replace the live preview with a static snapshot so the card can be animated.
  async detach() {
    if (!this.current) return;
    const id = this.current;
    const url = await invoke("preview_snapshot", { id }).catch(() => null);
    if (url) {
      const p = $("preview");
      p.innerHTML = "";
      p.append(el("img", { class: "snapshot", src: url }));
    }
    await invoke("preview_hide", { id }).catch(() => {});
    this.current = null;
  },
  release(path) {
    if (this.current === path) this.current = null;
    if (this.prepared.delete(path)) invoke("preview_release", { id: path }).catch(() => {});
  },
  releaseAll() {
    this.current = null;
    this.prepared.clear();
    invoke("preview_release_all").catch(() => {});
  },
};

let reshowQueued = false;
function queueReshow() {
  if (reshowQueued) return;
  reshowQueued = true;
  requestAnimationFrame(() => { reshowQueued = false; native.reshow(); });
}
appWindow.onMoved(queueReshow);
appWindow.onResized(queueReshow);
new ResizeObserver(queueReshow).observe($("preview"));

function genericPreview(f, note) {
  return el("div", { class: "generic", html: ICONS.file },
    el("div", { class: "ext" }, f.ext ? f.ext.toUpperCase() : "FILE"),
    note ? el("div", { class: "small" }, note) : null);
}

async function renderPreview(f) {
  const p = $("preview");
  p.innerHTML = "";
  native.current = null;
  const kind = kindOf(f);
  const src = () => convertFileSrc(f.path);
  const stillCurrent = () => session.file === f;

  if (kind === "image") {
    const img = el("img", { src: src(), alt: "" });
    img.onerror = () => { if (stillCurrent()) { p.innerHTML = ""; p.append(genericPreview(f, "Cannot display this image")); } };
    p.append(img);
  } else if (kind === "video") {
    p.append(el("video", { src: src(), controls: "", muted: "", preload: "metadata" }));
  } else if (kind === "audio") {
    p.append(el("div", { class: "generic", html: ICONS.music }), el("audio", { src: src(), controls: "", preload: "metadata" }));
    p.style.flexDirection = "column";
  } else if (kind === "pdf") {
    p.append(el("iframe", { src: `${src()}#toolbar=0&navpanes=0&view=FitH`, tabindex: "-1" }));
  } else if (kind === "text") {
    await renderText(f, p);
  } else if (kind === "native") {
    p.append(el("div", { class: "loading" }, "Loading preview…"));
    const ok = await native.prepare(f.path);
    if (!stillCurrent()) return;
    if (ok) {
      p.innerHTML = "";
      await native.show(f.path);
    } else {
      await renderFallback(f, p);
    }
  } else {
    await renderFallback(f, p);
  }
}

async function renderText(f, p) {
  try {
    const r = await invoke("read_text", { path: f.path, maxBytes: 64 * 1024 });
    if (session.file !== f) return false;
    p.innerHTML = "";
    if (r.binary) { p.append(genericPreview(f)); return false; }
    p.append(el("pre", {}, r.text + (r.truncated ? "\n…" : "")));
    return true;
  } catch {
    p.innerHTML = "";
    p.append(genericPreview(f, "Cannot read file"));
    return false;
  }
}

async function renderFallback(f, p) {
  // Small unknown files might be plain text.
  if (f.size > 0 && f.size < 2 * 1024 * 1024) {
    if (await renderText(f, p)) return;
    if (session.file !== f) return;
  }
  p.innerHTML = "";
  p.append(genericPreview(f));
}

function preloadAhead() {
  if (platform !== "windows" || !settings.nativePreviews) return;
  const { files, index } = session;
  let n = 0;
  for (let i = index + 1; i < files.length && n < 3; i++) {
    if (kindOf(files[i]) === "native") { native.prepare(files[i].path); n++; }
  }
}

// ---------- session state ----------
const session = {
  active: false,
  aborted: false,
  busy: false,
  root: null,
  recursive: false,
  dir: null,
  files: [],
  index: 0,
  file: null,
  counts: null,
  undo: [],
  moveDest: null,
  decisions: {},
  pendingDelete: null,
  resolveAction: null,
  cancelModal: null,
};

function newCounts() {
  return { kept: 0, moved: 0, recycled: 0, deleted: 0, bytesFreed: 0, foldersSkipped: 0, foldersRemoved: 0 };
}

// ---------- zones ----------
function renderZones() {
  for (const dir of DIRS) {
    const action = settings[dir];
    const color = `var(--${action})`;
    for (const node of document.querySelectorAll(`[data-dir="${dir}"]`)) {
      node.style.setProperty("--c", color);
      node.dataset.action = action;
    }
    const z = document.querySelector(`.zone[data-dir="${dir}"]`);
    z.innerHTML = "";
    z.append(
      el("span", { class: "chip", html: ICONS[action] }),
      el("span", {}, ACTION_LABEL[action]),
      el("span", { class: "key" }, keyLabel(dir)),
    );
  }
  const hint = [];
  if (session.recursive) hint.push("Esc ends the session");
  else hint.push("Esc to finish");
  hint.push("Ctrl+Z undo");
  $("key-hint").textContent = hint.join(" · ");
}
function flashZone(dir) {
  const z = document.querySelector(`.zone[data-dir="${dir}"]`);
  z.classList.add("flash");
  setTimeout(() => z.classList.remove("flash"), 140);
}

function updateUndoButton() {
  const top = session.undo[session.undo.length - 1];
  $("undo").disabled = !top || top.type === "delete";
  $("undo").title = top && top.type === "delete" ? "Permanently deleted files cannot be restored" : "Undo last swipe (Ctrl+Z)";
}

// ---------- card ----------
async function showCard() {
  const f = session.files[session.index];
  session.file = f;
  session.pendingDelete = null;
  const card = $("card");
  card.getAnimations().forEach((a) => a.cancel());
  card.style.transform = "";
  card.style.opacity = "";
  $("file-name").textContent = f.name;
  $("file-name").title = f.name;
  $("file-path").replaceChildren(el("span", { class: "ltr" }, f.path));
  $("file-path").title = f.path;
  $("file-meta").textContent = [fmtSize(f.size), f.modified ? `modified ${fmtDate(f.modified)}` : ""].filter(Boolean).join(" · ");
  $("progress").textContent = `${session.index + 1} of ${session.files.length}`;
  $("preview").style.flexDirection = "";
  updateUndoButton();
  anim(card, [{ opacity: 0, transform: "scale(0.97)" }, { opacity: 1, transform: "scale(1)" }], { duration: 140, easing: "ease-out" });
  await renderPreview(f);
  preloadAhead();
}

// ---------- animations ----------
async function animateOut(action, dir) {
  const card = $("card");
  if (!animOn()) return;
  const [vx, vy] = VEC[dir];
  const W = window.innerWidth, H = window.innerHeight;
  if (action === "move") {
    await anim(card, [{ transform: "scale(1)" }, { transform: "scale(0.94)" }], { duration: 70, easing: "ease-out" });
    contrails(card, vx, vy);
    await anim(card, [
      { transform: "scale(0.94) translate(0,0)", opacity: 1 },
      { transform: `scale(0.8) translate(${vx * W * 1.2}px, ${vy * H * 1.2}px)`, opacity: 1 },
    ], { duration: 260, easing: "cubic-bezier(.5,0,.9,.4)" });
  } else if (action === "delete") {
    await trashAnimation(card, dir);
  } else {
    const spin = (vx || vy) * 28;
    await anim(card, [
      { transform: "translate(0,0) rotate(0deg)", opacity: 1 },
      { transform: `translate(${vx * W}px, ${vy * H}px) rotate(${spin}deg)`, opacity: 0.6 },
    ], { duration: 340, easing: "cubic-bezier(.4,0,.8,.5)" });
  }
}

function contrails(card, vx, vy) {
  const r = card.getBoundingClientRect();
  const layer = $("fx-layer");
  const horizontal = vx !== 0;
  const len = horizontal ? r.width * 0.9 : r.height * 0.9;
  for (let i = 0; i < 3; i++) {
    const t = el("div", { class: "trail" });
    const offset = (i - 1) * (horizontal ? r.height * 0.25 : r.width * 0.25);
    if (horizontal) {
      t.style.width = `${len}px`;
      t.style.top = `${r.top + r.height / 2 + offset}px`;
      t.style.left = vx > 0 ? `${r.left - len * 0.3}px` : `${r.right - len * 0.7}px`;
      t.style.transformOrigin = vx > 0 ? "left center" : "right center";
    } else {
      t.style.width = "2px";
      t.style.height = `${len}px`;
      t.style.left = `${r.left + r.width / 2 + offset}px`;
      t.style.top = vy > 0 ? `${r.top - len * 0.3}px` : `${r.bottom - len * 0.7}px`;
      t.style.transformOrigin = vy > 0 ? "center top" : "center bottom";
    }
    const axis = horizontal ? "scaleX" : "scaleY";
    layer.append(t);
    t.animate([
      { transform: `${axis}(0)`, opacity: 0.9 },
      { transform: `${axis}(1)`, opacity: 0.6, offset: 0.5 },
      { transform: `${axis}(1)`, opacity: 0 },
    ], { duration: 420, delay: i * 30, easing: "ease-out" }).finished.finally(() => t.remove());
  }
}

function trashParts(dir) {
  const z = document.querySelector(`.zone[data-dir="${dir}"] .chip`);
  return { chip: z, lid: z && z.querySelector(".lid") };
}

async function trashAnimation(card, dir) {
  const { chip, lid } = trashParts(dir);
  if (!chip) return;
  const cr = card.getBoundingClientRect();
  const tr = chip.getBoundingClientRect();
  const dx = tr.left + tr.width / 2 - (cr.left + cr.width / 2);
  const dy = tr.top + tr.height / 2 - (cr.top + cr.height / 2);
  const grow = anim(chip, [{ transform: "scale(1)" }, { transform: "scale(1.7)" }], { duration: 160, easing: "ease-out" });
  const open = lid ? anim(lid, [{ transform: "rotate(0deg)" }, { transform: "rotate(40deg)" }], { duration: 160, easing: "ease-out" }) : null;
  await Promise.all([grow, open]);
  await anim(card, [
    { transform: "translate(0,0) scale(1)", opacity: 1 },
    { transform: `translate(${dx}px, ${dy}px) scale(0.04)`, opacity: 0.5 },
  ], { duration: 320, easing: "cubic-bezier(.55,0,.85,.45)" });
  const close = lid ? anim(lid, [{ transform: "rotate(40deg)" }, { transform: "rotate(0deg)" }], { duration: 140, easing: "ease-in" }) : null;
  await close;
  await anim(chip, [{ transform: "scale(1.7)" }, { transform: "scale(1)" }], { duration: 160, easing: "ease-out" });
  chip.getAnimations().forEach((a) => a.cancel());
  if (lid) lid.getAnimations().forEach((a) => a.cancel());
}

async function bounceDown(dir) {
  const card = $("card");
  const [vx, vy] = VEC[dir];
  const { lid } = trashParts(dir);
  if (lid) {
    anim(lid, [{ transform: "rotate(0deg)" }, { transform: "rotate(25deg)" }, { transform: "rotate(0deg)" }], { duration: 380 })
      .then(() => lid.getAnimations().forEach((a) => a.cancel()));
  }
  await anim(card, [
    { transform: "translate(0,0)" },
    { transform: `translate(${vx * 26}px, ${vy * 26}px)`, offset: 0.4 },
    { transform: `translate(${vx * -4}px, ${vy * -4}px)`, offset: 0.75 },
    { transform: "translate(0,0)" },
  ], { duration: 380, easing: "ease-out" });
  card.getAnimations().forEach((a) => a.cancel());
}

// ---------- actions ----------
async function getMoveDest() {
  if (settings.moveMode === "preset" && settings.movePath) {
    if (await invoke("path_exists", { path: settings.movePath })) return settings.movePath;
    toast("The preset move folder no longer exists. Choose another.");
  }
  if (session.moveDest) return session.moveDest;
  native.hide();
  const picked = await dialogApi.open({ directory: true, multiple: false, title: "Where should moved files go?" }).catch(() => null);
  native.reshow();
  if (!picked) return null;
  session.moveDest = Array.isArray(picked) ? picked[0] : picked;
  return session.moveDest;
}

async function perform(action, dir) {
  const f = session.file;
  if (action === "delete" && settings.deleteConfirm) {
    const now = performance.now();
    const p = session.pendingDelete;
    if (!p || p.path !== f.path || now > p.until) {
      const ms = settings.deleteConfirmSeconds * 1000;
      session.pendingDelete = { path: f.path, until: now + ms };
      toast(`Press ${keyLabel(dir)} again to permanently delete`, { bar: Math.max(ms, 1) });
      if (animOn()) {
        session.busy = true;
        const wasNative = native.current;
        if (wasNative) await native.detach();
        await bounceDown(dir);
        session.busy = false;
        if (wasNative && session.file === f) { $("preview").innerHTML = ""; await native.show(wasNative); }
      }
      return false;
    }
    hideToast();
  }

  let entry = { type: action, file: f, dir: session.dir, index: session.index };
  try {
    if (action === "keep") {
      session.counts.kept++;
    } else if (action === "recycle") {
      await invoke("recycle", { path: f.path });
      session.counts.recycled++;
      session.counts.bytesFreed += f.size;
    } else if (action === "delete") {
      await invoke("delete_file", { path: f.path });
      session.counts.deleted++;
      session.counts.bytesFreed += f.size;
    } else if (action === "move") {
      const dest = await getMoveDest();
      if (!dest) return false;
      if (samePath(dest, session.dir)) { toast("That is the folder being dusted. Choose a different one."); session.moveDest = null; return false; }
      entry.newPath = await invoke("move_file", { path: f.path, destDir: dest });
      session.counts.moved++;
    }
  } catch (e) {
    toast(`${ACTION_LABEL[action]} failed: ${e}`, { ms: 4000 });
    return false;
  }
  session.undo.push(entry);
  return true;
}

async function onDirection(dir) {
  if (session.busy || !session.file) return;
  const action = settings[dir];
  flashZone(dir);
  if (action === "none") return;
  session.busy = true;
  try {
    const done = await perform(action, dir);
    if (!done) return;
    const f = session.file;
    await native.detach();
    await animateOut(action, dir);
    native.release(f.path);
    session.resolveAction && session.resolveAction("next");
  } finally {
    session.busy = false;
  }
}

async function undo() {
  if (session.busy) return;
  const e = session.undo[session.undo.length - 1];
  if (!e || e.type === "delete") return;
  session.busy = true;
  try {
    if (e.type === "recycle") await invoke("restore", { path: e.file.path });
    if (e.type === "move") await invoke("move_back", { from: e.newPath, to: e.file.path });
    session.undo.pop();
    const c = session.counts;
    if (e.type === "keep") c.kept--;
    if (e.type === "move") c.moved--;
    if (e.type === "recycle") { c.recycled--; c.bytesFreed -= e.file.size; }
    if (e.type === "keep" || e.type === "recycle" || e.type === "move") {
      if (e.dir === session.dir && session.resolveAction) {
        session.resolveAction({ undoTo: e.index });
      } else {
        toast(`Restored ${e.file.name}`);
        updateUndoButton();
      }
    }
  } catch (err) {
    toast(`Undo failed: ${err}`, { ms: 4000 });
  } finally {
    session.busy = false;
  }
}

// ---------- traversal ----------
async function askInclude(kind, count) {
  const v = await showDialog({
    title: `Include ${kind} files?`,
    body: el("p", { class: "muted" }, `This folder has ${count} ${kind} file${count === 1 ? "" : "s"}. The choice applies to the rest of this session.`),
    buttons: [
      { label: "Skip", value: "skip", hint: "Esc", keys: (e) => e.key === "Escape" },
      { label: "Include", value: "include", primary: true, hint: "Enter", keys: (e) => e.key === "Enter" },
    ],
  });
  return v === "abort" ? "skip" : v;
}

async function filterFiles(files) {
  const rule = (f) => (f.system ? "system" : f.hidden ? "hidden" : null);
  for (const kind of ["system", "hidden"]) {
    const setting = settings[kind === "system" ? "systemFiles" : "hiddenFiles"];
    const n = files.filter((f) => rule(f) === kind).length;
    if (!n) continue;
    let d = setting;
    if (d === "ask") d = session.decisions[kind] ??= await askInclude(kind, n);
    if (session.aborted) return [];
    if (d === "skip") files = files.filter((f) => rule(f) !== kind);
  }
  return files;
}

function sortFiles(files) {
  const by = settings.sortBy;
  const key = {
    modified: (f) => f.modified,
    created: (f) => f.created,
    size: (f) => f.size,
    name: (f) => f.name.toLowerCase(),
    type: (f) => `${f.ext}\u0000${f.name.toLowerCase()}`,
  }[by] || ((f) => f.modified);
  const dir = settings.sortDir === "desc" ? -1 : 1;
  const coll = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });
  return files.sort((a, b) => {
    const ka = key(a), kb = key(b);
    const c = typeof ka === "string" ? coll.compare(ka, kb) : ka - kb;
    return c * dir;
  });
}

async function dustFiles(dir, files) {
  files = sortFiles(await filterFiles(files));
  if (!files.length || session.aborted) return;
  session.dir = dir;
  session.files = files;
  session.index = 0;
  $("folder-name").textContent = basename(dir);
  $("folder-name").title = dir;
  while (session.index < files.length && !session.aborted) {
    await showCard();
    const r = await new Promise((res) => { session.resolveAction = res; });
    session.resolveAction = null;
    if (r === "abort") break;
    if (r && r.undoTo !== undefined) {
      native.release(session.file.path);
      session.index = r.undoTo;
    } else {
      session.index++;
    }
  }
  session.file = null;
  native.releaseAll();
}

function relPath(dir) {
  const root = session.root.replace(/[\\/]+$/, "");
  return dir.startsWith(root) ? dir.slice(root.length).replace(/^[\\/]/, "") : dir;
}

async function splash(dir, listing) {
  const nf = listing.files.length, nd = listing.dirs.length;
  const body = el("div", {},
    el("div", { class: "folder-name" }, basename(dir)),
    el("div", { class: "muted small" }, relPath(dir)),
    el("div", { class: "muted small", style: "margin-top:6px" }, `${nf} file${nf === 1 ? "" : "s"} · ${nd} subfolder${nd === 1 ? "" : "s"}`));
  const isDir = (want) => (e) => !e.repeat && keyToDir(e) === want;
  return showDialog({
    title: "Next folder",
    body,
    countdown: { ms: settings.splashSeconds * 1000, value: "go" },
    escapeValue: "abort",
    buttons: [
      { label: "Skip all", value: "tree", hint: keyLabel("left"), keys: isDir("left") },
      { label: "Skip folder", value: "files", hint: keyLabel("right"), keys: isDir("right") },
      { label: "Start", value: "go", primary: true, hint: "Enter", keys: (e) => e.key === "Enter" || isDir("up")(e) },
    ],
  });
}

async function handleEmpty(dir, hadContent) {
  if (!hadContent || settings.emptyFolders === "never") return;
  if (!(await invoke("dir_is_empty", { path: dir }))) return;
  let remove = settings.emptyFolders === "delete";
  if (settings.emptyFolders === "ask") {
    const v = await showDialog({
      title: "Folder is now empty",
      body: el("p", {}, el("strong", {}, basename(dir)), el("span", { class: "muted" }, ` has nothing left in it. Remove the folder?`)),
      buttons: [
        { label: "Keep", value: "keep", hint: "Esc", keys: (e) => e.key === "Escape" },
        { label: "Remove", value: "remove", primary: true, hint: "Enter", keys: (e) => e.key === "Enter" },
      ],
    });
    remove = v === "remove";
  }
  if (!remove) return;
  try {
    await invoke("remove_empty_dir", { path: dir });
    session.counts.foldersRemoved++;
  } catch (e) {
    toast(`Could not remove folder: ${e}`);
  }
}

async function dustTree(dir, isRoot) {
  if (session.aborted) return;
  let listing;
  try {
    listing = await invoke("list_dir", { path: dir });
  } catch (e) {
    toast(String(e), { ms: 4000 });
    return;
  }
  const hadContent = listing.files.length + listing.dirs.length > 0;
  let doFiles = true;
  if (!isRoot && listing.files.length) {
    const c = await splash(dir, listing);
    if (c === "abort" || session.aborted) { session.aborted = true; return; }
    if (c === "tree") { session.counts.foldersSkipped++; return; }
    if (c === "files") { session.counts.foldersSkipped++; doFiles = false; }
  }
  if (doFiles) await dustFiles(dir, listing.files);
  if (session.recursive) {
    for (const sub of listing.dirs) {
      if (session.aborted) return;
      if (samePath(sub, session.moveDest) || samePath(sub, settings.moveMode === "preset" ? settings.movePath : null)) continue;
      await dustTree(sub, false);
    }
    if (!isRoot && !session.aborted) await handleEmpty(dir, hadContent);
  }
}

async function startSession(root, recursive) {
  Object.assign(session, {
    active: true, aborted: false, busy: false, root, recursive,
    dir: root, files: [], index: 0, file: null, counts: newCounts(),
    undo: [], moveDest: null, decisions: {}, pendingDelete: null,
  });
  renderZones();
  $("folder-name").textContent = basename(root);
  $("progress").textContent = "";
  $("file-name").textContent = "";
  $("file-path").textContent = "";
  $("file-meta").textContent = "";
  $("preview").innerHTML = "";
  showView("session");
  await dustTree(root, true);
  native.releaseAll();
  session.active = false;
  hideToast();
  showSummary();
}

function endSession() {
  session.aborted = true;
  if (session.cancelModal && !$("overlay").hidden) session.cancelModal();
  if (session.resolveAction) session.resolveAction("abort");
}

function showSummary() {
  const c = session.counts;
  const t = $("summary-table");
  t.innerHTML = "";
  const row = (label, value, color) =>
    t.append(el("tr", {}, el("td", {}, el("span", { class: "dot", style: color ? `--c: var(--${color})` : "" }), label), el("td", {}, String(value))));
  row("Kept", c.kept, "keep");
  row("Moved", c.moved, "move");
  row("Recycled", c.recycled, "recycle");
  row("Permanently deleted", c.deleted, "delete");
  if (session.recursive) {
    row("Folders skipped", c.foldersSkipped);
    row("Empty folders removed", c.foldersRemoved);
  }
  row("Space freed", fmtSize(Math.max(0, c.bytesFreed)));
  $("summary-folder").textContent = session.root;
  showView("summary");
  $("summary-done").focus();
}

// ---------- settings view ----------
let settingsReturn = "home";
function openSettings() {
  settingsReturn = !$("session").hidden ? "session" : "home";
  native.hide();
  renderSettings();
  showView("settings");
}
function closeSettings() {
  showView(settingsReturn);
  if (settingsReturn === "session") { renderZones(); native.reshow(); }
}

function seg(options, value, onChange) {
  const wrap = el("div", { class: "seg" });
  for (const [v, label] of options) {
    wrap.append(el("button", { class: v === value ? "on" : "", onclick: () => { onChange(v); renderSettings(); } }, label));
  }
  return wrap;
}
function sw(checked, onChange) {
  const input = el("input", { type: "checkbox" });
  input.checked = checked;
  input.addEventListener("change", () => { onChange(input.checked); renderSettings(); });
  return el("label", { class: "switch" }, input, el("span"));
}
function select(options, value, onChange) {
  const s = el("select", { onchange: (e) => { onChange(e.target.value); renderSettings(); } });
  for (const [v, label] of options) {
    const o = el("option", { value: v }, label);
    if (v === value) o.selected = true;
    s.append(o);
  }
  return s;
}
function row(label, desc, control, extraClass = "") {
  return el("div", { class: `row ${extraClass}` }, el("div", { class: "label" }, el("span", {}, label), desc ? el("span", { class: "muted" }, desc) : null), control);
}
function set(key) {
  return (v) => { settings[key] = v; saveSettings(); if (key === "theme") applyTheme(); };
}

async function renderSettings() {
  const body = $("settings-body");
  const scroll = body.scrollTop;
  body.innerHTML = "";
  const S = settings;

  const appearance = el("section", {}, el("h3", {}, "Appearance"),
    row("Theme", null, seg([["light", "Light"], ["dark", "Dark"], ["system", "System"]], S.theme, set("theme"))),
    row("Animations", "Swipe and trash-can effects", sw(S.animations, set("animations"))));

  const dirs = el("section", {}, el("h3", {}, "Directions"));
  for (const d of DIRS) {
    const r = row(`${d[0].toUpperCase()}${d.slice(1)}`, null,
      select(ACTIONS.map((a) => [a, ACTION_LABEL[a]]), S[d], (v) => {
        // Each action lives on one direction: swap with whoever had it.
        const other = DIRS.find((x) => x !== d && S[x] === v);
        if (other && v !== "none") S[other] = S[d];
        S[d] = v;
        saveSettings();
      }), "dir-row");
    r.querySelector(".label").prepend(el("span", { class: "swatch", style: `--c: var(--${S[d]})` }));
    dirs.append(r);
  }
  dirs.append(
    row("Arrow keys", null, sw(S.arrowKeys, (v) => { S.arrowKeys = v || !S.wasdKeys; saveSettings(); })),
    row("WASD keys", null, sw(S.wasdKeys, (v) => { S.wasdKeys = v || !S.arrowKeys; saveSettings(); })));

  const move = el("section", {}, el("h3", {}, "Move"),
    row("Destination", null, seg([["ask", "Ask each session"], ["preset", "Fixed folder"]], S.moveMode, set("moveMode"))));
  if (S.moveMode === "preset") {
    const input = el("input", { type: "text", value: S.movePath, placeholder: "No folder chosen", readonly: "" });
    const browse = el("button", { class: "btn small", onclick: async () => {
      const p = await dialogApi.open({ directory: true, multiple: false, title: "Folder for moved files" }).catch(() => null);
      if (p) { S.movePath = Array.isArray(p) ? p[0] : p; saveSettings(); renderSettings(); }
    } }, "Browse…");
    move.append(row("Folder", null, el("div", { class: "path-pick" }, input, browse), "indent"));
  }

  const del = el("section", {}, el("h3", {}, "Permanent delete"),
    row("Press twice to confirm", "A first press shows a reminder; press again before it runs out", sw(S.deleteConfirm, set("deleteConfirm"))));
  if (S.deleteConfirm) {
    const val = el("span", { class: "muted small" }, `${S.deleteConfirmSeconds.toFixed(1)} s`);
    const range = el("input", { type: "range", min: "0", max: "10", step: "0.5", value: String(S.deleteConfirmSeconds) });
    range.addEventListener("input", () => { S.deleteConfirmSeconds = Number(range.value); val.textContent = `${S.deleteConfirmSeconds.toFixed(1)} s`; });
    range.addEventListener("change", saveSettings);
    del.append(row("Time window", null, el("div", { class: "path-pick" }, range, val), "indent"));
  }

  const order = el("section", {}, el("h3", {}, "Files"),
    row("Sort by", null, el("div", { class: "path-pick" },
      select([["modified", "Date modified"], ["created", "Date created"], ["name", "Name"], ["size", "Size"], ["type", "Type"]], S.sortBy, set("sortBy")),
      select([["asc", S.sortBy === "name" || S.sortBy === "type" ? "A → Z" : S.sortBy === "size" ? "Smallest first" : "Oldest first"],
              ["desc", S.sortBy === "name" || S.sortBy === "type" ? "Z → A" : S.sortBy === "size" ? "Largest first" : "Newest first"]], S.sortDir, set("sortDir")))),
    row("Hidden files", null, seg([["ask", "Ask"], ["include", "Include"], ["skip", "Skip"]], S.hiddenFiles, set("hiddenFiles"))),
    row("System files", null, seg([["ask", "Ask"], ["include", "Include"], ["skip", "Skip"]], S.systemFiles, set("systemFiles"))));
  if (platform === "windows") {
    order.append(row("Document previews", "Use Windows preview handlers (Word, Excel, PowerPoint…)", sw(S.nativePreviews, set("nativePreviews"))));
  }

  const sub = el("section", {}, el("h3", {}, "Subfolders"),
    row("Empty folders", "After dusting a subfolder leaves it empty", seg([["ask", "Ask"], ["delete", "Remove"], ["never", "Keep"]], S.emptyFolders, set("emptyFolders"))));
  const sval = el("span", { class: "muted small" }, `${S.splashSeconds.toFixed(1)} s`);
  const srange = el("input", { type: "range", min: "1", max: "10", step: "0.5", value: String(S.splashSeconds) });
  srange.addEventListener("input", () => { S.splashSeconds = Number(srange.value); sval.textContent = `${S.splashSeconds.toFixed(1)} s`; });
  srange.addEventListener("change", saveSettings);
  sub.append(row("Next-folder countdown", "Time to skip a folder before it starts", el("div", { class: "path-pick" }, srange, sval)));

  body.append(appearance, dirs, move, del, order, sub);

  if (platform === "windows") {
    const on = await invoke("shell_menu_status").catch(() => false);
    const menu = el("section", {}, el("h3", {}, "Explorer"),
      row("Right-click menu", "Adds Dustr under “Show more options” for folders", sw(on, async (v) => {
        try { await invoke("shell_menu_set", { enabled: v }); } catch (e) { toast(String(e)); }
        renderSettings();
      })));
    body.append(menu);
  }
  body.scrollTop = scroll;
}

// ---------- global keys ----------
document.addEventListener("keydown", (e) => {
  if (modalKeyHandler) { modalKeyHandler(e); return; }
  if (!$("settings").hidden) { if (e.key === "Escape") closeSettings(); return; }
  if ($("session").hidden) return;
  if (e.key === "Escape") { e.preventDefault(); endSession(); return; }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") { e.preventDefault(); undo(); return; }
  const dir = keyToDir(e);
  if (dir) {
    e.preventDefault();
    if (e.repeat) return; // holding a key must never confirm a delete
    onDirection(dir);
  }
});

// ---------- boot ----------
async function chooseFolder() {
  const p = await dialogApi.open({ directory: true, multiple: false, title: "Choose a folder to dust" }).catch(() => null);
  if (!p) return;
  startSession(Array.isArray(p) ? p[0] : p, $("include-subfolders").checked);
}

async function boot() {
  $("home-settings").innerHTML = ICONS.settings;
  $("session-settings").innerHTML = ICONS.settings;
  $("session-end").innerHTML = ICONS.close;
  $("settings-back").innerHTML = ICONS.back;
  $("open-location").innerHTML = `${ICONS.folder}<span>Open file location</span>`;
  $("undo").innerHTML = `${ICONS.undo}<span>Undo</span>`;

  $("home-settings").onclick = openSettings;
  $("session-settings").onclick = openSettings;
  $("settings-back").onclick = closeSettings;
  $("session-end").onclick = endSession;
  $("choose-folder").onclick = chooseFolder;
  $("summary-done").onclick = () => showView("home");
  $("undo").onclick = undo;
  $("open-location").onclick = () => session.file && invoke("open_location", { path: session.file.path }).catch((e) => toast(String(e)));
  // In a session, clicked buttons must not keep focus (Enter/Space would re-trigger them).
  document.addEventListener("click", (e) => {
    const b = e.target.closest && e.target.closest("button");
    if (b && !$("session").hidden) b.blur();
  });

  platform = await invoke("platform").catch(() => "windows");
  settings = await invoke("get_settings");
  applyTheme();

  const launch = await invoke("launch_args").catch(() => ({}));
  if (launch && launch.path) {
    startSession(launch.path, !!launch.recursive);
  } else {
    showView("home");
  }
}

boot();
