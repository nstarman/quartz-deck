// quartz-deck editor. Every change is a CLI op sent to POST /api/op, so the GUI
// can do nothing the CLI cannot (FR-010). The .qmd file stays the only state.
import { region, hintsFor, gate, reserialize, splice } from '/ui/inline.js';
import { showToolbar, hideToolbar, place as placeToolbar, placeNear } from '/ui/toolbar.js';
import { openSource, closeSource, sourceError, sourceText } from '/ui/source.js';
import { makeActions, INSERTS, TRANSITIONS } from '/ui/actions.js';
import * as keys from '/ui/keys.js';
import * as settings from '/ui/settings.js';
import { submenu } from '/ui/menubar.js';
import { openMenu, closeMenu, menuOpen } from '/ui/menu.js';
import { makeList } from '/ui/list.js';
import * as pane from '/ui/pane.js';
import * as run from '/ui/run.js';
import * as focus from '/ui/focus.js';
import { ask, confirmSheet } from '/ui/sheet.js';

const $ = (sel, root = document) => root.querySelector(sel);
const frame = $('#preview');
const statusEl = $('#status');
const notice = $('#notice');
const banner = $('#banner');
$('.close', banner).addEventListener('click', () => { banner.hidden = true; });
const panel = $('#panel');

const state = {
  version: null, // last deck version we saw (sent as `expect`)
  deck: null, // /api/deck
  anchor: null, // current slide's preview anchor
  sel: null, // selected element ref "<slide>/<i>"
  goAfterRender: null, // anchor to show once the next render lands
  want: null, // anchor the freshly loaded preview should open on
  selectIdAfterRender: null, // element id (e.g. a new box) to select once rendered
  session: null, // open on-slide edit (spec 003), see beginSession
  pendingRender: null, // newest render {html, version} that arrived while editing (applied when it ends)
  loading: null, // deck version of the render the preview is loading
  editAfterRender: null, // {ref, mode}: open a just-inserted element for editing once rendered
  clip: null, // last copied element markdown (fallback when the clipboard can't be read)
};

// ---------- helpers ----------

/** The connection/render state: shown only when it needs attention (`quiet` hides it, still read out). */
function status(text, quiet = false) {
  statusEl.textContent = text;
  statusEl.classList.toggle('quiet', quiet);
}

function say(msg, isError = false) {
  notice.textContent = msg;
  notice.className = isError ? 'error' : '';
  clearTimeout(say.t);
  say.t = setTimeout(() => { notice.textContent = ''; }, isError ? 8000 : 3000);
}

/** The preview element for an element ref `<slide>/<i>`. */
const elOf = (ref) => frame.contentDocument?.querySelector(`[data-qd="${CSS.escape(ref)}"]`);

function frameWin() {
  try { return frame.contentWindow; } catch { return null; }
}

/** Deck slide for a preview anchor; null for the front-matter title slide. */
function slideByAnchor(anchor) {
  const slides = state.deck?.slides || [];
  if (anchor === 'title-slide') return null;
  return slides.find((s) => s.anchor === anchor) || (anchor ? null : slides[0] || null);
}

function slideById(id) {
  return (state.deck?.slides || []).find((s) => s.id === id) || null;
}

function editing() {
  const a = document.activeElement;
  return a && (a.tagName === 'INPUT' || a.tagName === 'TEXTAREA' || a.tagName === 'SELECT');
}

// ---------- API ----------

async function fetchDeck() {
  const res = await fetch('/api/deck');
  if (!res.ok) return;
  state.deck = await res.json();
  state.version = state.deck.version;
  // the deck's name in the top row and the tab (spec 008 US6)
  $('#deck-name .title').textContent = state.deck.title || '';
  $('#deck-name .file').textContent = state.deck.file || '';
  document.title = state.deck.title || state.deck.file || 'quartz-deck';
  focus.setVersion(state.version);
  slideList.render();
  renderSlidebar();
}

// slides as of the deck version we last saw: menus and keys resolve their target
// at once (spec 007 SC-008); any change on disk empties it (the 'changed' event)
let slides = { version: null, map: new Map() };
function fetchSlide(ref) {
  if (slides.version !== state.version) slides = { version: state.version, map: new Map() };
  let p = slides.map.get(ref);
  if (!p) {
    p = fetch('/api/slide/' + encodeURIComponent(ref)).then(async (res) => (res.ok ? (await res.json()).slide : null));
    p.catch(() => slides.map.delete(ref));
    slides.map.set(ref, p);
  }
  return p;
}

/** The block for an element ref `<slide>/<i>`, or null. */
async function fetchBlock(ref) {
  const cut = ref.lastIndexOf('/');
  const slide = await fetchSlide(ref.slice(0, cut));
  return slide?.blocks[Number(ref.slice(cut + 1))] || null;
}

/** The deck's front matter (without op()'s status message). */
async function fetchMeta() {
  const res = await fetch('/api/op', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ op: 'meta' }) });
  return res.ok ? (await res.json()).front_matter || {} : {};
}

/**
 * Run one CLI op. Returns the result object, or null after reporting the error.
 * `fileExpect: false` skips the whole-file version check (on-slide saves check
 * just their element with `expect-text`, so edits elsewhere don't block them).
 */
async function op(name, args = {}, { fileExpect = true } = {}) {
  status('Saving…', true);
  const res = await fetch('/api/op', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-QD-Token': run.token },
    body: JSON.stringify({ op: name, args, ...(fileExpect ? { expect: state.version } : {}) }),
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    const e = body.error || { code: res.status, message: res.statusText };
    op.lastError = e;
    status('Not saved');
    if (e.code === 'E_CONFLICT' && !fileExpect) {
      say(`${name}: changed elsewhere while you were editing — choose a version`, true);
    } else if (e.code === 'E_CONFLICT') {
      say('The file changed elsewhere — reloaded; please retry.', true);
      await fetchDeck();
      await renderPanel();
    } else {
      say(`${name}: ${e.message}`, true);
    }
    return null;
  }
  if (body.version) state.version = body.version;
  status('Saved', true);
  say(`${name} ✓`);
  return body;
}

// ---------- preview ----------

function load(path, version = null) {
  state.loading = version;
  // Reveal may ignore the initial URL hash (e.g. when it starts in scroll view),
  // so it is steered once it reports ready (see hookFrame): to the slide an op
  // chose, else to the one shown then (the person may move on while it loads)
  state.want = state.goAfterRender;
  state.goAfterRender = null;
  // until the new render is hooked up, clicks and keys land on the old one
  frame.setAttribute('aria-busy', 'true');
  frame.src = path + '?v=' + Date.now();
}

function go(anchor) {
  const hash = '#/' + anchor;
  history.replaceState(null, '', hash);
  const w = frameWin();
  if (w) w.location.hash = hash;
}

// (none of it while presenting: html.qd-present)
const PREVIEW_CSS = `html:not(.qd-present) {
  [data-qd] { cursor: pointer; }
  .slide-menu-button { display: none !important; }
  /* an untitled slide (spec 008 US1): a placeholder on the canvas only, never in the file */
  .reveal .slides section > h2:empty::before { content: 'Click to add title'; color: #9a9aa0; font-weight: 400; }
  /* a layout's empty boxes (spec 009): what goes there, on the canvas only */
  .qd-ph:not(:has(*)) { display: flex !important; align-items: center; justify-content: center; outline: 2px dashed #b9b9bf;
    outline-offset: -2px; border-radius: 6px; cursor: text; }
  .qd-ph:not(:has(*))::before { content: 'Click to add text'; color: #9a9aa0; font-size: .6em; }
  .qd-ph[data-ph=image]:not(:has(*))::before { content: 'Drop an image'; }
  .qd-ph[data-ph=image]:not(:has(*)) { cursor: copy; }
  [data-qd]:hover { outline: 2px dashed rgba(42, 91, 215, .55); outline-offset: 2px; }
  .qd-sel, .qd-sel:hover { outline: 3px solid #2a5bd7 !important; outline-offset: 2px; }
  .qd-editing, .qd-editing:hover { outline: 3px solid #2a5bd7 !important; outline-offset: 4px; cursor: text; }
  .qd-editing:focus { outline-style: solid; }
  /* every fragment is shown (and reachable) in the editor; the marker says it appears on click */
  .reveal .slides section .fragment { opacity: 1 !important; visibility: inherit !important; transform: none !important;
    box-shadow: inset 3px 0 0 rgba(42, 91, 215, .35); }
  section > h1:hover, section > h2:hover, section > h3:hover, #title-slide :is(.title, .subtitle, .date):hover {
    outline: 2px dashed rgba(42, 91, 215, .55); outline-offset: 2px; cursor: text; }
  .qd-off { outline: 2px dashed #b3261e; outline-offset: 2px; }
  .qd-guide { position: absolute; z-index: 1001; pointer-events: none; background: #d6249f; }
  .qd-overlay { position: absolute; box-sizing: border-box; border: 2px solid #2a5bd7; cursor: move;
    touch-action: none; z-index: 1000; }
  /* a 10 px square to see, a 24 px target to hit (spec 008 US7: at least 20) */
  .qd-handle { position: absolute; --h: calc(24px * var(--px, 1)); width: var(--h); height: var(--h);
    margin: calc(var(--h) / -2) 0 0 calc(var(--h) / -2); }
  .qd-handle::after { content: ''; position: absolute; inset: calc(7px * var(--px, 1)); background: #fff;
    border: calc(2px * var(--px, 1)) solid #2a5bd7; border-radius: 2px; box-sizing: border-box; }
  section[data-qd-hidden]::after { content: "hidden slide (visibility=hidden): not in the rendered deck";
    position: absolute; top: 0; right: 0; font: 14px system-ui, sans-serif; padding: 2px 8px;
    background: #fff3c4; color: #5c4400; border-radius: 4px; }
}`;

function highlight() {
  const doc = frame.contentDocument;
  if (!doc) return;
  doc.querySelectorAll('.qd-sel').forEach((el) => el.classList.remove('qd-sel'));
  doc.querySelectorAll('.qd-overlay').forEach((el) => el.remove());
  if (state.sel) {
    const els = doc.querySelectorAll(`[data-qd="${CSS.escape(state.sel)}"]`);
    els.forEach((el) => el.classList.add('qd-sel'));
    const box = [...els].find((el) => el.classList.contains('absolute'));
    if (els.length) state.selBox = !!box; // kept while a new render swaps in (the arrow keys)
    if (box) attachOverlay(box);
  }
  placeElbar();
}

// ---------- placing an element freely (spec 009 US2) ----------

const revealScale = () => frameWin()?.Reveal?.getScale?.() || 1;

/** Where `el` shows on its slide now, [left, top, width, height] in slide px. */
function slideRect(el) {
  const s = revealScale();
  const sec = el.closest('section').getBoundingClientRect();
  const r = el.getBoundingClientRect();
  return [r.left - sec.left, r.top - sec.top, r.width, r.height].map((v) => Math.round(v / s));
}

/** `block place` args for target `t`: where its element shows on the slide now, in slide px. */
function placement(t) {
  const s = revealScale();
  const r = t.el.getBoundingClientRect();
  // a block is as wide as the slide; the box is as wide as its text (a little more, so it doesn't rewrap)
  let right = 0;
  const doc = t.el.ownerDocument;
  const walk = doc.createTreeWalker(t.el, NodeFilter.SHOW_TEXT);
  const range = doc.createRange();
  while (walk.nextNode()) {
    if (!walk.currentNode.data.trim()) continue;
    range.selectNodeContents(walk.currentNode);
    right = Math.max(right, range.getBoundingClientRect().right);
  }
  const w = right ? Math.min(r.width, right - r.left + 8 * s) : r.width;
  const [l, top] = slideRect(t.el);
  return { element: t.ref, at: `${l},${top}`, width: Math.round(w / s) };
}

/** ⌥-drag on an element in the flow: it follows the pointer, and the drop places it there. */
let placeDragged = false; // the click ending the drag is not an ⌥-click (source)
function placeDrag(e) {
  const el = e.target.closest('[data-qd]');
  if (!e.altKey || e.button !== 0 || !el || el.classList.contains('absolute') || state.session || presenting()) return;
  e.preventDefault(); // no text selection
  const doc = el.ownerDocument;
  const s = revealScale();
  const [x0, y0] = [e.clientX, e.clientY];
  let moved = false;
  const onMove = (ev) => {
    const [dx, dy] = [ev.clientX - x0, ev.clientY - y0];
    if (!moved && Math.hypot(dx, dy) < 4) return;
    moved = true;
    Object.assign(el.style, { transform: `translate(${dx / s}px, ${dy / s}px)`, opacity: '.6' });
  };
  const onUp = async () => {
    doc.removeEventListener('pointermove', onMove);
    doc.removeEventListener('pointerup', onUp);
    if (!moved) return;
    placeDragged = true;
    setTimeout(() => { placeDragged = false; }, 300); // in case no click follows
    const t = await elementTarget(el.dataset.qd);
    const why = t && actions.why('place', t);
    if (why) say(why, true);
    if (!t || why || !(await actions.run('place', t))) Object.assign(el.style, { transform: '', opacity: '' });
  };
  doc.addEventListener('pointermove', onMove);
  doc.addEventListener('pointerup', onUp);
}

/** A placed element outside the slide doesn't show when presenting: outline it. */
function markOffSlide(w) {
  const { width, height } = w.Reveal.getConfig();
  for (const b of w.document.querySelectorAll('.absolute[data-qd]')) {
    const r = rectOf(b);
    const off = r.l < 0 || r.t < 0 || r.l + r.w > width || r.t + r.h > height;
    b.classList.toggle('qd-off', off);
    b.title = off ? 'Outside the slide: it won’t show when presenting' : '';
  }
}

// ---------- guides and snapping (spec 009 US4) ----------

const SNAP_PX = 6; // screen px

/**
 * The lines a box dragged on `sec` can snap to, in slide px: the slide's edges and
 * centre and every other element's, measured once when the drag starts.
 */
function guideLines(sec, box) {
  const { width, height } = frameWin().Reveal.getConfig();
  const xs = [0, width / 2, width];
  const ys = [0, height / 2, height];
  for (const el of sec.querySelectorAll(':scope > [data-qd]')) {
    if (el === box) continue;
    const [l, t, w, h] = slideRect(el);
    xs.push(l, l + w / 2, l + w);
    ys.push(t, t + h / 2, t + h);
  }
  return { xs, ys, width, height };
}

/** The nearest line to any of `edges` within `reach`: [shift, line], or null. */
function nearest(edges, lines, reach) {
  let best = null;
  for (const e of edges) for (const x of lines) if (Math.abs(x - e) <= reach && (!best || Math.abs(x - e) < Math.abs(best[0]))) best = [x - e, x];
  return best;
}

const clearGuides = (sec) => sec.querySelectorAll('.qd-guide').forEach((g) => g.remove());

/** Snap rectangle `r` (moved, or resized from `dir`) to `lines`, drawing the guides it meets on `sec`. */
function snap(r, dir, lines, sec, reach) {
  const x = nearest(!dir ? [r.l, r.l + r.w / 2, r.l + r.w] : dir.includes('w') ? [r.l] : dir.includes('e') ? [r.l + r.w] : [], lines.xs, reach);
  const y = nearest(!dir ? [r.t, r.t + r.h / 2, r.t + r.h] : dir.includes('n') ? [r.t] : dir.includes('s') ? [r.t + r.h] : [], lines.ys, reach);
  if (x) {
    if (!dir) r.l += x[0]; else if (dir.includes('w')) { r.l += x[0]; r.w -= x[0]; } else r.w += x[0];
  }
  if (y) {
    if (!dir) r.t += y[0]; else if (dir.includes('n')) { r.t += y[0]; r.h -= y[0]; } else r.h += y[0];
  }
  for (const [hit, vertical] of [[x, true], [y, false]]) {
    if (!hit) continue;
    const g = sec.ownerDocument.createElement('div');
    g.className = 'qd-guide';
    const thin = reach / SNAP_PX + 'px'; // 1 screen px on the scaled slide
    Object.assign(g.style, vertical ? { left: hit[1] + 'px', top: 0, width: thin, height: lines.height + 'px' } : { top: hit[1] + 'px', left: 0, height: thin, width: lines.width + 'px' });
    sec.append(g);
  }
}

// ---------- drag / resize positioned boxes (US4) ----------

const HANDLES = { nw: [0, 0], n: [0.5, 0], ne: [1, 0], e: [1, 0.5], se: [1, 1], s: [0.5, 1], sw: [0, 1], w: [0, 0.5] };

function rectOf(el) {
  return { l: el.offsetLeft, t: el.offsetTop, w: el.offsetWidth, h: el.offsetHeight };
}

function place(el, r) {
  Object.assign(el.style, { left: r.l + 'px', top: r.t + 'px', width: r.w + 'px', height: r.h + 'px', right: 'auto', bottom: 'auto' });
}

function attachOverlay(box) {
  const doc = box.ownerDocument;
  const parent = box.offsetParent;
  if (!parent) return;
  const ov = doc.createElement('div');
  ov.className = 'qd-overlay';
  ov.title = 'Drag to move';
  // the slide is drawn scaled: size the handles in screen pixels
  ov.style.setProperty('--px', String(1 / (revealScale())));
  for (const [name, [x, y]] of Object.entries(HANDLES)) {
    const h = doc.createElement('div');
    h.className = 'qd-handle';
    h.dataset.dir = name;
    Object.assign(h.style, { left: x * 100 + '%', top: y * 100 + '%', cursor: name + '-resize' });
    ov.append(h);
  }
  place(ov, rectOf(box));
  parent.append(ov);

  ov.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    e.stopPropagation();
    const dir = e.target.dataset.dir || null;
    const scale = revealScale();
    const start = rectOf(box);
    const x0 = e.clientX;
    const y0 = e.clientY;
    let cur = start;
    ov.setPointerCapture(e.pointerId);
    const sec = box.closest('section');
    const lines = settings.get('placement.snap') && guideLines(sec, box);
    const onMove = (ev) => {
      const dx = (ev.clientX - x0) / scale;
      const dy = (ev.clientY - y0) / scale;
      const r = { ...start };
      if (!dir) { r.l += dx; r.t += dy; } else {
        if (dir.includes('e')) r.w = start.w + dx;
        if (dir.includes('s')) r.h = start.h + dy;
        if (dir.includes('w')) { r.w = start.w - dx; r.l = start.l + dx; }
        if (dir.includes('n')) { r.h = start.h - dy; r.t = start.t + dy; }
        if (ev.shiftKey && start.h > 0) {
          const ratio = start.w / start.h;
          if (dir === 'n' || dir === 's') r.w = r.h * ratio; else r.h = r.w / ratio;
          if (dir.includes('n')) r.t = start.t + start.h - r.h;
          if (dir.includes('w')) r.l = start.l + start.w - r.w;
        }
      }
      clearGuides(sec);
      // ⌘/Ctrl held: placed freely, this drag only; a ⇧ resize keeps its proportions instead
      if (lines && !ev.metaKey && !ev.ctrlKey && !(dir && ev.shiftKey)) snap(r, dir, lines, sec, SNAP_PX / scale);
      r.w = Math.max(1, r.w);
      r.h = Math.max(1, r.h);
      cur = { l: Math.round(r.l), t: Math.round(r.t), w: Math.round(r.w), h: Math.round(r.h) };
      place(box, cur);
      place(ov, cur);
    };
    const onUp = async () => {
      clearGuides(sec);
      ov.removeEventListener('pointermove', onMove);
      ov.removeEventListener('pointerup', onUp);
      ov.removeEventListener('pointercancel', onUp);
      const moved = cur.l !== start.l || cur.t !== start.t;
      const resized = cur.w !== start.w || cur.h !== start.h;
      if (!moved && !resized) return;
      const ref = state.sel;
      const r = resized
        ? await op('box resize', { box: ref, size: `${cur.w},${cur.h}`, at: `${cur.l},${cur.t}` })
        : await op('box move', { box: ref, at: `${cur.l},${cur.t}` });
      if (r) { await fetchDeck(); await renderPanel(); } else { place(box, start); place(ov, start); }
    };
    ov.addEventListener('pointermove', onMove);
    ov.addEventListener('pointerup', onUp);
    ov.addEventListener('pointercancel', onUp);
  });
}

// ---------- present (spec 008 US4) ----------
// The canvas's own frame goes full screen: the render already on screen, from the
// slide on screen, with reveal.js's keys and controls; leaving gives the editor them back.
// Escape belongs to the editor (leave an edit / deselect), not Reveal's overview;
// reveal.js's menu button, arrows and progress bar belong to presenting (US2)
const EDITING = { keyboard: { 27: null }, controls: false, progress: false, slideNumber: false };
let deckConfig = null; // the deck's own settings for those, read when the frame loads
const presenting = () => document.fullscreenElement === frame;
async function present(fromStart) {
  if (state.session) await saveSession();
  select(null);
  if (fromStart) frameWin()?.Reveal.slide(0, 0);
  // some hosts refuse full screen, and an app's embedded browser may never answer:
  // after a moment without it, say what to do instead
  const refused = () => { if (!presenting()) say('Full screen isn’t allowed here: ▾ ▸ Open in browser tab presents in a tab instead', true); };
  const late = setTimeout(refused, 1500);
  await frame.requestFullscreen().catch(refused).finally(() => clearTimeout(late));
}
document.addEventListener('fullscreenchange', () => {
  const R = frameWin()?.Reveal;
  if (!R || !deckConfig) return;
  frame.contentDocument.documentElement.classList.toggle('qd-present', presenting());
  R.configure(presenting() ? deckConfig : EDITING);
});

function hookFrame() {
  const w = frameWin();
  const doc = frame.contentDocument;
  if (!w || !doc) return;
  doc.addEventListener('selectionchange', () => { if (state.session) syncFocus(); });
  const style = doc.createElement('style');
  style.textContent = PREVIEW_CSS;
  doc.head.append(style);

  doc.addEventListener('click', (e) => {
    if (presenting()) return;
    if (placeDragged) { placeDragged = false; e.preventDefault(); e.stopPropagation(); return; }
    if (e.target.closest('.qd-overlay')) return; // the end of a box drag keeps the box selected
    if (state.session) {
      if (state.session.root.contains(e.target)) return; // caret moves inside the edit
      saveSession(); // clicking elsewhere ends the edit
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    const el = e.target.closest('[data-qd]');
    const target = editTarget(e.target);
    const sec = e.target.closest('section');
    if (!el && !target && !(e.altKey && sec)) { if (state.sel) select(null); return; }
    e.preventDefault();
    e.stopPropagation();
    // Alt+click: always the source (an element's, or the whole slide's)
    if (e.altKey) { (el ? beginSource(el) : beginSlideSource(sec)); return; }
    const point = { x: e.clientX, y: e.clientY };
    (target ? beginEdit(target, point) : Promise.resolve(false)).then((ok) => {
      if (ok || !el) return;
      // a positioned box selects for dragging; anything else opens its source
      if (el.classList.contains('absolute')) select(el.dataset.qd); else beginSource(el);
    });
  }, true);
  doc.addEventListener('pointerdown', placeDrag, true);
  doc.addEventListener('dblclick', (e) => {
    if (state.session || presenting()) return;
    const el = e.target.closest('[data-qd]') || doc.querySelector(`[data-qd="${CSS.escape(state.sel || '')}"]`);
    if (!el) return;
    e.preventDefault();
    // a positioned text box: single click selects it for dragging, double click types
    beginEdit({ kind: 'block', root: el }, { x: e.clientX, y: e.clientY }).then((ok) => {
      if (!ok) beginSource(el);
    });
  }, true);
  doc.addEventListener('keydown', onKey, true);
  // an element changing size (math typesetting, an image loading) moves its neighbours:
  // place the bars again, so they still cover nothing (spec 008 US2)
  const replace = new w.ResizeObserver(() => { placeElbar(); placeToolbar(frame); });
  for (const el of doc.querySelectorAll('[data-qd]')) replace.observe(el);
  doc.addEventListener('paste', onPaste);
  // an image file dropped on the slide (spec 008 US5): copied beside the deck, placed where it fell
  doc.addEventListener('dragover', (e) => {
    if (presenting() || !e.dataTransfer?.types.includes('Files')) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
  });
  doc.addEventListener('drop', (e) => {
    const file = [...(e.dataTransfer?.files || [])].find((f) => f.type.startsWith('image/'));
    if (!file || presenting()) return;
    e.preventDefault();
    dropImage(file, e.target.closest?.('[data-qd]'), e.clientY);
  });
  doc.addEventListener('contextmenu', (e) => {
    if (state.session || presenting()) return; // the browser's own menu (spelling) while typing
    const sec = e.target.closest('section');
    if (!sec) return;
    e.preventDefault();
    const f = frame.getBoundingClientRect();
    contextMenuAt(e.target.closest('[data-qd]'), sec, f.left + e.clientX, f.top + e.clientY);
  });
  doc.addEventListener('mousemove', (e) => hoverAt(e.target), { passive: true });
  doc.addEventListener('mousedown', () => closeMenu());

  const onReady = () => {
    // A narrow preview pane would put Reveal into its scroll view; the editor
    // always wants the normal slide view.
    // always: the pane may narrow later (panels, window), which would switch
    // Reveal into scroll view, where hash navigation to the last slide fails
    const { controls, progress, slideNumber } = w.Reveal.getConfig();
    deckConfig = { controls, progress, slideNumber, keyboard: true };
    w.Reveal.configure({ scrollActivationWidth: 1, ...EDITING });
    w.dispatchEvent(new Event('resize'));
    // synchronously: a hash change lands a task later, and onSlide() below would
    // meanwhile report the first slide (hiding the slide bar under a click)
    const id = state.want || state.anchor || location.hash.replace(/^#\//, '');
    const want = id && frame.contentDocument.getElementById(id);
    if (want && w.Reveal.getCurrentSlide() !== want) {
      const { h, v } = w.Reveal.getIndices(want);
      w.Reveal.slide(h, v);
    }
    state.want = null;
    if (state.selectIdAfterRender) {
      const el = frame.contentDocument.getElementById(state.selectIdAfterRender);
      state.selectIdAfterRender = null;
      if (el?.dataset.qd) state.sel = el.dataset.qd;
    }
    // only the render that has the inserted element: an older one still loading would
    // show other text at that ref, and the edit would not open
    if (state.editAfterRender?.version === state.loading) openInserted(state.editAfterRender);
    slideList.refresh();
    markOffSlide(w);
    w.MathJax?.startup?.promise?.then(() => slideList.refresh()); // math typesets after ready
    onSlide();
    frame.removeAttribute('aria-busy');
  };
  const onSlide = () => {
    const id = w.Reveal?.getCurrentSlide()?.id;
    if (!id || id === state.anchor) return;
    state.anchor = id;
    history.replaceState(null, '', '#/' + id);
    const s = slideByAnchor(id);
    if (state.sel && s && !state.sel.startsWith(s.id + '/')) state.sel = null;
    // a slide change that followed the focus isn't a new one (keeps the source pane's)
    if (state.followed === id) state.followed = null;
    else syncFocus();
    slideList.render();
    renderPanel();
    renderSlidebar();
  };
  if (w.Reveal?.isReady?.()) onReady();
  else w.Reveal?.on?.('ready', onReady);
  w.Reveal?.on?.('slidechanged', onSlide);
  highlight();
}

frame.addEventListener('load', () => {
  hookFrame();
  fetchDeck().then(renderPanel);
});

// ---------- slide list (spec 008 US1: list.js) ----------

// ---------- saved layouts (spec 009 US3) ----------

const LAYOUT_NAME = (x) => (/^[a-z0-9-]{1,40}$/.test(x) ? '' : 'Lowercase letters, digits and - (up to 40)');

/** `POST /api/layouts`: the new list, 'taken' for a name in use, or null (said). */
async function layoutChange(change) {
  const res = await fetch('/api/layouts', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-QD-Token': run.token }, body: JSON.stringify(change) });
  const body = await res.json().catch(() => ({}));
  if (res.status === 409) return 'taken';
  if (!res.ok) { say(res.status === 403 ? 'Saving layouts needs the editor link from `qd serve` or `qd url`' : body.error?.message || res.statusText, true); return null; }
  return body;
}

/** Slide ▸ Save slide as layout…: a name, where to keep it, and where each element shows now. */
async function saveLayout(t) {
  const v = await ask({
    title: 'Save slide as layout', ok: 'Save',
    fields: [
      { name: 'name', kind: 'text', label: 'Name', placeholder: 'e.g. chart-and-notes', validate: LAYOUT_NAME },
      { name: 'where', kind: 'choice', label: 'Keep it', value: 'deck', options: [['deck', 'With this deck'], ['mine', 'In my layouts, for every deck']] },
    ],
  });
  if (!v) return;
  const at = {};
  t.full.blocks.forEach((b, i) => {
    // where each element shows now: flow elements, and boxes without a full px place
    const el = elOf(`${t.slideId}/${i}`);
    const r = el && slideRect(el);
    if (r?.[2] > 0 && r[3] > 0) at[`${t.slideId}/${i}`] = r; // a slide not on screen measures 0: stacked instead
  });
  const save = { slide: t.slideId, name: v.name, where: v.where, at };
  let r = await layoutChange({ save });
  if (r === 'taken' && (await confirmSheet('Replace layout?', `There is already a layout named “${v.name}” there.`, 'Replace'))) r = await layoutChange({ save: { ...save, replace: true } });
  if (r && r !== 'taken') say(`Saved layout “${v.name}”`);
}

/** The gallery's rename and delete, for saved layouts. */
const manageLayouts = {
  rename: async (o) => {
    const v = await ask({ title: `Rename “${o.name}”`, ok: 'Rename', fields: [{ name: 'to', kind: 'text', label: 'New name', validate: LAYOUT_NAME }] });
    if (!v) return null;
    const r = await layoutChange({ rename: { from: o.name, to: v.to, where: o.from } });
    if (r === 'taken') say(`There is already a layout named “${v.to}”`, true);
    return r && r !== 'taken' ? v.to : null;
  },
  remove: async (o) => (await confirmSheet('Delete layout?', `“${o.name}” will be deleted. Slides made from it stay as they are.`, 'Delete'))
    && !!(await layoutChange({ rm: { name: o.name, where: o.from } })),
};

/** A new slide `where` (`{after}`/`{before}`), from a layout picked in the gallery (spec 009). */
async function addSlide(where, anchor) {
  const { layouts } = await fetch('/api/layouts').then((r) => r.json());
  const v = await ask({
    title: 'New slide', ok: 'Add slide', anchor,
    fields: [{ name: 'layout', kind: 'gallery', label: 'Layout', options: layouts, value: settings.get('ui.last_layout'), size: [state.deck.width, state.deck.height], manage: manageLayouts }],
  });
  if (!v?.layout) return null;
  settings.set({ 'ui.last_layout': v.layout });
  const r = await op('add', { ...where, layout: v.layout });
  if (r) { state.goAfterRender = r.target; state.sel = null; await fetchDeck(); }
  return r;
}

const slideList = makeList({
  state, frame, go, op, fetchDeck, openMenu, addSlide,
  slideMenu: async (id) => actions.slide(await slideTarget(id)),
});

// ---------- properties panel ----------

const BG = 'background-color';
const BG_IMG = 'background-image';
const TRANSITION = 'transition';

function blockOfBody(slide) {
  return slide.blocks.filter((b) => b.kind !== 'box').map((b) => b.text).join('\n\n');
}

async function renderPanel() {
  const cur = slideByAnchor(state.anchor);
  if (!cur) return renderTitlePanel();
  if (state.sel) return renderElementPanel();
  const slide = await fetchSlide(cur.id);
  if (!slide) return;
  const view = $('#slide-panel').content.cloneNode(true);
  const f = $('form.slide-form', view);
  const kv = slide.attrs?.kv || {};
  $('.ref', view).textContent = slide.id;
  f.title.value = slide.title || '';
  f.bg.value = kv[BG] || '';
  f.bgpick.value = /^#[0-9a-f]{6}$/i.test(kv[BG] || '') ? kv[BG] : '#ffffff';
  f.bgimage.value = kv[BG_IMG] || '';
  f.transition.value = kv[TRANSITION] || '';
  f.classes.value = (slide.attrs?.classes || []).join(' ');
  f.body.value = blockOfBody(slide);
  f.bgpick.addEventListener('input', () => { f.bg.value = f.bgpick.value; });
  f.addEventListener('reset', (e) => { e.preventDefault(); renderPanel(); });
  f.addEventListener('submit', (e) => { e.preventDefault(); applySlide(slide, f); });
  view.querySelectorAll('[data-act]').forEach((b) => b.addEventListener('click', () => slideAction(b.dataset.act, slide)));
  const addBox = $('form.add-box', view);
  addBox.addEventListener('submit', (e) => {
    e.preventDefault();
    const path = addBox.image.value.trim();
    if (path) newBox(slide, { image: path });
  });
  panel.replaceChildren(view);
}

async function newBox(slide, source) {
  const r = await op('box add', { slide: slide.id, ...source });
  if (r) { state.selectIdAfterRender = r.target; await fetchDeck(); }
}

async function renderTitlePanel() {
  const view = $('#title-panel').content.cloneNode(true);
  const f = $('form', view);
  const meta = await fetchMeta();
  const before = typeof meta.title === 'string' ? meta.title : '';
  f.title.value = before;
  f.addEventListener('submit', async (e) => {
    e.preventDefault();
    const t = f.title.value.trim();
    if (t === before) return;
    const r = t ? await op('meta set', { key: 'title', value: JSON.stringify(t) }) : await op('meta unset', { key: 'title' });
    if (r) renderPanel();
  });
  panel.replaceChildren(view);
}

async function applySlide(slide, f) {
  let target = slide.id;
  const title = f.title.value.trim();
  if (title !== (slide.title || '')) {
    const r = await op('title', { slide: target, text: title });
    if (!r) return;
    target = r.target;
  }
  const kv = slide.attrs?.kv || {};
  const attr = [];
  const unset = [];
  for (const [key, val] of [[BG, f.bg.value.trim()], [BG_IMG, f.bgimage.value.trim()], [TRANSITION, f.transition.value]]) {
    if (val && val !== kv[key]) attr.push(`${key}=${val}`);
    if (!val && kv[key] !== undefined) unset.push(key);
  }
  const before = new Set(slide.attrs?.classes || []);
  const after = new Set(f.classes.value.split(/\s+/).filter(Boolean));
  const cls = [...after].filter((c) => !before.has(c));
  const uncls = [...before].filter((c) => !after.has(c));
  if (attr.length || unset.length || cls.length || uncls.length) {
    const r = await op('set', { slide: target, attr, unset, class: cls, unclass: uncls });
    if (!r) return;
    target = r.target;
  }
  if (f.body.value.trim() !== blockOfBody(slide).trim()) {
    const r = await op('body', { slide: target, md: f.body.value });
    if (!r) return;
  }
  await fetchDeck();
  await renderPanel();
}

async function slideAction(act, slide) {
  const slides = state.deck.slides;
  const i = slides.findIndex((s) => s.id === slide.id);
  let res = null;
  if (act === 'textbox') return newBox(slide, { text: 'Text' });
  if (act === 'up' && i > 0) res = await op('move', { slide: slide.id, before: slides[i - 1].id });
  if (act === 'down' && i < slides.length - 1) res = await op('move', { slide: slide.id, after: slides[i + 1].id });
  if (act === 'dup' || act === 'rm') return actions.run(`slide-${act}`, await slideTarget(slide.id), 'slide');
  if (res) await fetchDeck();
}

async function select(ref) {
  state.sel = ref;
  highlight();
  syncFocus();
  if (ref) revealSource({ ref });
  await renderPanel();
}

async function renderElementPanel() {
  const ref = state.sel;
  const block = await fetchBlock(ref);
  if (!block) { state.sel = null; return renderPanel(); }
  const view = $('#element-panel').content.cloneNode(true);
  const f = $('form.element-form', view);
  $('.kind', view).textContent = block.kind[0].toUpperCase() + block.kind.slice(1);
  $('.ref', view).textContent = ref;
  f.md.value = block.text;
  f.addEventListener('submit', async (e) => {
    e.preventDefault();
    if (f.md.value.trim() === block.text.trim()) return;
    const r = await op('edit', { element: ref, md: f.md.value });
    if (r) { await fetchDeck(); await renderPanel(); }
  });
  f.md.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); f.requestSubmit(); }
  });
  $('[data-act=deselect]', view).addEventListener('click', () => select(null));
  if (block.kind === 'box') {
    const g = $('form.geom-form', view);
    g.hidden = false;
    const el = elOf(ref);
    const r = el ? rectOf(el) : null;
    const val = (k, measured) => (typeof block.geom?.[k] === 'number' ? block.geom[k] : measured ?? '');
    g.left.value = val('left', r?.l);
    g.top.value = val('top', r?.t);
    g.width.value = val('width', r?.w);
    g.height.value = val('height', r?.h);
    g.addEventListener('submit', async (e) => {
      e.preventDefault();
      const res = await op('box resize', { box: ref, size: `${g.width.value},${g.height.value}`, at: `${g.left.value},${g.top.value}` });
      if (res) { await fetchDeck(); await renderPanel(); }
    });
    $('[data-act=rmbox]', g).addEventListener('click', async () => {
      const res = await op('box rm', { box: ref });
      if (res) { await select(null); await fetchDeck(); }
    });
  }
  panel.replaceChildren(view);
}

// ---------- undo / redo / keys ----------

/** Slide anchor → the anchor of the slide it was duplicated from. */
const copiedFrom = new Map();
async function undoRedo(kind) {
  const was = state.deck?.slides || [];
  const r = await op(kind);
  if (!r) return;
  await fetchDeck();
  // the slide on screen went away (an add or a duplicate undone): show the one it was
  // copied from, else its neighbour, not the first slide (spec 008 US9)
  if (state.anchor && state.anchor !== 'title-slide' && !slideByAnchor(state.anchor)) {
    const i = was.findIndex((s) => s.anchor === state.anchor);
    const near = [copiedFrom.get(state.anchor), was[i - 1]?.anchor, was[i + 1]?.anchor].find((a) => a && slideByAnchor(a));
    if (near) { state.goAfterRender = near; go(near); }
  }
  await renderPanel();
}
/** File ▸ Render / Export…: one at a time each (the menu greys them out meanwhile). */
const busy = { render: false, export: false };
async function renderDeck() {
  busy.render = true;
  say('Rendering…');
  const link = $('#rendered');
  try {
    const r = await op('render');
    if (!r) return;
    say('Rendered → ' + r.output.split(/[\\/]/).pop());
    link.hidden = !r.href;
    if (r.href) {
      link.href = r.href + '?v=' + Date.now();
      link.title = r.output;
    }
  } finally {
    busy.render = false;
  }
}
// ---------- export (spec 002 US4) ----------

/** An export result in the side panel, opened for it even when the person keeps
 * the panel closed (008); "Back to slide" puts the panel back as it was. */
function showResult(view) {
  $('[data-act=close]', view).addEventListener('click', () => {
    layout('panel-open', 'ui.form_panel', !!settings.get('ui.form_panel'), false);
    renderPanel();
  });
  layout('panel-open', 'ui.form_panel', true, false);
  panel.replaceChildren(view);
}

function renderExportPanel(r) {
  const view = $('#export-panel').content.cloneNode(true);
  const status = $('.status', view);
  const check = r.check || { problems: [], ok: false };
  status.textContent = r.ok
    ? `Exported: ${check.slides} slides, works offline${r.pdf ? `; PDF ${r.pdf.pages} pages` : ''}.`
    : `Not exported: ${check.problems.filter((p) => p.severity === 'error').length} problem(s); the previous export is unchanged.`;
  status.className = 'status ' + (r.ok ? 'ok' : 'fail');
  const list = $('.problems', view);
  for (const p of check.problems) {
    const li = document.createElement('li');
    if (p.severity !== 'error') li.className = 'warn';
    const b = document.createElement('button');
    b.type = 'button';
    const code = document.createElement('span');
    code.className = 'code';
    code.textContent = p.code + (p.slide ? ` · ${p.slide}` : '');
    const msg = document.createElement('span');
    msg.textContent = ' ' + p.message;
    const hint = document.createElement('span');
    hint.className = 'phint';
    hint.textContent = p.hint;
    b.append(code, msg, hint);
    if (p.slide) b.addEventListener('click', () => { state.sel = null; go(p.slide); });
    else b.disabled = true;
    li.append(b);
    list.append(li);
  }
  const links = $('.links', view);
  for (const [href, label] of [[r.html_href, 'Open offline deck ↗'], [r.pdf_href, 'Open PDF ↗']]) {
    if (!href || !r.ok) continue; // a failed export wrote nothing there
    const a = document.createElement('a');
    a.href = href + '?v=' + Date.now();
    a.target = '_blank';
    a.rel = 'noopener';
    a.textContent = label;
    links.append(a, ' ');
  }
  showResult(view);
}

/** Another format (spec 011): the losses first, then the export, then what it kept. */
async function exportTo(to, label) {
  const dry = await op('export', { to, dry_run: true });
  if (!dry) return;
  const lines = dry.losses.map((l) => `• ${cap(l.what)}${l.slides.length ? ` (${l.slides.join(', ')})` : ''}`);
  const notes = dry.losses.some((l) => l.feature === 'notes');
  const message = (notes ? 'Speaker notes will not show in this format.\n\n' : '')
    + (lines.length ? 'What changes:\n' + lines.join('\n') : 'Everything on these slides carries over.');
  if (!(await confirmSheet(`Export ${label}?`, message, 'Export'))) return;
  busy.export = true;
  say(`Exporting ${label}…`);
  try {
    const r = await op('export', { to });
    if (r) renderFormatPanel(r, label);
  } finally {
    busy.export = false;
  }
}
const cap = (t) => t[0].toUpperCase() + t.slice(1);

function renderFormatPanel(r, label) {
  const view = $('#export-panel').content.cloneNode(true);
  const name = r.out.split(/[\\/]/).pop();
  $('.status', view).textContent = `Exported ${label}: ${name}` + (r.losses.length ? '. What changed:' : ', with everything kept.');
  $('.status', view).className = 'status ok';
  for (const l of r.losses) {
    const li = document.createElement('li');
    li.className = 'warn';
    li.append(cap(l.what) + (l.slides.length ? ': ' : '.'));
    for (const [i, s] of l.slides.entries()) {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'slide-link';
      b.textContent = s;
      b.addEventListener('click', () => { state.sel = null; go(s); });
      li.append(i ? ', ' : '', b);
    }
    $('.problems', view).append(li);
  }
  for (const n of r.notes) $('.problems', view).append(Object.assign(document.createElement('li'), { textContent: n }));
  if (r.href) {
    const a = Object.assign(document.createElement('a'), { href: r.href + '?v=' + Date.now(), download: name, textContent: `Download ${name} ↓`, title: r.out });
    $('.links', view).append(a);
  }
  $('.hint', view).textContent = `Saved as ${r.out}. Slide names go to that slide.`;
  showResult(view);
}

async function exportDeck(pdf) {
  busy.export = true;
  say(pdf ? 'Exporting (with PDF)…' : 'Exporting…');
  try {
    const r = await op('export', { pdf });
    if (r) renderExportPanel(r);
  } finally {
    busy.export = false;
  }
}

/** Tab / Shift+Tab in the preview: select the next / previous element of the slide (FR-017). */
function stepSelection(back) {
  const sec = frameWin()?.Reveal?.getCurrentSlide();
  const els = [...(sec?.querySelectorAll('[data-qd]') || [])].filter((el) => el.closest('section') === sec);
  if (!els.length) return false;
  const i = els.findIndex((el) => el.dataset.qd === state.sel);
  const next = els[i < 0 ? (back ? els.length - 1 : 0) : (i + (back ? -1 : 1) + els.length) % els.length];
  select(next.dataset.qd);
  next.scrollIntoView({ block: 'nearest' });
  fetchBlock(next.dataset.qd).then((b) => b && say(`Selected ${b.kind} ${next.dataset.qd} — Enter edits, ⇧F10 for actions`));
  return true;
}

function onKey(e) {
  if (presenting()) return; // reveal.js's keys while presenting
  // widget key: Esc closes an open menu even when focus stayed elsewhere (the preview)
  // (and only that: an edit under a menu opened with the mouse stays open, one level per Esc)
  if (menuOpen()) { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); closeMenu(); } return; }
  if (state.session || editing()) return; // edits and form fields handle their own keys
  // every other key is an action from the list (spec 007): the selected element first, then the slide
  keys.dispatch(e, state.sel ? ['element', 'slide'] : [e.target.closest?.('nav') ? 'slides' : 'slide']);
}
document.addEventListener('keydown', onKey);

// ---------- live updates ----------

const es = new EventSource(`/events?tab=${focus.tabId}`);
run.listen(es);
settings.listen(es);
es.addEventListener('open', () => status('Connected', true));
es.addEventListener('error', () => status('Disconnected — is qd serve running?'));
es.addEventListener('changed', async (e) => {
  const { version } = JSON.parse(e.data);
  if (version !== state.version) {
    status('Changed on disk', true);
    slides = { version: null, map: new Map() };
    await fetchDeck();
    if (!editing()) await renderPanel();
  }
  pane.changed();
});
// an agent points this tab at a slide or element (spec 005 US2)
const offer = $('#gooffer');
let offered = null;
function showPoint({ anchor, ref }) {
  offer.hidden = true;
  // where an agent points isn't the person's own focus
  if (anchor !== state.anchor) { state.followed = anchor; go(anchor); }
  if (!ref.includes('/')) return revealSource({ slide: ref });
  revealSource({ ref });
  // flash it (not select it); every slide stays in the DOM, so no need to wait
  for (const el of frame.contentDocument?.querySelectorAll(`[data-qd="${CSS.escape(ref)}"]`) || []) {
    el.animate([{ outline: '3px solid #f0a020', outlineOffset: '4px' }, {}], 1500);
  }
}
es.addEventListener('point', (e) => {
  const p = JSON.parse(e.data);
  if (p.tab !== focus.tabId) return;
  if (!focus.isEditing()) return showPoint(p);
  // never while the person types (FR-008): offer it instead
  offered = p;
  const s = slideByAnchor(p.anchor);
  offer.querySelector('.msg').textContent = `An agent points at ${s?.title ? `"${s.title}"` : 'a slide'}${p.ref.includes('/') ? ' (an element)' : ''}.`;
  offer.hidden = false;
});
offer.addEventListener('click', (e) => {
  const act = e.target.closest('button')?.dataset.act;
  if (act === 'go' && offered) showPoint(offered);
  if (act) offer.hidden = true;
});
es.addEventListener('rendering', () => status('Rendering…'));
es.addEventListener('rendered', (e) => {
  const d = JSON.parse(e.data);
  status('Up to date', true);
  statusEl.title = `Version ${d.version.slice(0, 7)}`;
  banner.hidden = true;
  if (state.session) {
    // reloading now would throw away the open edit (research R4)
    state.pendingRender = d;
    pendingBadge(true);
  } else {
    load(d.html, d.version);
  }
});
es.addEventListener('render_error', (e) => {
  const d = JSON.parse(e.data);
  status('Render failed — showing the last good one');
  $('.msg', banner).textContent = d.message;
  banner.hidden = false;
});

// ---------- on-slide editing (spec 003 US1) ----------

const NOT_TYPEABLE = new Set(['code', 'math', 'html', 'table', 'image', 'notes', 'pause', 'raw']);
const META_FIELDS = [['title', '.title'], ['subtitle', '.subtitle'], ['author', '.quarto-title-author-name'], ['date', '.date']];

function pendingBadge(on) {
  let b = $('#pending');
  if (!b) {
    b = Object.assign(document.createElement('span'), { id: 'pending' });
    statusEl.after(b);
  }
  b.textContent = on ? 'updated — will refresh when you finish editing' : '';
}

/** What a click on `node` would edit: a block, a slide title, or a title-slide field. */
function editTarget(node) {
  const el = node.closest?.('[data-qd]');
  // a positioned box selects for dragging, but a layout's empty text box is for typing into
  if (el) return el.classList.contains('absolute') && !el.matches('.qd-ph[data-ph=text]') ? null : { kind: 'block', root: el };
  const sec = node.closest?.('section');
  if (!sec) return null;
  if (sec.id === 'title-slide') {
    for (const [key, sel] of META_FIELDS) {
      const f = node.closest(sel);
      if (f && sec.contains(f)) return { kind: 'meta', key, root: f };
    }
    return null;
  }
  const h = node.closest('h1, h2, h3');
  if (h && h.parentElement === sec) return { kind: 'title', root: h, anchor: sec.id };
  return null;
}

/** Start typing into `target` if its rendering round-trips to its source. */
async function beginEdit(target, point) {
  let s = null;
  if (target.kind === 'block') {
    const ref = target.root.dataset.qd;
    const block = await fetchBlock(ref);
    if (!block || NOT_TYPEABLE.has(block.kind)) return false;
    const reg = region(block.kind, block.text);
    if (!reg) return false;
    s = { ...target, ref, source: block.text, hash: block.hash, reg, hints: hintsFor(block.text) };
  } else if (target.kind === 'title') {
    const slide = slideByAnchor(target.anchor);
    const full = slide && (await fetchSlide(slide.id));
    if (!full) return false;
    // an untitled slide starts from an empty title (its placeholder is the canvas's only)
    const title = full.title ?? '';
    s = { ...target, slide: full.id, source: title, hash: full.title_hash ?? undefined,
      reg: { mode: 'inline', prefix: '', inner: title, suffix: '' }, hints: {} };
  } else {
    const value = (await fetchMeta())[target.key];
    if (typeof value !== 'string') return false;
    s = { ...target, source: value, reg: { mode: 'inline', prefix: '', inner: value, suffix: '' }, hints: {} };
  }
  const g = gate(s.root, s.reg, s.hints);
  if (!g) return false;
  if (state.session) return false; // another edit started meanwhile
  s.mode = 'typed';
  Object.assign(s, g, { html: s.root.innerHTML, align: null });
  beginSession(s, point);
  return true;
}

/** Common start of a typing or source session. */
function openSession(s) {
  hoverbar.hidden = true;
  emptyadd.hidden = true;
  state.session = s;
  state.sel = null;
  highlight();
  syncFocus();
  s.root.classList.add('qd-editing');
  revealSource(s);
}

function beginSession(s, point) {
  const doc = s.root.ownerDocument;
  openSession(s);
  s.root.contentEditable = 'true';
  // an empty box (a layout's placeholder) gets a paragraph to type into: the
  // browser would put the text straight into the box, which isn't markdown
  if (s.kind === 'block' && !s.root.children.length && !s.root.textContent.trim()) {
    s.root.innerHTML = '<p><br></p>';
    point = null;
  }
  doc.execCommand('styleWithCSS', false, false);
  doc.execCommand('defaultParagraphSeparator', false, 'p');
  frame.focus(); // keyboard input must reach the iframe (e.g. after a menu in the page had focus)
  s.root.focus();
  const r = point && doc.caretRangeFromPoint?.(point.x, point.y);
  if (r && s.root.contains(r.startContainer)) {
    const sel = doc.getSelection();
    sel.removeAllRanges();
    sel.addRange(r);
  }
  s.ac = new AbortController(); // removes all of these listeners in endSession
  const on = (type, fn) => s.root.addEventListener(type, fn, { signal: s.ac.signal });
  on('keydown', (e) => sessionKey(e, s));
  on('focusout', () => { if (!s.hold && !s.failed) setTimeout(() => { if (state.session === s && !s.root.contains(doc.activeElement)) saveSession(); }, 0); });
  on('paste', (e) => {
    e.preventDefault(); // plain text only (formatting from other apps is dropped)
    doc.execCommand('insertText', false, e.clipboardData.getData('text/plain'));
  });
  // text dragged in from elsewhere comes in as plain text where it was dropped, like a
  // paste (the browser would insert its markup, which markdown here can't hold)
  on('drop', (e) => {
    if (e.dataTransfer?.types.includes('Files')) return; // an image file: the canvas's drop
    e.preventDefault();
    e.stopPropagation();
    const text = e.dataTransfer?.getData('text/plain');
    const at = doc.caretRangeFromPoint?.(e.clientX, e.clientY);
    if (!text || !at || !s.root.contains(at.startContainer)) return;
    doc.getSelection().removeAllRanges();
    doc.getSelection().addRange(at);
    doc.execCommand('insertText', false, text);
  });
  on('input', () => { s.failed = false; placeToolbar(frame); });
  showToolbar(s, frame);
  say(s.reg.mode === 'inline' ? 'Editing — Enter, Esc or click elsewhere saves' : 'Editing — ⌘Enter, Tab, Esc or click elsewhere saves');
}

function sessionKey(e, s) {
  e.stopPropagation(); // keep Reveal and the editor's own keys out of the edit
  // Escape saves, like clicking elsewhere (spec 007); after a refused save it discards
  if (e.key === 'Escape') { e.preventDefault(); if (s.failed) { endSession(s, false); if (s.ref) select(s.ref); } else saveSession().then(() => { if (!state.session && s.ref) select(s.ref); }); return; }
  if (e.key === 'Enter' && e.altKey && s.kind === 'block') {
    // switch to the source, carrying over what was typed so far
    e.preventDefault();
    const draft = draftSource(s) ?? s.source;
    endSession(s, false, { keepPending: true });
    beginSource(s.root, draft, s);
    return;
  }
  const mod = e.metaKey || e.ctrlKey;
  if ((e.key === 'Enter' && (mod || s.reg.mode === 'inline')) || (e.key === 'Tab' && !inList(s))) {
    e.preventDefault();
    saveSession();
    return;
  }
  if (e.key === 'Tab') { // indent/outdent inside a list
    e.preventDefault();
    s.root.ownerDocument.execCommand(e.shiftKey ? 'outdent' : 'indent');
    return;
  }
  keys.dispatch(e, ['text']); // formatting keys while typing (the action list)
}

function inList(s) {
  const sel = s.root.ownerDocument.getSelection();
  const n = sel?.anchorNode;
  return !!(n && (n.nodeType === 1 ? n : n.parentElement)?.closest('li'));
}

/** What the serializer refused (`IMG with attributes`, `span`…), in words. */
function cantWrite(why = '') {
  const tag = why.split(' ')[0];
  const names = { IMG: 'an image', TABLE: 'a table', SVG: 'a drawing', IFRAME: 'an embedded page', VIDEO: 'a video', span: 'styling', div: 'a block', FONT: 'a font' };
  return names[tag] || (/^[A-Z][A-Z0-9]*$/.test(tag) ? `a <${tag.toLowerCase()}> element` : 'formatting');
}

/** Markdown the session would write, or null if the edit can't be expressed. */
function draftSource(s) {
  let S1;
  try { S1 = reserialize(s.root, s.reg, s.hints); } catch (e) { s.why = e.message; return null; }
  let inner = splice(s.reg.inner, s.S0, S1, s.pos);
  let { prefix, suffix } = s.reg;
  if (s.align) {
    const fenced = prefix.match(/^(\s*:{3,})\s*\{style="text-align:\s*\w+;?"\}\s*(\r?\n)$/);
    if (fenced) {
      if (s.align === 'left') { prefix = ''; suffix = ''; } else prefix = `${fenced[1]} {style="text-align:${s.align}"}${fenced[2]}`;
    } else if (!prefix && s.align !== 'left' && s.kind === 'block') {
      const nl = s.source.includes('\r\n') ? '\r\n' : '\n';
      prefix = `::: {style="text-align:${s.align}"}${nl}`;
      suffix = `${nl}:::`;
    }
  }
  return prefix + inner + suffix;
}

async function saveSession() {
  const s = state.session;
  if (!s || s.saving) return;
  if (s.mode === 'source') return saveSource(s);
  const md = draftSource(s);
  if (md === null) {
    s.failed = true;
    say(`Part of this edit (${cantWrite(s.why)}) has no markdown form here. ⌘Z undoes it; Esc again discards the whole edit.`, true);
    return;
  }
  if (md === s.source) { endSession(s, false); return; }
  s.saving = true;
  let r;
  if (s.kind === 'block') r = await op('edit', { element: s.ref, md, 'expect-text': s.hash }, { fileExpect: false });
  else if (s.kind === 'title') r = await op('title', { slide: s.slide, text: md, 'expect-text': s.hash }, { fileExpect: false });
  else r = await op('meta set', { key: s.key, value: JSON.stringify(md) }, { fileExpect: false });
  s.saving = false;
  if (!r) {
    if (op.lastError?.code === 'E_CONFLICT') { await conflict(s, md, op.lastError.details); return; }
    s.failed = true;
    s.root.focus();
    return;
  }
  savedNote(s.root);
  endSession(s, true);
  await fetchDeck();
}

/** Close the edit; `saved` keeps the typed text visible until the new render loads. */
function endSession(s, saved, { keepPending = false } = {}) {
  if (state.session !== s) return;
  s.root.classList.remove('qd-editing');
  if (s.mode === 'source') {
    closeSource();
  } else {
    s.ac.abort();
    s.root.removeAttribute('contenteditable');
    if (!saved) { s.root.innerHTML = s.html; s.root.style.textAlign = ''; }
    s.root.blur();
    hideToolbar();
  }
  state.session = null;
  syncFocus();
  if (state.pendingRender && !keepPending) {
    const { html, version } = state.pendingRender;
    state.pendingRender = null;
    pendingBadge(false);
    load(html, version);
  }
}

// ---------- source editing in place (spec 003 US2) ----------

/** "Saved" beside an element for a moment after its edit lands (spec 008 US9). */
function savedNote(el) {
  const p = pageRect(el);
  const n = document.body.appendChild(Object.assign(document.createElement('div'), { className: 'saved-note', textContent: 'Saved', role: 'status' }));
  n.style.left = Math.max(4, Math.min(p.right + 6, innerWidth - n.offsetWidth - 4)) + 'px';
  n.style.top = p.top + 'px';
  setTimeout(() => n.remove(), 1500);
}

function pageRect(el) {
  const f = frame.getBoundingClientRect();
  const r = el.getBoundingClientRect();
  return { left: f.left + r.left, top: f.top + r.top, width: r.width, height: r.height };
}

function startSourceSession(s, text) {
  openSession(s);
  openSource(pageRect(s.root), text, { save: () => saveSession(), cancel: () => endSession(s, false) });
  say('Editing source — ⌘Enter, Esc or click elsewhere saves; Cancel discards');
}

/** Edit one element's markdown over it; `draft` and `base` carry over a typing session. */
async function beginSource(el, draft = null, base = null) {
  if (state.session) return false;
  const ref = el.dataset.qd;
  let source = base?.source;
  let hash = base?.hash;
  if (!base) {
    const block = await fetchBlock(ref);
    if (!block || state.session) return false;
    source = block.text;
    hash = block.hash;
  }
  startSourceSession({ kind: 'block', mode: 'source', root: el, ref, source, hash }, draft ?? source);
  return true;
}

/** Edit a whole slide's content (positioned boxes are kept, as with `qd body`). */
async function beginSlideSource(sec, draft = null) {
  if (state.session || !sec) return false;
  const cur = slideByAnchor(sec.id);
  const slide = cur && (await fetchSlide(cur.id));
  if (!slide || slide.opaque || state.session) return false;
  await fetchDeck(); // the save checks the whole file against this version
  const text = blockOfBody(slide);
  startSourceSession({ kind: 'slide', mode: 'source', root: sec, slide: slide.id, source: text }, draft ?? text);
  return true;
}

// ---------- conflicts with edits made elsewhere (spec 003 US5) ----------

const conflictBar = document.createElement('div');
conflictBar.id = 'conflict';
conflictBar.setAttribute('role', 'alert');
conflictBar.hidden = true;
conflictBar.innerHTML = '<strong>This was changed elsewhere while you were editing it.</strong>'
  + '<div class="cols"><div><span class="lbl">Now in the file</span><pre class="theirs"></pre></div>'
  + '<div><span class="lbl">Your version</span><pre class="mine"></pre></div></div>'
  + '<div class="buttons"><button type="button" data-act="mine" class="primary">Use mine…</button>'
  + '<button type="button" data-act="theirs">Keep theirs</button></div>';
banner.after(conflictBar);

function showConflict(c) {
  state.conflict = c;
  $('.theirs', conflictBar).textContent = c.theirs || '(see the slide)';
  $('.mine', conflictBar).textContent = c.mine;
  conflictBar.hidden = false;
}

/**
 * A save was refused because the element changed meanwhile: close the edit
 * (the file keeps the other version) and let the person choose (FR-015).
 */
async function conflict(s, mine, details) {
  let theirs = details?.current_source;
  if (theirs === undefined && s.kind === 'slide') theirs = blockOfBody((await fetchSlide(s.slide)) || { blocks: [] });
  endSession(s, false);
  showConflict({ kind: s.kind, ref: s.ref, slide: s.slide, anchor: s.root.closest?.('section')?.id || s.anchor, mine, theirs: theirs ?? '', hash: details?.current_hash });
  await fetchDeck();
}

conflictBar.addEventListener('click', async (e) => {
  const act = e.target.dataset?.act;
  const c = state.conflict;
  if (!act || !c) return;
  conflictBar.hidden = true;
  state.conflict = null;
  if (c.kind === 'source') { (act === 'theirs' ? c.onTheirs : c.onMine)(); return; }
  if (act === 'theirs') { say('Kept the version in the file'); return; }
  const doc = frame.contentDocument;
  if (c.kind === 'block') {
    const el = doc?.querySelector(`[data-qd="${CSS.escape(c.ref)}"]`);
    if (el && (await beginSource(el, c.mine, { source: c.theirs, hash: c.hash }))) return;
  } else if (c.kind === 'slide') {
    if (await beginSlideSource(doc?.getElementById(c.anchor), c.mine)) return;
  } else if (c.kind === 'title') {
    if (await op('title', { slide: c.slide, text: c.mine, 'expect-text': c.hash }, { fileExpect: false })) { await fetchDeck(); return; }
  }
  // the element is gone (or changed again): keep the text safe on the clipboard
  try { await navigator.clipboard.writeText(c.mine); } catch { /* not allowed */ }
  say('Could not reopen that element — your version is on the clipboard', true);
});

async function saveSource(s) {
  const md = sourceText();
  if (md.trim() === s.source.trim()) { endSession(s, false); return; }
  s.saving = true;
  const r = s.kind === 'slide'
    ? await op('body', { slide: s.slide, md })
    : await op('edit', { element: s.ref, md, 'expect-text': s.hash }, { fileExpect: false });
  s.saving = false;
  if (!r) {
    if (op.lastError?.code === 'E_CONFLICT') { await conflict(s, md, op.lastError.details); return; }
    sourceError(op.lastError?.message || 'not saved');
    return;
  }
  savedNote(s.root);
  endSession(s, true);
  await fetchDeck();
}

// test/automation hook (tests/onslide.rs): read-only view of the open edit
window.qdSession = () => {
  const s = state.session;
  if (!s) return null;
  return { kind: s.kind, mode: s.mode || 'typed', ref: s.ref || null, draft: s.mode === 'source' ? sourceText() : draftSource(s) };
};

// ---------- menus, bars, insert, drag, clipboard (spec 003 US3/US4/US6) ----------

/** What actions act on: an element (`<slide>/<i>`) … */
async function elementTarget(ref) {
  const cut = ref.lastIndexOf('/');
  const slideId = ref.slice(0, cut);
  const slide = await fetchSlide(slideId);
  const index = Number(ref.slice(cut + 1));
  const block = slide?.blocks[index];
  if (!block) return null;
  const el = elOf(ref);
  return { ref, block, slideId, index, count: slide.blocks.length, kinds: slide.blocks.map((b) => b.kind), el };
}

/** … or a slide. */
async function slideTarget(slideId) {
  const listed = slideById(slideId);
  const full = await fetchSlide(slideId);
  return { slideId, anchor: listed?.anchor || slideId, title: full?.title, full };
}

const actions = makeActions({
  runCells: run.runCells,
  addSlide,
  op: async (name, args) => {
    if (name === 'rm') {
      const many = slideList.selected(args.slide); // the picked slides, if this is one (spec 008 US9)
      if (many.length > 1) args = { ...args, slide: many };
      const slides = state.deck?.slides || [];
      const i = slides.findIndex((x) => x.id === many[0]);
      const near = [...slides.slice(0, i).reverse(), ...slides.slice(i + 1)].find((x) => !many.includes(x.id));
      if (near) state.goAfterRender = near.anchor;
    }
    const r = await op(name, args);
    if (!r) return null;
    if (name === 'dup') { state.goAfterRender = r.target; copiedFrom.set(r.target, slideById(args.slide)?.anchor); }
    if (/^(block|box) /.test(name) && name !== 'block rm' && r.target?.includes('/')) state.sel = r.target; // the element at its new place
    if (name === 'block rm' || name === 'rm') state.sel = null;
    if (name === 'block place') state.sel = args.element; // the box is where the element was
    await fetchDeck();
    await renderPanel();
    renderSlidebar();
    return r;
  },
  placement,
  saveLayout,
  beginEditRef: (t) => t.el && beginEdit({ kind: 'block', root: t.el }, null).then((ok) => ok || beginSource(t.el)),
  beginSourceEl: (t) => t.el && beginSource(t.el),
  beginSlideSource: (t) => beginSlideSource(frame.contentDocument?.getElementById(t.anchor)),
  editNotes,
  insert: insertBlock,
  copy: copyElement,
  paste: pasteAfter,
  copySlide,
  pasteSlides,
  say,
  slideUrl: (anchor) => `${location.origin}/#/${anchor}`,
});

/** Right-click in the preview: the element's menu, or the slide's (empty area). */
async function contextMenuAt(el, sec, x, y) {
  if (el) {
    await select(el.dataset.qd);
    const t = await elementTarget(el.dataset.qd);
    if (t) openMenu(actions.element(t), x, y);
    return;
  }
  const cur = slideByAnchor(sec.id);
  if (!cur) return; // the title slide has no slide-level actions
  openMenu(actions.empty(await slideTarget(cur.id)), x, y);
}

/** Shift+F10 / ContextMenu key: menu for the selected element, else the slide. */
async function keyboardMenu() {
  const el = state.sel && elOf(state.sel);
  if (el) {
    const p = pageRect(el);
    const t = await elementTarget(state.sel);
    if (t) openMenu(actions.element(t), p.left, p.top + p.height);
    return;
  }
  const cur = slideByAnchor(state.anchor);
  const f = frame.getBoundingClientRect();
  if (cur) openMenu(actions.empty(await slideTarget(cur.id)), f.left + 40, f.top + 40);
}

// element bar: the common actions for the selected element (US3)
const elbar = document.createElement('div');
elbar.id = 'elbar';
elbar.setAttribute('role', 'toolbar');
elbar.setAttribute('aria-label', 'Element');
elbar.hidden = true;
for (const [id, label, title] of [['run', '▶ Run', 'Run this code cell'], ['edit', 'Edit', 'Edit text (Enter)'], ['source', 'Markdown', 'Edit markdown (⌥Enter)'],
  ['fragment', 'Appears on click', 'Appears on click (a fragment)'], ['delete', 'Delete', 'Delete (⌫)'], ['more', '⋯', 'More (right-click, ⇧F10)']]) {
  const b = Object.assign(document.createElement('button'), { type: 'button', textContent: label, title });
  b.dataset.act = id;
  b.setAttribute('aria-label', title);
  elbar.append(b);
}
document.body.append(elbar);
elbar.addEventListener('click', async (e) => {
  const id = e.target.closest('button')?.dataset.act;
  const t = id && state.sel && (await elementTarget(state.sel));
  if (!t) return;
  if (id === 'more') { const r = e.target.getBoundingClientRect(); openMenu(actions.element(t), r.left, r.bottom + 4); return; }
  actions.run(id, t);
});

async function placeElbar() {
  const el = !state.session && state.sel && elOf(state.sel);
  if (!el) { elbar.hidden = true; return; }
  const t = await elementTarget(state.sel);
  $('[data-act=run]', elbar).hidden = t?.block.kind !== 'cell';
  $('[data-act=fragment]', elbar).setAttribute('aria-pressed', String(!!t?.block.attrs?.classes?.includes('fragment')));
  elbar.hidden = false;
  placeNear(elbar, el, frame);
}

// the preview resized (source pane, panels, window): follow the element, or the bar
// is left floating over whatever is there now. After reveal.js has laid out again.
new ResizeObserver(() => requestAnimationFrame(() => requestAnimationFrame(placeElbar))).observe(frame);

// slide bar: the slide's common properties (US3)
const slidebar = $('#slidebar');
async function renderSlidebar() {
  const cur = slideByAnchor(state.anchor);
  // the title slide has no slide properties: the bar keeps its row (the canvas
  // doesn't jump, spec 008 US2) but shows nothing and can't be reached (hidden)
  slidebar.style.visibility = cur ? '' : 'hidden';
  if (!cur) return;
  const full = await fetchSlide(cur.id);
  const kv = full?.attrs?.kv || {};
  $('[data-act=bg] .chip', slidebar).style.background = kv['background-color'] || '';
  slidebar.querySelector('[name=transition]').value = kv.transition || '';
  slidebar.querySelector('[name=hidden]').checked = kv.visibility === 'hidden';
  placeEmptyadd();
}
const setSlideAttr = (key, value) => {
  const cur = slideByAnchor(state.anchor);
  if (cur) actions.setAttr({ slideId: cur.id }, key, value);
};
slidebar.querySelector('[name=transition]').addEventListener('change', (e) => setSlideAttr('transition', e.target.value));
slidebar.querySelector('[name=hidden]').addEventListener('change', (e) => setSlideAttr('visibility', e.target.checked ? 'hidden' : null));
slidebar.addEventListener('click', async (e) => {
  const act = e.target.closest('button')?.dataset.act;
  const cur = slideByAnchor(state.anchor);
  if (!act || !cur) return;
  const t = await slideTarget(cur.id);
  if (act === 'bg') actions.run('slide-bg', t, 'slide');
  else if (act === 'bg-image') actions.run('slide-bgimg', t, 'slide');
  else if (act === 'more') { const r = e.target.getBoundingClientRect(); openMenu(actions.slide(t), r.left, r.bottom + 4); }
});

// layout: source pane (which side, open, width, full width) and the old form panel,
// remembered in the person's settings (spec 007), so the same in every deck and in VS Code
const mainEl = document.querySelector('main');
const srcpane = $('#srcpane');
/** Toggle a layout class on <main>; remember it unless applying settings. */
function layout(cls, key, on, remember = true) {
  mainEl.classList.toggle(cls, on);
  if (remember && settings.get(key) !== on) settings.set({ [key]: on });
}
const setPanel = (on) => layout('panel-open', 'ui.form_panel', on);
const setFull = (on) => { $('[data-act=full]', srcpane).setAttribute('aria-pressed', String(on)); layout('source-full', 'ui.source_full', on); };
function setSource(open, remember) {
  layout('source-open', 'ui.source_was_open', open, remember);
  srcpane.hidden = !open;
  if (open) pane.load();
}
const splitter = $('#splitter');
let widthTimer = null;
const setWidth = (px, remember = true) => {
  mainEl.style.setProperty('--srcw', Math.max(240, Math.min(px, innerWidth - 320)) + 'px');
  clearTimeout(widthTimer); // one write once the drag pauses
  if (remember) widthTimer = setTimeout(() => settings.set({ 'ui.source_width': Math.round(px) }), 300);
};
const onRight = () => mainEl.classList.contains('source-right');
splitter.addEventListener('pointerdown', (e) => {
  splitter.setPointerCapture(e.pointerId);
  const move = (ev) => {
    const r = srcpane.getBoundingClientRect();
    setWidth(onRight() ? r.right - ev.clientX : ev.clientX - r.left);
  };
  splitter.addEventListener('pointermove', move);
  splitter.addEventListener('pointerup', () => splitter.removeEventListener('pointermove', move), { once: true });
});
splitter.addEventListener('keydown', (e) => {
  const d = { ArrowLeft: -20, ArrowRight: 20 }[e.key];
  if (d) { e.preventDefault(); setWidth(srcpane.getBoundingClientRect().width + (onRight() ? -d : d)); }
});

// the person's settings: the layout once, then everything live (here, another editor, `qd settings`)
const themed = new URLSearchParams(location.search).has('theme'); // VS Code sets the theme (spec 006)
settings.onChange((v) => {
  mainEl.classList.toggle('source-right', v['source.side'] === 'right');
  if (!themed) {
    if (v.theme === 'system') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = v.theme;
  }
  pane.setAutosave(v['source.save'] === 'pause');
  for (const el of document.querySelectorAll('[data-key]')) el.title = v['hints.shortcuts'] ? `${el.dataset.hint} (${el.dataset.key})` : el.dataset.hint;
});
// the saved layout before any button works: arriving late, it would undo a click
await settings.ready((n) => say(n, true)).catch(() => say('settings could not be loaded; using the defaults', true));
{
  const v = settings.get;
  if (v('ui.source_width')) setWidth(v('ui.source_width'), false);
  setPanel(v('ui.form_panel'));
  setFull(v('ui.source_full'));
  setSource(v('source.open') === 'open' || (v('source.open') === 'last' && v('ui.source_was_open')), false);
}
$('[data-act=form]', srcpane).addEventListener('click', () => setPanel(!mainEl.classList.contains('panel-open')));
$('[data-act=full]', srcpane).addEventListener('click', () => setFull(!mainEl.classList.contains('source-full')));


pane.initPane({
  op: (name, args) => op(name, args, { fileExpect: false }),
  lastError: () => op.lastError,
  conflict: (theirs, mine, onMine, onTheirs) => showConflict({ kind: 'source', theirs, mine, onMine, onTheirs }),
  onCaret: sourceFocus,
  typed: focus.typed,
});

/** The slide view's part of the shared focus (spec 005): slide, element, text selected while editing. */
function syncFocus() {
  seq++; // a source report still fetching is now older than this
  const text = state.session ? String(frameWin()?.getSelection?.() || '') : '';
  const ref = state.sel || state.session?.ref;
  focus.editingSlide(!!state.session);
  focus.set({
    slide: slideByAnchor(state.anchor)?.id ?? null,
    elements: ref ? [ref] : [],
    line: null,
    selection: text ? { text, where: 'slide' } : null,
  });
}

/** The source pane's part: the caret's slide, every block the selection touches (or the caret's). */
let sourceTimer = null;
let seq = 0;
function sourceFocus({ line, first, last, text }) {
  clearTimeout(sourceTimer);
  sourceTimer = setTimeout(async () => {
    const n = ++seq;
    const hit = (state.deck?.slides || []).filter((x) => x.lines[0] <= last && first <= x.lines[1]);
    const blocks = (await Promise.all(hit.map((x) => fetchSlide(x.id)))).flatMap((sl) => sl?.blocks || []);
    if (n !== seq) return; // a newer focus arrived meanwhile
    focus.set({
      slide: hit.at(-1)?.id ?? null, // the caret (= the selection's end) is in the last one
      elements: blocks.filter((b) => b.lines && b.lines[0] <= last && first <= b.lines[1]).map((b) => b.id),
      line,
      selection: text ? { text, where: 'source' } : null,
    });
  }, 100);
}

// 004's source → slide sync follows the shared focus (FR-012); the front matter shows the title slide
focus.on((f, changed) => {
  if (!changed.includes('slide')) return;
  const s = slideById(f.slide);
  const anchor = s ? s.anchor : (state.deck?.title_slide ? 'title-slide' : null);
  if (anchor && anchor !== state.anchor) { state.followed = anchor; go(anchor); }
});

// framed in VS Code (spec 006): its theme, its Quarto extension's Run, a few messages
const framed = new URLSearchParams(location.search);
const setTheme = (t) => { if (['light', 'dark', 'hc'].includes(t)) document.documentElement.dataset.theme = t; };
setTheme(framed.get('theme'));
const runExternal = framed.get('run') === 'external';
document.documentElement.classList.toggle('run-external', runExternal);
if (!runExternal) run.initRun({ say, flush: pane.save });
if (keys.framed) {
  // only the panel that framed this page, and only navigation/display — never edits or code
  addEventListener('message', (e) => {
    if (e.source !== window.parent || typeof e.data !== 'object' || !e.data) return;
    if (e.data.type === 'theme') setTheme(e.data.kind);
    // VS Code's cursor: same as the source pane's caret (US3)
    const { line, first, last, text } = e.data;
    if (e.data.type === 'follow' && [line, first, last].every(Number.isInteger) && typeof text === 'string') {
      sourceFocus({ line, first, last, text });
    }
  });
}

/** Element (or slide title) → its source lines, in the pane (US2) and in VS Code's text (006 US3). */
async function revealSource(s) {
  if (srcpane.hidden && !keys.framed) return;
  const sl = !s?.ref && s?.slide && (await fetchSlide(s.slide));
  const lines = s?.ref ? (await fetchBlock(s.ref))?.lines : sl && [sl.lines[0], sl.lines[0]];
  if (!lines) return;
  if (!srcpane.hidden) pane.reveal(lines);
  if (keys.framed) window.parent.postMessage({ type: 'reveal', lines }, '*');
}

// insert (+) and drag (⠿) handles beside the element under the mouse (US4)
const hoverbar = document.createElement('div');
hoverbar.id = 'hoverbar';
hoverbar.hidden = true;
hoverbar.innerHTML = '<button type="button" data-act="insert" title="Insert after" aria-label="Insert after">+</button>'
  + '<button type="button" data-act="drag" title="Drag to move" aria-label="Drag to move">⠿</button>';
document.body.append(hoverbar);
let hovered = null;

function hoverAt(node) {
  if (state.session || menuOpen() || dragging) return;
  const el = node.closest?.('[data-qd]');
  if (!el || el.classList.contains('absolute')) return;
  hovered = el;
  const p = pageRect(el);
  hoverbar.hidden = false;
  const left = p.left - hoverbar.offsetWidth - 6;
  // no room beside it (a narrow window): no bar over the text; the right-click menu has both
  if (left < 2) { hoverbar.hidden = true; return; }
  hoverbar.style.left = left + 'px';
  hoverbar.style.top = p.top + 'px';
}
frame.addEventListener('mouseleave', (e) => { if (!hoverbar.contains(e.relatedTarget)) hoverbar.hidden = true; });

hoverbar.querySelector('[data-act=insert]').addEventListener('click', async (e) => {
  const t = hovered && (await elementTarget(hovered.dataset.qd));
  if (!t) return;
  const r = e.target.getBoundingClientRect();
  openMenu(actions.insert(t, { after: t.ref }), r.right + 4, r.top);
});

let dragging = false;
const dropline = Object.assign(document.createElement('div'), { id: 'dropline', hidden: true });
document.body.append(dropline);
hoverbar.querySelector('[data-act=drag]').addEventListener('pointerdown', (e) => {
  if (!hovered) return;
  e.preventDefault();
  const handle = e.currentTarget;
  handle.setPointerCapture(e.pointerId);
  dragging = true;
  const from = hovered.dataset.qd;
  const slideId = from.slice(0, from.lastIndexOf('/'));
  let drop = null; // { ref, before }
  const onMove = (ev) => {
    const f = frame.getBoundingClientRect();
    const hit = frame.contentDocument.elementFromPoint(ev.clientX - f.left, ev.clientY - f.top)?.closest?.('[data-qd]');
    const ref = hit?.dataset.qd;
    if (!hit || ref === from || ref.slice(0, ref.lastIndexOf('/')) !== slideId) { dropline.hidden = true; drop = null; return; }
    const p = pageRect(hit);
    const before = ev.clientY < p.top + p.height / 2;
    drop = { ref, before };
    Object.assign(dropline.style, { left: p.left + 'px', width: p.width + 'px', top: (before ? p.top - 4 : p.top + p.height + 2) + 'px' });
    dropline.hidden = false;
  };
  const onUp = async () => {
    handle.removeEventListener('pointermove', onMove);
    handle.removeEventListener('pointerup', onUp);
    dragging = false;
    dropline.hidden = true;
    hoverbar.hidden = true;
    if (!drop) return;
    const r = await op('block move', { element: from, [drop.before ? 'before' : 'after']: drop.ref });
    if (r) { state.sel = r.target; await fetchDeck(); }
  };
  handle.addEventListener('pointermove', onMove);
  handle.addEventListener('pointerup', onUp);
});

// an empty slide offers its first element
const emptyadd = Object.assign(document.createElement('button'), { type: 'button', id: 'emptyadd', textContent: '+ Add content', hidden: true });
document.body.append(emptyadd);
emptyadd.addEventListener('click', async () => {
  const cur = slideByAnchor(state.anchor);
  if (!cur) return;
  const r = emptyadd.getBoundingClientRect();
  openMenu(actions.insert({ slideId: cur.id }, { end: true }), r.left, r.bottom + 4);
});
function placeEmptyadd() {
  const cur = slideByAnchor(state.anchor);
  emptyadd.hidden = !cur || cur.blocks > 0 || cur.opaque || !!state.session;
  if (emptyadd.hidden) return;
  const f = frame.getBoundingClientRect();
  emptyadd.style.left = f.left + f.width / 2 - emptyadd.offsetWidth / 2 + 'px';
  emptyadd.style.top = f.top + f.height / 2 + 'px';
}

/** Copy a dropped image file beside the deck, then insert it before or after `on` (the half it fell on). */
async function dropImage(file, on, y) {
  const slide = slideByAnchor(state.anchor);
  if (!slide) return say('Images go on a content slide, not the title slide', true);
  const res = await fetch(`/api/images?name=${encodeURIComponent(file.name)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/octet-stream', 'X-QD-Token': run.token },
    body: file,
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) return say(res.status === 403 ? 'Adding images needs the editor link from `qd serve` or `qd url`' : `${file.name}: ${body.error?.message || res.statusText}`, true);
  // a layout's image placeholder takes the image in its place
  if (on?.matches('.qd-ph[data-ph=image]')) {
    if (await op('edit', { element: on.dataset.qd, md: `![](${body.path}){width="100%"}` })) await fetchDeck();
    return;
  }
  const r = on?.getBoundingClientRect();
  const where = on ? { [y > r.top + r.height / 2 ? 'after' : 'before']: on.dataset.qd } : { end: true };
  await insertBlock(slide.id, where, { md: `![](${body.path})` });
}

/** Insert a new element of `kind` on a slide; it opens for editing once rendered. */
async function insertBlock(slideId, where, kind) {
  if (kind.box) return newBox({ id: slideId }, { text: 'Text' });
  let md = kind.md;
  if (kind.image) {
    const v = await ask({ title: 'Image', fields: [{ name: 'src', kind: 'files', label: 'Path (beside the deck) or URL', validate: (x) => (x ? '' : 'Choose an image or type its path') }] });
    if (!v) return;
    md = `![](${v.src})`;
  }
  const r = await op('block add', { slide: slideId, ...where, md });
  if (!r) return;
  if (kind.edit) state.editAfterRender = { ref: r.target, mode: kind.edit, version: r.version };
  else state.sel = r.target;
  await fetchDeck();
}

function openInserted({ ref, mode }) {
  state.editAfterRender = null;
  const el = elOf(ref);
  if (!el) return;
  if (mode === 'source') { beginSource(el); return; }
  beginEdit({ kind: 'block', root: el }, null).then((ok) => {
    if (!ok) return;
    // select the placeholder text so typing replaces it
    const d = el.ownerDocument;
    const r = d.createRange();
    r.selectNodeContents(el.querySelector('li') || el.querySelector('p') || el);
    d.getSelection().removeAllRanges();
    d.getSelection().addRange(r);
  });
}

/** Copy an element's markdown. Our own copy is immediate; the system clipboard is
 *  written in the background (it can stay pending, e.g. in a page without focus). */
function copyElement(t) {
  state.clip = t.block.text;
  const item = new ClipboardItem({
    'text/plain': new Blob([t.block.text], { type: 'text/plain' }),
    'text/html': new Blob([t.el?.innerHTML || ''], { type: 'text/html' }),
  });
  navigator.clipboard.write([item]).catch(() => navigator.clipboard.writeText(t.block.text)).catch(() => {});
  say('Copied');
}

/** The system clipboard if it answers within a second, else our own copy. */
async function clipboardText() {
  const fromEvent = pasted;
  pasted = null;
  if (fromEvent?.trim()) return fromEvent; // an empty paste event: try the API and our own copy
  const timeout = new Promise((resolve) => { setTimeout(() => resolve(null), 1000); });
  let text = null;
  try { text = await Promise.race([navigator.clipboard.readText(), timeout]); } catch { /* not allowed */ }
  return text?.trim() ? text : state.clip;
}
// ⌘V: the keydown is let through, and the browser's own paste event brings the
// text, with no clipboard permission needed (unlike readText: a deck in another
// page, or VS Code's panel). No paste event (nothing on the page can take one)?
// Then the clipboard API, as from the menu.
let pasted = null;
let pasteKey = null; // {id, e}: the paste action the last ⌘V reached
const runPaste = async ({ id, e }) => keys.run(id, await keys.currentContext(e));
function onPaste(e) {
  if (!pasteKey) return; // set only by a ⌘V that onKey let through (not while typing)
  const k = pasteKey;
  pasteKey = null;
  e.preventDefault();
  pasted = e.clipboardData.getData('text/plain');
  runPaste(k);
}
document.addEventListener('paste', onPaste);
/** The `key` check of the paste actions: note it, let the browser fire its paste event. */
function pasteKeyFor(id) {
  return (e) => {
    if (pasteKey?.e !== e) { // the most specific place's action (dispatch tries them in order)
      pasteKey = { id, e };
      setTimeout(() => { if (pasteKey?.e === e) { pasteKey = null; runPaste({ id, e }); } }, 100);
    }
    return false;
  };
}

/** Text that isn't qd's own copy is never taken as raw HTML (spec 007 edge case). */
const outside = (text) => (text === state.clip ? text : text.replace(/&/g, '&amp;').replace(/</g, '&lt;'));

async function pasteAfter(t) {
  const text = await clipboardText();
  if (!text) { say('Nothing to paste', true); return; }
  const r = await op('block add', { slide: t.slideId, after: t.ref, md: outside(text) });
  if (r) { state.sel = r.target; await fetchDeck(); }
}

/** Copy a whole slide's markdown (it pastes into this or another deck with fresh ids). */
async function copySlide(t) {
  const ids = slideList.selected(t.slideId); // the picked slides, if this is one
  const text = ids.length > 1 ? (await Promise.all(ids.map(fetchSlide))).map((s) => s?.source?.trimEnd()).join('\n\n') + '\n' : t.full?.source;
  if (!text) return;
  state.clip = text;
  navigator.clipboard.writeText(text).catch(() => {});
  say(ids.length > 1 ? `${ids.length} slides copied` : 'Slide copied');
}

/** Slides (markdown starting with a slide heading or ---) go in after `t`; other text becomes a paragraph on it. */
async function pasteSlides(t) {
  const text = await clipboardText();
  if (!text) { say('Nothing to paste', true); return; }
  const slides = /^(#{1,6}\s|---[ \t]*(\n|$))/.test(text.trimStart());
  const r = slides
    ? await op('paste', { after: t.slideId, md: text })
    : await op('block add', { slide: t.slideId, end: true, md: outside(text) });
  if (!r) return;
  if (slides) state.goAfterRender = r.target;
  else state.sel = r.target;
  await fetchDeck();
}

/** Speaker notes (`::: notes`): edit them in the source box over the slide, creating them if missing. */
async function editNotes(t) {
  const sec = frame.contentDocument?.getElementById(t.anchor);
  if (!sec || state.session || !t.full) return;
  const i = t.full.blocks.findIndex((b) => b.kind === 'notes');
  let ref = `${t.slideId}/${i}`;
  if (i < 0) {
    const r = await op('block add', { slide: t.slideId, end: true, md: '::: notes\nSpeaker notes.\n:::' });
    if (!r) return;
    await fetchDeck();
    ref = r.target;
  }
  const b = await fetchBlock(ref);
  if (b) startSourceSession({ kind: 'block', mode: 'source', root: sec, ref, source: b.text, hash: b.hash }, b.text);
}

// ---------- what each action in the list does (spec 007) ----------

keys.setContext(async (e) => {
  // a key in the slide list acts on its focused slide, which may not be shown yet
  const li = e?.target?.closest?.('#slides li');
  const cur = li ? slideById(li.dataset.id) : slideByAnchor(state.anchor);
  return { e, el: state.sel ? await elementTarget(state.sel) : null, slide: cur ? await slideTarget(cur.id) : null };
});
keys.on('render', { enabled: () => !busy.render, run: renderDeck });
keys.on('export', { enabled: () => !busy.export, run: () => exportDeck(false) });
keys.on('export-pdf', { enabled: () => !busy.export, run: () => exportDeck(true) });
// spec 011: one action per format (actions.json's `qd export --to <name>`);
// a format whose tool is missing is disabled, saying why
let formats = new Map();
async function loadFormats() {
  const res = await fetch('/api/op', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-QD-Token': run.token }, body: JSON.stringify({ op: 'export', args: { list: true } }) });
  if (res.ok) formats = new Map((await res.json()).formats.map((f) => [f.name, f]));
}
loadFormats();
for (const a of keys.ACTIONS.filter((a) => a.qd?.startsWith('qd export --to '))) {
  const name = a.qd.split(' ').pop();
  keys.on(a.id, {
    enabled: () => !busy.export && formats.get(name)?.ready !== false,
    why: () => formats.get(name)?.hint,
    run: () => exportTo(name, formats.get(name)?.label || name),
  });
}
// any other writer Pandoc knows, by name (US4)
keys.on('export-other', {
  enabled: () => !busy.export,
  run: async () => {
    const others = [...formats.values()].filter((f) => f.group === 'other').map((f) => f.name);
    const v = await ask({
      title: 'Export in another format',
      message: 'Any format Pandoc writes, such as rst, org, latex or asciidoc. Quarto-only features may be simplified.',
      fields: [{ name: 'format', label: 'Format', suggestions: others, validate: (x) => (others.includes(x) ? '' : 'Not a format Pandoc writes') }],
      ok: 'Next',
    });
    if (v) await exportTo(v.format, v.format);
  },
});
keys.on('preview', () => window.open('/present/', '_blank', 'noopener'));
keys.on('present', () => present(false));
keys.on('present-start', () => present(true));
$('#present').addEventListener('click', () => keys.run('present'));
/** A header button that opens a menu. */
const menuButton = (sel, items) => $(sel).addEventListener('click', async (e) => {
  const r = e.currentTarget.getBoundingClientRect();
  openMenu(await items(), r.left, r.bottom + 2);
});
// File ▸ Export's items; formats are checked again on each open (TeX installed meanwhile)
menuButton('#export-menu', async () => { await loadFormats(); return submenu(['File', 'Export']); });
menuButton('#present-more', () => ['present', 'present-start', 'preview'].map((id) => ({ label: keys.action(id).label, shortcut: keys.shortcut(id), run: () => keys.run(id) })));
keys.on('undo', () => undoRedo('undo'));
keys.on('redo', () => undoRedo('redo'));
// element and slide actions: the right-click menus' own (actions.js), on what is selected / shown
const ELEMENT = ['edit', 'source', 'cut', 'copy', 'paste', 'dup', 'delete', 'up', 'down', 'fragment', 'class-smaller', 'class-incremental', 'class-custom', 'place', 'unplace', 'box-front', 'box-forward', 'box-backward', 'box-back'];
const SLIDE = ['slide-before', 'slide-after', 'slide-dup', 'slide-cut', 'slide-copy', 'slide-paste', 'slide-hide', 'slide-rm', 'slide-bg', 'slide-bgimg', 'slide-notes', 'slide-source', 'slide-link', 'layout-save'];
for (const [ids, on, table] of [[ELEMENT, 'el', 'element'], [SLIDE, 'slide', 'slide']]) {
  for (const id of ids) {
    keys.on(id, {
      key: id.endsWith('paste') ? pasteKeyFor(id) : undefined,
      // Paste reads the system clipboard on a key press; from a menu it needs our own copy
      enabled: (ctx) => !!ctx[on] && actions.can(id, ctx[on], table) && (!id.endsWith('paste') || !!ctx.e || !!state.clip),
      checked: (ctx) => ctx[on] && actions.checked(id, ctx[on], table),
      why: (ctx) => ctx[on] && actions.tip(id, ctx[on], table),
      run: (ctx) => actions.run(id, ctx[on], table),
    });
  }
}
// arrow keys nudge a placed element (spec 009 US2), ⇧ by 10; otherwise they stay reveal.js's
const boxSelected = () => !!state.sel && !!state.selBox;
for (const [dir, dx, dy] of [['left', -1, 0], ['right', 1, 0], ['up', 0, -1], ['down', 0, 1]]) {
  keys.on(`box-nudge-${dir}`, {
    key: boxSelected,
    enabled: (ctx) => ctx.el?.block.kind === 'box',
    run: async (ctx) => {
      const n = ctx.e?.shiftKey ? 10 : 1;
      if (await op('box move', { box: ctx.el.ref, by: `${dx * n},${dy * n}` })) { await fetchDeck(); await renderPanel(); }
    },
  });
}
const inSlide = (e) => e.target.ownerDocument === frame.contentDocument && !!frameWin()?.Reveal?.getCurrentSlide()?.querySelector('[data-qd]');
keys.on('next-element', { key: inSlide, enabled: (ctx) => !!ctx.slide, run: () => stepSelection(false) });
keys.on('prev-element', { key: inSlide, enabled: (ctx) => !!ctx.slide, run: () => stepSelection(true) });
keys.on('deselect', { enabled: () => !!state.sel, run: () => select(null) });
keys.on('actions-menu', keyboardMenu);
keys.on('focus-text', () => window.parent.postMessage({ type: 'focusText' }, '*')); // VS Code's panel only
for (const kind of INSERTS) {
  keys.on(`insert-${kind.id}`, {
    enabled: (ctx) => !!(ctx.el || ctx.slide),
    run: (ctx) => insertBlock(ctx.el?.slideId ?? ctx.slide.slideId, ctx.el ? { after: ctx.el.ref } : { end: true }, kind),
  });
}
for (const x of ['default', ...TRANSITIONS]) {
  keys.on(`transition-${x}`, {
    enabled: (ctx) => !!ctx.slide,
    checked: (ctx) => (ctx.slide?.full?.attrs?.kv?.transition || 'default') === x,
    run: (ctx) => actions.setAttr(ctx.slide, 'transition', x === 'default' ? null : x),
  });
}
const neighbour = (ctx, d) => { const ss = state.deck?.slides || []; return ss[ss.findIndex((x) => x.id === ctx.slide?.slideId) + d]; };
for (const [id, d, where] of [['slide-move-up', -1, 'before'], ['slide-move-down', 1, 'after']]) {
  keys.on(id, {
    enabled: (ctx) => !!neighbour(ctx, d),
    run: async (ctx) => { if (await op('move', { slide: ctx.slide.slideId, [where]: neighbour(ctx, d).id })) { state.goAfterRender = ctx.slide.anchor; await fetchDeck(); } },
  });
}
const toggle = (cls, set) => ({ checked: () => mainEl.classList.contains(cls), run: () => set(!mainEl.classList.contains(cls)) });
keys.on('toggle-source', { checked: () => !srcpane.hidden, run: () => setSource(srcpane.hidden) });
keys.on('source-full', toggle('source-full', setFull));
keys.on('form-panel', toggle('panel-open', setPanel));
keys.on('snap', { checked: () => settings.get('placement.snap'), run: () => settings.set({ 'placement.snap': !settings.get('placement.snap') }) });
keys.on('about', () => ask({ title: 'About quartz-deck', message: 'A slide editor for Quarto reveal.js decks. Alpha software, not licensed for use by others (see LICENSE).', cancel: null }));
keys.on('settings', settings.open);

