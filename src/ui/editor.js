// quartz-deck editor. Every change is a CLI op sent to POST /api/op, so the GUI
// can do nothing the CLI cannot (FR-010). The .qmd file stays the only state.
import { region, hintsFor, gate, reserialize, splice } from '/ui/inline.js';
import { showToolbar, hideToolbar, formatKey, place as placeToolbar } from '/ui/toolbar.js';
import { openSource, closeSource, sourceError, sourceText } from '/ui/source.js';
import { makeActions } from '/ui/actions.js';
import { openMenu, closeMenu, menuOpen } from '/ui/menu.js';
import * as pane from '/ui/pane.js';
import * as run from '/ui/run.js';
import * as focus from '/ui/focus.js';

const $ = (sel, root = document) => root.querySelector(sel);
const frame = $('#preview');
const statusEl = $('#status');
const notice = $('#notice');
const banner = $('#banner');
const list = $('#slides');
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
  pendingRender: null, // newest render that arrived while editing (applied when it ends)
  editAfterRender: null, // {ref, mode}: open a just-inserted element for editing once rendered
  clip: null, // last copied element markdown (fallback when the clipboard can't be read)
};

// ---------- helpers ----------

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
  focus.setVersion(state.version);
  renderList();
  renderSlidebar();
}

async function fetchSlide(ref) {
  const res = await fetch('/api/slide/' + encodeURIComponent(ref));
  return res.ok ? (await res.json()).slide : null;
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
  const res = await fetch('/api/op', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ op: name, args, ...(fileExpect ? { expect: state.version } : {}) }),
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    const e = body.error || { code: res.status, message: res.statusText };
    op.lastError = e;
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
  say(`${name} ✓`);
  return body;
}

// ---------- preview ----------

function load(path) {
  // Reveal may ignore the initial URL hash (e.g. when it starts in scroll view),
  // so remember where to go and steer it once it reports ready (see hookFrame).
  state.want = state.goAfterRender || state.anchor || location.hash.replace(/^#\//, '') || null;
  state.goAfterRender = null;
  frame.src = path + '?v=' + Date.now();
}

function go(anchor) {
  const hash = '#/' + anchor;
  history.replaceState(null, '', hash);
  const w = frameWin();
  if (w) w.location.hash = hash;
}

const PREVIEW_CSS = `
  [data-qd] { cursor: pointer; }
  [data-qd]:hover { outline: 2px dashed rgba(42, 91, 215, .55); outline-offset: 2px; }
  .qd-sel, .qd-sel:hover { outline: 3px solid #2a5bd7 !important; outline-offset: 2px; }
  .qd-editing, .qd-editing:hover { outline: 3px solid #2a5bd7 !important; outline-offset: 4px; cursor: text; }
  .qd-editing:focus { outline-style: solid; }
  /* every fragment is shown (and reachable) in the editor; the marker says it appears on click */
  .reveal .slides section .fragment { opacity: 1 !important; visibility: inherit !important; transform: none !important;
    box-shadow: inset 3px 0 0 rgba(42, 91, 215, .35); }
  section > h1:hover, section > h2:hover, section > h3:hover, #title-slide :is(.title, .subtitle, .date):hover {
    outline: 2px dashed rgba(42, 91, 215, .55); outline-offset: 2px; cursor: text; }
  .qd-overlay { position: absolute; box-sizing: border-box; border: 2px solid #2a5bd7; cursor: move;
    touch-action: none; z-index: 1000; }
  .qd-handle { position: absolute; width: 14px; height: 14px; margin: -7px 0 0 -7px; background: #fff;
    border: 2px solid #2a5bd7; border-radius: 3px; box-sizing: border-box; }
  section[data-qd-hidden]::after { content: "hidden slide (visibility=hidden): not in the rendered deck";
    position: absolute; top: 0; right: 0; font: 14px system-ui, sans-serif; padding: 2px 8px;
    background: #fff3c4; color: #5c4400; border-radius: 4px; }
`;

function highlight() {
  const doc = frame.contentDocument;
  if (!doc) return;
  doc.querySelectorAll('.qd-sel').forEach((el) => el.classList.remove('qd-sel'));
  doc.querySelectorAll('.qd-overlay').forEach((el) => el.remove());
  if (state.sel) {
    const els = doc.querySelectorAll(`[data-qd="${CSS.escape(state.sel)}"]`);
    els.forEach((el) => el.classList.add('qd-sel'));
    const box = [...els].find((el) => el.classList.contains('absolute'));
    if (box) attachOverlay(box);
  }
  placeElbar();
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
    const scale = frameWin().Reveal?.getScale?.() || 1;
    const start = rectOf(box);
    const x0 = e.clientX;
    const y0 = e.clientY;
    let cur = start;
    ov.setPointerCapture(e.pointerId);
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
        r.w = Math.max(1, r.w);
        r.h = Math.max(1, r.h);
      }
      cur = { l: Math.round(r.l), t: Math.round(r.t), w: Math.round(r.w), h: Math.round(r.h) };
      place(box, cur);
      place(ov, cur);
    };
    const onUp = async () => {
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

function hookFrame() {
  const w = frameWin();
  const doc = frame.contentDocument;
  if (!w || !doc) return;
  doc.addEventListener('selectionchange', () => { if (state.session) syncFocus(); });
  const style = doc.createElement('style');
  style.textContent = PREVIEW_CSS;
  doc.head.append(style);

  doc.addEventListener('click', (e) => {
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
  doc.addEventListener('dblclick', (e) => {
    if (state.session) return;
    const el = e.target.closest('[data-qd]') || doc.querySelector(`[data-qd="${CSS.escape(state.sel || '')}"]`);
    if (!el) return;
    e.preventDefault();
    // a positioned text box: single click selects it for dragging, double click types
    beginEdit({ kind: 'block', root: el }, { x: e.clientX, y: e.clientY }).then((ok) => {
      if (!ok) beginSource(el);
    });
  }, true);
  doc.addEventListener('keydown', onKey, true);
  doc.addEventListener('contextmenu', (e) => {
    if (state.session) return; // the browser's own menu (spelling) while typing
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
    // Escape belongs to the editor (leave an edit / deselect), not Reveal's overview
    w.Reveal.configure({ scrollActivationWidth: 1, keyboard: { 27: null } });
    w.dispatchEvent(new Event('resize'));
    // synchronously: a hash change lands a task later, and onSlide() below would
    // meanwhile report the first slide (hiding the slide bar under a click)
    const want = state.want && frame.contentDocument.getElementById(state.want);
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
    if (state.editAfterRender) openInserted(state.editAfterRender);
    onSlide();
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
    renderList();
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

// ---------- hover preview of a slide in the list ----------

// One small second copy of the rendered deck, steered by hash. Reloaded only when
// the main preview re-renders, so hovering is instant after the first time.
const peek = Object.assign(document.createElement('iframe'), { id: 'peek', tabIndex: -1, hidden: true });
peek.setAttribute('aria-hidden', 'true');
document.body.append(peek);
let peekSrc = null;
let peekWant = null;
let peekTimer;

peek.addEventListener('load', () => {
  const w = peek.contentWindow;
  const ready = () => {
    // as in hookFrame: the small frame would otherwise switch Reveal to scroll view
    w.Reveal.configure({ scrollActivationWidth: 1, controls: false, progress: false, slideNumber: false });
    const style = w.document.createElement('style');
    style.textContent = '.slide-menu-button { display: none !important; }';
    w.document.head.append(style);
    w.dispatchEvent(new Event('resize'));
    if (peekWant) w.location.hash = '#/' + peekWant;
  };
  if (w.Reveal?.isReady?.()) ready(); else w.Reveal?.on?.('ready', ready);
});

function showPeek(li, anchor) {
  clearTimeout(peekTimer);
  peekTimer = setTimeout(() => {
    peekWant = anchor;
    const src = frame.getAttribute('src');
    if (!src) return;
    if (peekSrc !== src) { peekSrc = src; peek.src = src + '#/' + anchor; }
    else peek.contentWindow.location.hash = '#/' + anchor;
    const r = li.getBoundingClientRect();
    const W = 300; const H = 200; const m = 8;
    const across = getComputedStyle(list).display === 'flex';
    const clamp = (v, max) => Math.max(m, Math.min(v, max - m));
    peek.style.left = (across ? clamp(r.left, innerWidth - W) : r.right + m) + 'px';
    peek.style.top = (across ? r.bottom + 6 : clamp(r.top, innerHeight - H)) + 'px';
    peek.hidden = false;
  }, 200);
}

function hidePeek() {
  clearTimeout(peekTimer);
  peek.hidden = true;
}

// ---------- slide list (drag to reorder) ----------

let dragId = null;

function clearDrop() {
  list.querySelectorAll('.drop-before, .drop-after, .dragging').forEach((li) => li.classList.remove('drop-before', 'drop-after', 'dragging'));
}

function renderList() {
  const slides = state.deck?.slides || [];
  const cur = slideByAnchor(state.anchor);
  list.replaceChildren(...slides.map((s) => {
    const li = document.createElement('li');
    li.draggable = true;
    li.dataset.id = s.id;
    if (cur && s.id === cur.id) li.classList.add('current');
    const b = document.createElement('button');
    b.type = 'button';
    const idx = document.createElement('span');
    idx.className = 'idx';
    idx.textContent = s.index;
    b.append(idx, s.title || s.summary || '(untitled)');
    if (s.hidden) {
      const tag = document.createElement('span');
      tag.className = 'tag';
      tag.textContent = 'hidden';
      b.append(' ', tag);
    }
    if (cur && s.id === cur.id) b.setAttribute('aria-current', 'true');
    b.addEventListener('click', () => { hidePeek(); state.sel = null; go(s.anchor); });
    li.append(b);
    li.addEventListener('mouseenter', () => { if (!dragId && !state.session) showPeek(li, s.anchor); });
    li.addEventListener('contextmenu', async (e) => {
      e.preventDefault();
      hidePeek();
      openMenu(actions.slide(await slideTarget(s.id)), e.clientX, e.clientY);
    });
    li.addEventListener('mouseleave', hidePeek);

    li.addEventListener('dragstart', (e) => {
      hidePeek();
      dragId = s.id;
      li.classList.add('dragging');
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', s.id);
    });
    li.addEventListener('dragend', () => { dragId = null; clearDrop(); });
    li.addEventListener('dragover', (e) => {
      if (!dragId || dragId === s.id) return;
      e.preventDefault();
      const r = li.getBoundingClientRect();
      const across = getComputedStyle(list).display === 'flex'; // narrow layout: horizontal strip
      const before = across ? e.clientX < r.left + r.width / 2 : e.clientY < r.top + r.height / 2;
      li.classList.toggle('drop-before', before);
      li.classList.toggle('drop-after', !before);
    });
    li.addEventListener('dragleave', () => li.classList.remove('drop-before', 'drop-after'));
    li.addEventListener('drop', async (e) => {
      e.preventDefault();
      const before = li.classList.contains('drop-before');
      const moving = dragId;
      clearDrop();
      if (!moving || moving === s.id) return;
      const ids = (state.deck?.slides || []).map((x) => x.id);
      const from = ids.indexOf(moving);
      const to = ids.indexOf(s.id) + (before ? 0 : 1);
      if (to === from || to === from + 1) return; // already there
      const res = await op('move', { slide: moving, [before ? 'before' : 'after']: s.id });
      if (res) await fetchDeck();
    });
    return li;
  }));
  list.querySelector('li.current')?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
}

$('#add-slide').addEventListener('click', async () => {
  const cur = slideByAnchor(state.anchor);
  const args = { title: 'New slide' };
  if (cur) args.after = cur.id;
  const res = await op('add', args);
  if (res) { state.goAfterRender = res.target; state.sel = null; await fetchDeck(); }
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

async function undoRedo(kind) {
  const r = await op(kind);
  if (r) { await fetchDeck(); await renderPanel(); }
}
$('#render').addEventListener('click', async (e) => {
  const btn = e.currentTarget;
  btn.disabled = true;
  btn.textContent = 'Rendering…';
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
    btn.disabled = false;
    btn.textContent = 'Render';
  }
});
// ---------- export (spec 002 US4) ----------

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
  $('[data-act=close]', view).addEventListener('click', () => renderPanel());
  panel.replaceChildren(view);
}

$('#export').addEventListener('click', async (e) => {
  const btn = e.currentTarget;
  btn.disabled = true;
  btn.textContent = 'Exporting…';
  try {
    const r = await op('export', { pdf: $('#export-pdf').checked });
    if (r) renderExportPanel(r);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Export';
  }
});

$('#undo').addEventListener('click', () => undoRedo('undo'));
$('#redo').addEventListener('click', () => undoRedo('redo'));

/** Element action for a key press on the selected element (contracts/ui.md), or undefined. */
function elementKey(e, mod) {
  if (mod) return e.shiftKey ? undefined : { c: 'copy', x: 'cut', v: 'paste', d: 'dup' }[e.key.toLowerCase()];
  if (e.key === 'Enter') return e.altKey ? 'source' : 'edit';
  if (e.key === 'Backspace' || e.key === 'Delete') return 'delete';
  if (e.altKey) return { ArrowUp: 'up', ArrowDown: 'down' }[e.key];
  return undefined;
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
  if (menuOpen()) { if (e.key === 'Escape') { e.preventDefault(); closeMenu(); } return; }
  if (state.session || editing()) return; // edits and form fields handle their own keys
  // Tab moves between elements only while the slide has focus; elsewhere it moves focus as usual
  if (e.key === 'Tab' && e.target.ownerDocument === frame.contentDocument && stepSelection(e.shiftKey)) { e.preventDefault(); e.stopPropagation(); return; }
  const mod = e.metaKey || e.ctrlKey;
  if ((e.shiftKey && e.key === 'F10') || e.key === 'ContextMenu') { e.preventDefault(); keyboardMenu(); return; }
  if (state.sel) {
    const id = elementKey(e, mod);
    if (id) { e.preventDefault(); e.stopPropagation(); elementTarget(state.sel).then((t) => t && actions.run(id, t)); return; }
  }
  if (mod && e.shiftKey && (e.key.toLowerCase() === 'n' || e.key.toLowerCase() === 'd')) {
    const cur = slideByAnchor(state.anchor);
    if (cur) { e.preventDefault(); slideTarget(cur.id).then((t) => actions.run(e.key.toLowerCase() === 'n' ? 'slide-after' : 'slide-dup', t, 'slide')); }
    return;
  }
  if (e.key === 'Escape' && state.sel) { e.preventDefault(); e.stopPropagation(); select(null); return; }
  if (!mod || e.key.toLowerCase() !== 'z' || editing()) return;
  e.preventDefault();
  undoRedo(e.shiftKey ? 'redo' : 'undo');
}
document.addEventListener('keydown', onKey);

// ---------- live updates ----------

const es = new EventSource(`/events?tab=${focus.tabId}`);
run.listen(es);
es.addEventListener('open', () => { statusEl.textContent = 'connected'; });
es.addEventListener('error', () => { statusEl.textContent = 'disconnected — is qd serve running?'; });
es.addEventListener('changed', async (e) => {
  statusEl.textContent = 'changed…';
  const { version } = JSON.parse(e.data);
  if (version !== state.version) {
    await fetchDeck();
    if (!editing()) await renderPanel();
  }
  pane.changed();
});
es.addEventListener('rendering', () => { statusEl.textContent = 'rendering…'; });
es.addEventListener('rendered', (e) => {
  const d = JSON.parse(e.data);
  statusEl.textContent = 'rendered ' + d.version.slice(0, 7);
  banner.hidden = true;
  if (state.session) {
    // reloading now would throw away the open edit (research R4)
    state.pendingRender = d.html;
    pendingBadge(true);
  } else {
    load(d.html);
  }
});
es.addEventListener('render_error', (e) => {
  const d = JSON.parse(e.data);
  statusEl.textContent = 'render failed (showing last good render)';
  banner.textContent = d.message;
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
  if (el) return el.classList.contains('absolute') ? null : { kind: 'block', root: el };
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
    if (!full?.title) return false;
    s = { ...target, slide: full.id, source: full.title, hash: full.title_hash,
      reg: { mode: 'inline', prefix: '', inner: full.title, suffix: '' }, hints: {} };
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
  hidePeek();
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
  on('input', () => { s.failed = false; placeToolbar(frame); });
  showToolbar(s, frame);
  say(s.reg.mode === 'inline' ? 'Editing — Enter or click elsewhere saves, Esc cancels' : 'Editing — ⌘Enter, Tab or click elsewhere saves, Esc cancels');
}

function sessionKey(e, s) {
  e.stopPropagation(); // keep Reveal and the editor's own keys out of the edit
  if (e.key === 'Escape') { e.preventDefault(); endSession(s, false); if (s.ref) select(s.ref); return; }
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
  formatKey(e);
}

function inList(s) {
  const sel = s.root.ownerDocument.getSelection();
  const n = sel?.anchorNode;
  return !!(n && (n.nodeType === 1 ? n : n.parentElement)?.closest('li'));
}

/** Markdown the session would write, or null if the edit can't be expressed. */
function draftSource(s) {
  let S1;
  try { S1 = reserialize(s.root, s.reg, s.hints); } catch { return null; }
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
    say('This formatting can’t be written as markdown here — undo it (⌘Z) or press Esc to cancel', true);
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
    const html = state.pendingRender;
    state.pendingRender = null;
    pendingBadge(false);
    load(html);
  }
}

// ---------- source editing in place (spec 003 US2) ----------

function pageRect(el) {
  const f = frame.getBoundingClientRect();
  const r = el.getBoundingClientRect();
  return { left: f.left + r.left, top: f.top + r.top, width: r.width, height: r.height };
}

function startSourceSession(s, text) {
  openSession(s);
  openSource(pageRect(s.root), text, { save: () => saveSession(), cancel: () => endSession(s, false) });
  say('Editing source — ⌘Enter or click elsewhere saves, Esc cancels');
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
  return { ref, block, slideId, index, count: slide.blocks.length, el };
}

/** … or a slide. */
async function slideTarget(slideId) {
  const listed = slideById(slideId);
  const full = await fetchSlide(slideId);
  return { slideId, anchor: listed?.anchor || slideId, title: full?.title, full };
}

const actions = makeActions({
  runCells: run.runCells,
  op: async (name, args) => {
    if (name === 'rm') {
      const slides = state.deck?.slides || [];
      const i = slides.findIndex((x) => x.id === args.slide);
      const near = slides[i - 1] || slides[i + 1];
      if (near) state.goAfterRender = near.anchor;
    }
    const r = await op(name, args);
    if (!r) return null;
    if (name === 'add' || name === 'dup') state.goAfterRender = r.target;
    if (name.startsWith('block') && name !== 'block rm' && r.target?.includes('/')) state.sel = r.target;
    if (name === 'block rm' || name === 'rm') state.sel = null;
    await fetchDeck();
    await renderPanel();
    renderSlidebar();
    return r;
  },
  beginEditRef: (t) => t.el && beginEdit({ kind: 'block', root: t.el }, null).then((ok) => ok || beginSource(t.el)),
  beginSourceEl: (t) => t.el && beginSource(t.el),
  beginSlideSource: (t) => beginSlideSource(frame.contentDocument?.getElementById(t.anchor)),
  editNotes,
  insert: insertBlock,
  copy: copyElement,
  paste: pasteAfter,
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
for (const [id, label, title] of [['run', '▶ Run', 'Run this code cell'], ['edit', 'Edit', 'Edit text (Enter)'], ['source', 'Source', 'Edit source (⌥Enter)'],
  ['fragment', 'On click', 'Appears on click'], ['delete', 'Delete', 'Delete (⌫)'], ['more', '⋯', 'More (right-click, ⇧F10)']]) {
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
  const p = pageRect(el);
  elbar.hidden = false;
  elbar.style.left = Math.max(4, Math.min(p.left, innerWidth - elbar.offsetWidth - 4)) + 'px';
  elbar.style.top = Math.max(frame.getBoundingClientRect().top + 2, p.top - elbar.offsetHeight - 10) + 'px';
}

// slide bar: the slide's common properties (US3)
const slidebar = $('#slidebar');
async function renderSlidebar() {
  const cur = slideByAnchor(state.anchor);
  slidebar.hidden = !cur;
  if (!cur) return;
  const full = await fetchSlide(cur.id);
  const kv = full?.attrs?.kv || {};
  slidebar.bg.value = /^#[0-9a-f]{6}$/i.test(kv['background-color'] || '') ? kv['background-color'] : '#ffffff';
  slidebar.querySelector('[name=transition]').value = kv.transition || '';
  slidebar.querySelector('[name=hidden]').checked = kv.visibility === 'hidden';
  placeEmptyadd();
}
slidebar.bg = slidebar.querySelector('[name=bg]');
const setSlideAttr = (key, value) => {
  const cur = slideByAnchor(state.anchor);
  if (cur) actions.setAttr({ slideId: cur.id }, key, value);
};
slidebar.bg.addEventListener('change', () => setSlideAttr('background-color', slidebar.bg.value));
slidebar.querySelector('[name=transition]').addEventListener('change', (e) => setSlideAttr('transition', e.target.value));
slidebar.querySelector('[name=hidden]').addEventListener('change', (e) => setSlideAttr('visibility', e.target.checked ? 'hidden' : null));
slidebar.addEventListener('click', async (e) => {
  const act = e.target.closest('button')?.dataset.act;
  const cur = slideByAnchor(state.anchor);
  if (!act || !cur) return;
  const t = await slideTarget(cur.id);
  if (act === 'bg-clear') setSlideAttr('background-color', null);
  else if (act === 'bg-image') actions.run('slide-bgimg', t, 'slide');
  else if (act === 'more') { const r = e.target.getBoundingClientRect(); openMenu(actions.slide(t), r.left, r.bottom + 4); }
});

// layout prefs (per browser): source pane, its width / full width, the old form panel
const mainEl = document.querySelector('main');
const toggleSource = $('#toggle-source');
const srcpane = $('#srcpane');
const pref = (k, v) => { try { if (v === undefined) return localStorage.getItem(k); localStorage.setItem(k, v); } catch { /* storage unavailable */ } return null; };
/** Toggle a layout class on <main>, its button's aria-pressed, and the remembered pref. */
function layout(cls, key, on, btn) {
  mainEl.classList.toggle(cls, on);
  btn?.setAttribute('aria-pressed', String(on));
  pref(key, on ? '1' : '');
}
const setPanel = (on) => layout('panel-open', 'qd.panel', on);
const setFull = (on) => layout('source-full', 'qd.sourcefull', on, $('[data-act=full]', srcpane));
function setSource(open) {
  layout('source-open', 'qd.source', open, toggleSource);
  srcpane.hidden = !open;
  if (open) pane.load();
}
toggleSource.addEventListener('click', () => setSource(srcpane.hidden));
$('[data-act=form]', srcpane).addEventListener('click', () => setPanel(!mainEl.classList.contains('panel-open')));
$('[data-act=full]', srcpane).addEventListener('click', () => setFull(!mainEl.classList.contains('source-full')));
const splitter = $('#splitter');
const setWidth = (px) => { mainEl.style.setProperty('--srcw', Math.max(240, Math.min(px, innerWidth - 320)) + 'px'); pref('qd.srcw', String(px)); };
splitter.addEventListener('pointerdown', (e) => {
  splitter.setPointerCapture(e.pointerId);
  const move = (ev) => setWidth(ev.clientX - srcpane.getBoundingClientRect().left); // source on the left
  splitter.addEventListener('pointermove', move);
  splitter.addEventListener('pointerup', () => splitter.removeEventListener('pointermove', move), { once: true });
});
splitter.addEventListener('keydown', (e) => {
  const d = { ArrowLeft: -20, ArrowRight: 20 }[e.key];
  if (d) { e.preventDefault(); setWidth(srcpane.getBoundingClientRect().width + d); }
});
if (pref('qd.srcw')) setWidth(Number(pref('qd.srcw')));
setPanel(pref('qd.panel') === '1');
setFull(pref('qd.sourcefull') === '1');
setSource(pref('qd.source') === '1');

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

run.initRun({ say, flush: pane.save });

/** Element (or slide title) → its source lines in the pane (US2). */
async function revealSource(s) {
  if (srcpane.hidden) return;
  if (s?.ref) { const b = await fetchBlock(s.ref); if (b) pane.reveal(b.lines); return; }
  if (s?.slide) { const sl = await fetchSlide(s.slide); if (sl) pane.reveal([sl.lines[0], sl.lines[0]]); }
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
  hoverbar.style.left = Math.max(2, p.left - hoverbar.offsetWidth - 6) + 'px';
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

/** Insert a new element of `kind` on a slide; it opens for editing once rendered. */
async function insertBlock(slideId, where, kind) {
  if (kind.box) return newBox({ id: slideId }, { text: 'Text' });
  let md = kind.md;
  if (kind.image) {
    const path = prompt('Image path (relative to the deck) or URL');
    if (!path?.trim()) return;
    md = `![](${path.trim()})`;
  }
  const r = await op('block add', { slide: slideId, ...where, md });
  if (!r) return;
  if (kind.edit) state.editAfterRender = { ref: r.target, mode: kind.edit };
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

async function pasteAfter(t) {
  let text = null;
  // the system clipboard if it answers within a second, else our own copy
  const timeout = new Promise((resolve) => { setTimeout(() => resolve(null), 1000); });
  try { text = await Promise.race([navigator.clipboard.readText(), timeout]); } catch { /* not allowed */ }
  text = text?.trim() ? text : state.clip;
  if (!text) { say('Nothing to paste', true); return; }
  const r = await op('block add', { slide: t.slideId, after: t.ref, md: text });
  if (r) { state.sel = r.target; await fetchDeck(); }
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
