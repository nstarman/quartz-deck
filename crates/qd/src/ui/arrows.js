// Arrows on the slide (spec 016): quarto-arrows shortcodes placed in slide
// pixels. A click on an arrow's line selects it; its start and end handles
// drag one end, its line drags the whole arrow, with the editor's snap
// guides (⌘/Ctrl places freely); Insert ▸ Arrow draws a new one. Every
// change is a `qd arrow` command (contracts/editor.md); the preview's next
// render draws the result.

import { read } from '/ui/run.js';
import { get as setting } from '/ui/settings.js';
import { SNAP_PX } from '/ui/guides.js';

let api = null; // { frame, op, select, guideLines, snap, clearGuides }
const info = new Map(); // ref → `qd arrow show` result
let sel = null; // the selected arrow's ref
let drag = null;
let drawing = null; // the slide id Insert ▸ Arrow draws on
const NS = 'http://www.w3.org/2000/svg';

const win = () => api.frame.contentWindow;
const doc = () => api.frame.contentDocument;
const wrapper = (ref) => doc()?.querySelector(`[data-qd="${CSS.escape(ref)}"]`);
const drawn = (el) => el?.querySelector('div[style*="position: absolute"]'); // quarto-arrows' own div
const editable = (ref) => info.get(ref)?.editable === true;
const xy = (p) => `${Math.round(p[0])},${Math.round(p[1])}`;

/** A page point in the current slide's pixels. */
function slidePoint(e) {
  const w = win();
  const r = w.Reveal.getCurrentSlide().getBoundingClientRect();
  const s = w.Reveal.getScale();
  return [(e.clientX - r.left) / s, (e.clientY - r.top) / s];
}

const attrs = (n, a) => (Object.entries(a).forEach(([k, v]) => n.setAttribute(k, v)), n);

function el(name, a, parent) {
  const n = attrs(doc().createElementNS(NS, name), a);
  parent?.append(n);
  return n;
}

const clearUI = () => doc()?.querySelectorAll('svg.qd-arrow-ui').forEach((u) => u.replaceChildren());

/** The overlay of the current slide: handles and provisional lines, in slide pixels. */
function overlay() {
  const sec = win().Reveal.getCurrentSlide();
  return sec.querySelector(':scope > svg.qd-arrow-ui')
    || el('svg', { class: 'qd-arrow-ui', style: 'position:absolute;left:0;top:0;width:1px;height:1px;overflow:visible;z-index:10000;pointer-events:none' }, sec);
}

/** The selected arrow's handles (or none). */
function handles() {
  clearUI();
  const a = sel && editable(sel) && info.get(sel);
  if (!a || !wrapper(sel)?.closest('section')?.classList.contains('present')) return;
  const r = 7 / win().Reveal.getScale(); // 7 screen px
  for (const [end, [x, y], label] of [['from', a.from, 'Arrow start'], ['to', a.to, 'Arrow end']]) {
    el('circle', { class: 'qd-arrow-handle', cx: x, cy: y, r, 'data-end': end, role: 'button', 'aria-label': label }, overlay());
  }
}

/** Select an arrow (`ref`), or nothing; called by the editor's own selection too. */
export function follow(ref) {
  sel = ref && info.has(ref) ? ref : null;
  handles();
}

export const nudgeable = () => !!sel && editable(sel) && !drag;
export const nudge = (dx, dy) => api.op('arrow move', { arrow: sel, by: `${dx},${dy}` });

/** Insert ▸ Arrow: the next drag on the slide draws one (a click: 200 px to the right). */
export function startDraw(slideId) {
  drawing = slideId;
  doc().documentElement.classList.add('qd-drawing');
}

function stopDraw() {
  drawing = null;
  doc()?.documentElement.classList.remove('qd-drawing');
}

/** Snap point `p` (or the box of `ps`) to the guides; returns the shift. */
function snapped(d, ps, e) {
  api.clearGuides(d.sec);
  if (!d.lines || e.metaKey || e.ctrlKey) return [0, 0];
  const xs = ps.map((p) => p[0]);
  const ys = ps.map((p) => p[1]);
  const [l, t] = [Math.min(...xs), Math.min(...ys)];
  const r = { l, t, w: Math.max(...xs) - l, h: Math.max(...ys) - t };
  api.snap(r, null, d.lines, d.sec, SNAP_PX / d.scale);
  return [r.l - l, r.t - t];
}

/** A dashed line from `a` to `b` while drawing or dragging an end. */
function provisional(d, a, b) {
  d.line ??= el('line', { class: 'qd-arrow-provisional' }, overlay());
  attrs(d.line, { x1: a[0], y1: a[1], x2: b[0], y2: b[1], 'stroke-width': 2 });
}

function begin(e, d) {
  e.stopImmediatePropagation();
  e.preventDefault();
  const sec = win().Reveal.getCurrentSlide();
  drag = { ...d, sec, scale: win().Reveal.getScale(), start: slidePoint(e), lines: setting('placement.snap') && api.guideLines(sec, d.el) };
}

function down(e) {
  if (e.button !== 0) return;
  if (drawing) {
    if (!e.target.closest?.('section.present')) return;
    begin(e, { kind: 'draw' });
    return;
  }
  const h = e.target.closest?.('.qd-arrow-handle');
  const hit = !h && e.target.closest?.('.qd-arrow-hit');
  if (!h && !hit) return;
  const ref = h ? sel : hit.dataset.ref;
  if (!editable(ref)) return;
  if (sel !== ref) api.select(ref);
  const a = info.get(ref);
  begin(e, { kind: h ? h.dataset.end : 'move', ref, a, el: wrapper(ref) });
}

function move(e) {
  if (!drag) return;
  const p = slidePoint(e);
  const d = [p[0] - drag.start[0], p[1] - drag.start[1]];
  if (!drag.far && Math.hypot(d[0], d[1]) * drag.scale < 3) return;
  drag.far = true;
  if (drag.kind === 'move') {
    const [sx, sy] = snapped(drag, [drag.a.from, drag.a.to].map((q) => [q[0] + d[0], q[1] + d[1]]), e);
    drag.by = [d[0] + sx, d[1] + sy];
    drawn(drag.el).style.transform = `translate(${drag.by[0]}px, ${drag.by[1]}px)`;
    clearUI();
    return;
  }
  // an end, or the new arrow's end: snapped to the guides
  const [sx, sy] = snapped(drag, [p], e);
  drag.at = [p[0] + sx, p[1] + sy];
  if (drag.kind === 'draw') { provisional(drag, drag.start, drag.at); return; }
  drawn(drag.el).style.visibility = 'hidden';
  for (const h of doc().querySelectorAll('.qd-arrow-handle')) h.remove();
  provisional(drag, drag.kind === 'from' ? drag.a.to : drag.a.from, drag.at);
}

/** Undo a drag's provisional look. */
function restore(d) {
  api.clearGuides(d.sec);
  d.line?.remove();
  const a = d.el && drawn(d.el);
  if (a) Object.assign(a.style, { transform: '', visibility: '' });
  handles();
}

async function up(e) {
  const d = drag;
  drag = null;
  if (!d) return;
  e.stopImmediatePropagation();
  if (d.kind === 'draw') {
    const slide = drawing;
    stopDraw();
    restore(d);
    const from = d.start;
    const to = d.far ? d.at : [from[0] + 200, from[1]];
    await api.op('arrow add', { slide, from: xy(from), to: xy(to) });
    return;
  }
  if (!d.far) { restore(d); return; }
  api.clearGuides(d.sec);
  const args = d.kind === 'move' ? { by: xy(d.by) } : { [d.kind]: xy(d.at) };
  if (!(await api.op('arrow move', { arrow: d.ref, ...args }))) restore(d);
}

/** Escape: a drag puts things back; drawing stops. */
function key(e) {
  if (e.key !== 'Escape' || !(drag || drawing)) return;
  e.stopImmediatePropagation();
  if (drag) { restore(drag); drag = null; }
  stopDraw();
}

/** After every render: read the slide's arrows, give each editable one a hit path. */
export async function mark() {
  if (!api) return;
  const w = win();
  const d = doc();
  if (!w || !d) return;
  if (!w.__qdArrows) {
    w.__qdArrows = true;
    w.addEventListener('pointerdown', down, true);
    // a click on an arrow selects it: the editor would start typing in its paragraph
    for (const t of ['mousedown', 'click', 'dblclick']) {
      w.addEventListener(t, (e) => {
        if (e.target.closest?.('.qd-arrow-hit, .qd-arrow-handle') || drag || (drawing && t !== 'dblclick')) {
          e.stopImmediatePropagation();
          e.preventDefault();
        }
      }, true);
    }
    w.addEventListener('pointermove', move, true);
    w.addEventListener('pointerup', up, true);
    w.addEventListener('keydown', key, true);
  }
  info.clear();
  const slides = new Set([...d.querySelectorAll('[data-qd]')].filter((x) => drawn(x)?.querySelector('svg')).map((x) => x.dataset.qd.split('/')[0]));
  for (const s of slides) {
    for (const a of (await read('arrow list', { slide: s }))?.arrows || []) info.set(a.ref, a);
  }
  for (const [ref, a] of info) {
    const x = wrapper(ref);
    const line = drawn(x)?.querySelector(':scope > svg > path');
    if (!x || !line || a.editable !== true) continue;
    x.classList.add('qd-arrow');
    const hit = line.cloneNode();
    for (const k of ['marker-end', 'marker-start', 'stroke-dasharray']) hit.removeAttribute(k);
    attrs(hit, { class: 'qd-arrow-hit', 'data-ref': ref, stroke: 'transparent', 'stroke-width': 14, fill: 'none' });
    line.after(hit);
  }
  follow(sel);
}

export function initArrows(a) {
  api = a;
  addEventListener('keydown', key, true);
  a.frame.addEventListener('load', () => { drag = null; stopDraw(); });
}
