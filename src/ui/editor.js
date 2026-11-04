// quartz-deck editor. Every change is a CLI op sent to POST /api/op, so the GUI
// can do nothing the CLI cannot (FR-010). The .qmd file stays the only state.
'use strict';

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
};

// ---------- helpers ----------

function say(msg, isError = false) {
  notice.textContent = msg;
  notice.className = isError ? 'error' : '';
  clearTimeout(say.t);
  say.t = setTimeout(() => { notice.textContent = ''; }, isError ? 8000 : 3000);
}

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
  renderList();
}

async function fetchSlide(ref) {
  const res = await fetch('/api/slide/' + encodeURIComponent(ref));
  return res.ok ? (await res.json()).slide : null;
}

/** Run one CLI op. Returns the result object, or null after reporting the error. */
async function op(name, args = {}) {
  const res = await fetch('/api/op', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ op: name, args, expect: state.version }),
  });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    const e = body.error || { code: res.status, message: res.statusText };
    if (e.code === 'E_CONFLICT') {
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
`;

function highlight() {
  const doc = frame.contentDocument;
  if (!doc) return;
  doc.querySelectorAll('.qd-sel').forEach((el) => el.classList.remove('qd-sel'));
  if (state.sel) doc.querySelectorAll(`[data-qd="${CSS.escape(state.sel)}"]`).forEach((el) => el.classList.add('qd-sel'));
}

function hookFrame() {
  const w = frameWin();
  const doc = frame.contentDocument;
  if (!w || !doc) return;
  const style = doc.createElement('style');
  style.textContent = PREVIEW_CSS;
  doc.head.append(style);

  doc.addEventListener('click', (e) => {
    const el = e.target.closest('[data-qd]');
    if (!el) return;
    e.preventDefault();
    e.stopPropagation();
    select(el.dataset.qd);
  }, true);
  doc.addEventListener('dblclick', (e) => {
    const el = e.target.closest('[data-qd]');
    if (!el) return;
    e.preventDefault();
    select(el.dataset.qd).then(() => $('textarea', panel)?.focus());
  }, true);
  doc.addEventListener('keydown', onKey, true);

  const onReady = () => {
    // A narrow preview pane would put Reveal into its scroll view; the editor
    // always wants the normal slide view.
    if (w.Reveal.isScrollView?.()) {
      w.Reveal.configure({ scrollActivationWidth: 1 });
      w.dispatchEvent(new Event('resize'));
    }
    if (state.want && w.Reveal.getCurrentSlide()?.id !== state.want) w.location.hash = '#/' + state.want;
    state.want = null;
    onSlide();
  };
  const onSlide = () => {
    const id = w.Reveal?.getCurrentSlide()?.id;
    if (!id || id === state.anchor) return;
    state.anchor = id;
    history.replaceState(null, '', '#/' + id);
    const s = slideByAnchor(id);
    if (state.sel && s && !state.sel.startsWith(s.id + '/')) state.sel = null;
    renderList();
    renderPanel();
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
    if (cur && s.id === cur.id) b.setAttribute('aria-current', 'true');
    b.addEventListener('click', () => { state.sel = null; go(s.anchor); });
    li.append(b);

    li.addEventListener('dragstart', (e) => {
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
      const before = e.clientY < r.top + r.height / 2;
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
  const f = $('form', view);
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
  panel.replaceChildren(view);
}

async function renderTitlePanel() {
  const view = $('#title-panel').content.cloneNode(true);
  const f = $('form', view);
  const res = await fetch('/api/op', {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ op: 'meta' }),
  });
  const meta = res.ok ? (await res.json()).front_matter || {} : {};
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
  if (act === 'up' && i > 0) res = await op('move', { slide: slide.id, before: slides[i - 1].id });
  if (act === 'down' && i < slides.length - 1) res = await op('move', { slide: slide.id, after: slides[i + 1].id });
  if (act === 'dup') {
    res = await op('dup', { slide: slide.id });
    if (res) state.goAfterRender = res.target;
  }
  if (act === 'rm') {
    if (!confirm(`Delete slide "${slide.title || slide.id}"? (Undo can restore it.)`)) return;
    res = await op('rm', { slide: slide.id });
    const prev = slides[i - 1] || slides[i + 1];
    if (res && prev) state.goAfterRender = prev.anchor;
  }
  if (res) await fetchDeck();
}

async function select(ref) {
  state.sel = ref;
  highlight();
  await renderPanel();
}

async function renderElementPanel() {
  const ref = state.sel;
  const cut = ref.lastIndexOf('/');
  const slide = await fetchSlide(ref.slice(0, cut));
  const block = slide?.blocks[Number(ref.slice(cut + 1))];
  if (!block) { state.sel = null; return renderPanel(); }
  const view = $('#element-panel').content.cloneNode(true);
  const f = $('form', view);
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
  panel.replaceChildren(view);
}

// ---------- undo / redo / keys ----------

async function undoRedo(kind) {
  const r = await op(kind);
  if (r) { await fetchDeck(); await renderPanel(); }
}
$('#undo').addEventListener('click', () => undoRedo('undo'));
$('#redo').addEventListener('click', () => undoRedo('redo'));

function onKey(e) {
  if (e.key === 'Escape' && state.sel) { e.preventDefault(); select(null); return; }
  const mod = e.metaKey || e.ctrlKey;
  if (!mod || e.key.toLowerCase() !== 'z' || editing()) return;
  e.preventDefault();
  undoRedo(e.shiftKey ? 'redo' : 'undo');
}
document.addEventListener('keydown', onKey);

// ---------- live updates ----------

const es = new EventSource('/events');
es.addEventListener('open', () => { statusEl.textContent = 'connected'; });
es.addEventListener('error', () => { statusEl.textContent = 'disconnected — is qd serve running?'; });
es.addEventListener('changed', async (e) => {
  statusEl.textContent = 'changed…';
  const { version } = JSON.parse(e.data);
  if (version !== state.version) {
    await fetchDeck();
    if (!editing()) await renderPanel();
  }
});
es.addEventListener('rendering', () => { statusEl.textContent = 'rendering…'; });
es.addEventListener('rendered', (e) => {
  const d = JSON.parse(e.data);
  statusEl.textContent = 'rendered ' + d.version.slice(0, 7);
  banner.hidden = true;
  load(d.html);
});
es.addEventListener('render_error', (e) => {
  const d = JSON.parse(e.data);
  statusEl.textContent = 'render failed (showing last good render)';
  banner.textContent = d.message;
  banner.hidden = false;
});
