// Diagrams on the slide (spec 015 US1): a box of a `{dot}` cell is selected
// by a click, dragged to a new place (its arrows become straight provisional
// lines until the drop), nudged with the arrow keys, and an unpinned diagram
// offers "Pin layout". Every change is a `qd dot` op (contracts/editor.md);
// the preview's next render draws the routed arrows.

import { token, el } from '/ui/run.js';

let api = null; // { elOf, frame, op, pageRect }
const info = new Map(); // cell ref → `qd dot show` result
let sel = null; // { cell, node }
let drag = null;
let dragged = false; // the click ending a drag lands on the svg, not the box
const pinBtn = el('button', { type: 'button', id: 'dotpin', hidden: true, textContent: 'Pin layout', title: 'Fix every box where it is now, so boxes can be placed by hand' });
document.body.append(pinBtn);

/** A read-only op (`dot show`), quiet: without the editor's saving status. */
const read = (op, args) => fetch('/api/op', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-QD-Token': token }, body: JSON.stringify({ op, args }) })
  .then((r) => (r.ok ? r.json() : null));

const nameOf = (g) => g.querySelector('title')?.textContent ?? '';
const diagramOf = (node) => node?.closest?.('[data-qd-lang=dot]');
const nodeAt = (t) => t?.closest?.('g.node');

const nodes = (cellEl) => [...cellEl.querySelectorAll('svg g.node')];
const nodeNamed = (cellEl, name) => nodes(cellEl).find((g) => nameOf(g) === name);

/** Centre of a node in the graph group's units. */
function centre(g) {
  const b = g.getBBox();
  return [b.x + b.width / 2, b.y + b.height / 2];
}

/** The edges touching `name`, with the name at their other end. */
function edgesOf(cellEl, name) {
  const out = [];
  for (const e of cellEl.querySelectorAll('svg g.edge')) {
    const [a, b] = (e.querySelector('title')?.textContent ?? '').split(/->|--/);
    if (a === name) out.push([e, b]);
    else if (b === name) out.push([e, a]);
  }
  return out;
}

function highlight() {
  const doc = api.frame.contentDocument;
  for (const g of doc?.querySelectorAll('.qd-dot-sel') || []) g.classList.remove('qd-dot-sel');
  if (!sel) return;
  const cellEl = api.elOf(sel.cell);
  if (cellEl) nodeNamed(cellEl, sel.node)?.classList.add('qd-dot-sel');
}

function select(cell, node) {
  sel = node ? { cell, node } : null;
  highlight();
}

/** Undo the provisional look of a drag. */
function restore(d) {
  d.g.removeAttribute('transform');
  for (const l of d.lines) l.line.remove();
  for (const [e] of d.edges) e.style.display = '';
}

function showPin(cellEl) {
  const r = api.pageRect(cellEl);
  pinBtn.dataset.cell = cellEl.dataset.qd;
  pinBtn.hidden = false;
  pinBtn.style.left = r.left + 4 + 'px';
  pinBtn.style.top = r.top + 4 + 'px';
}

function down(e) {
  const g = nodeAt(e.target);
  const cellEl = diagramOf(g);
  const about = cellEl && info.get(cellEl.dataset.qd);
  if (!g || about?.editable !== true || e.button !== 0) {
    if (sel) select(null); // a click anywhere else ends the box's selection
    return;
  }
  // the editor would select the whole cell or open its source: this is a box
  e.stopImmediatePropagation();
  e.preventDefault();
  const cell = cellEl.dataset.qd;
  const node = nameOf(g);
  select(cell, node);
  const graph = g.closest('g.graph');
  drag = { cell, node, g, cellEl, x: e.clientX, y: e.clientY, scale: graph.getScreenCTM().a, pinned: about.pinned, lines: [], edges: [] };
}

function move(e) {
  if (!drag) return;
  const dx = e.clientX - drag.x;
  const dy = e.clientY - drag.y;
  if (!drag.far && Math.hypot(dx, dy) < 3) return;
  drag.far = dragged = true;
  if (!drag.pinned) {
    showPin(drag.cellEl);
    return;
  }
  if (!drag.moved) {
    drag.moved = true;
    drag.edges = edgesOf(drag.cellEl, drag.node);
    const graph = drag.g.closest('g.graph');
    const ns = 'http://www.w3.org/2000/svg';
    for (const [edge, other] of drag.edges) {
      edge.style.display = 'none';
      const o = nodeNamed(drag.cellEl, other);
      const line = graph.ownerDocument.createElementNS(ns, 'line');
      line.setAttribute('class', 'qd-dot-provisional');
      graph.append(line);
      drag.lines.push({ line, to: o ? centre(o) : centre(drag.g) });
    }
  }
  const ux = dx / drag.scale;
  const uy = dy / drag.scale;
  drag.g.setAttribute('transform', `translate(${ux} ${uy})`);
  const [cx, cy] = centre(drag.g);
  for (const { line, to } of drag.lines) {
    line.setAttribute('x1', cx + ux);
    line.setAttribute('y1', cy + uy);
    line.setAttribute('x2', to[0]);
    line.setAttribute('y2', to[1]);
  }
  drag.ux = ux;
  drag.uy = uy;
}

async function up() {
  const d = drag;
  drag = null;
  if (d) dragged = d.far;
  if (!d?.moved || d.cancelled) return;
  // graph units are points; Graphviz's y grows upward
  const by = `${(d.ux / 72).toFixed(3)},${(-d.uy / 72).toFixed(3)}`;
  const r = await api.op('dot move', { cell: d.cell, node: d.node, by });
  if (!r) restore(d);
}

/** Arrange ▸ Nudge on a selected box of a pinned diagram: one grid step (0.25 in); y grows upward. */
export const nudgeable = () => !!sel && !drag && !!info.get(sel.cell)?.pinned;
export const nudge = (dx, dy) => api.op('dot move', { cell: sel.cell, node: sel.node, by: `${dx * 0.25},${-dy * 0.25}` });

/** Esc mid-drag puts the box back. */
function key(e) {
  if (e.key !== 'Escape' || !drag) return;
  e.stopImmediatePropagation();
  drag.cancelled = true;
  restore(drag);
  drag = null;
}

/** Positions read from the rendered SVG: centres in inches, lowest-left at 0, 0. */
function positions(cellEl) {
  const pts = nodes(cellEl).map((g) => {
    const [x, y] = centre(g);
    return [nameOf(g), x / 72, -y / 72];
  });
  const x0 = Math.min(...pts.map((p) => p[1]));
  const y0 = Math.min(...pts.map((p) => p[2]));
  const r = (v) => Math.round(v * 100) / 100;
  return Object.fromEntries(pts.map(([n, x, y]) => [n, [r(x - x0), r(y - y0)]]));
}

/** Pin layout: the prompt on a drag, and Arrange ▸ Pin layout. */
export async function pin(cell) {
  const cellEl = api.elOf(cell);
  pinBtn.hidden = true;
  if (cellEl) await api.op('dot pin', { cell, positions: JSON.stringify(positions(cellEl)) });
}

/** After every render: learn each diagram's state, wire the preview window once. */
export async function mark() {
  if (!api) return;
  const w = api.frame.contentWindow;
  const doc = api.frame.contentDocument;
  if (!w || !doc) return;
  pinBtn.hidden = true;
  if (!w.__qdDot) {
    w.__qdDot = true;
    // the window sees events before the editor's document listeners
    w.addEventListener('pointerdown', down, true);
    const onBox = (e) => info.get(diagramOf(nodeAt(e.target))?.dataset.qd)?.editable === true;
    for (const t of ['mousedown', 'click', 'dblclick']) {
      w.addEventListener(t, (e) => {
        if (onBox(e) || (t === 'click' && dragged)) e.stopImmediatePropagation();
        if (t === 'click') dragged = false;
      }, true);
    }
    w.addEventListener('pointermove', move, true);
    w.addEventListener('pointerup', up, true);
    w.addEventListener('keydown', key, true);
  }
  info.clear();
  for (const c of doc.querySelectorAll('[data-qd-lang=dot]')) {
    const v = await read('dot show', { cell: c.dataset.qd });
    if (v) info.set(c.dataset.qd, v);
  }
  if (sel && !info.has(sel.cell)) sel = null;
  highlight();
}

/** A diagram the editor can pin: readable, not pinned yet. */
export const pinnable = (cell) => info.get(cell)?.editable === true && !info.get(cell).pinned;

export const selected = () => sel;

export function initDot(a) {
  api = a;
  pinBtn.addEventListener('click', () => pin(pinBtn.dataset.cell));
  addEventListener('keydown', key, true);
  a.frame.addEventListener('load', () => { drag = null; });
}
