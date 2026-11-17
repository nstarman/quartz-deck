// Diagrams on the slide (spec 015): a box or an arrow of a `{dot}` cell is
// selected by a click. A box is dragged to a new place (its arrows become
// straight provisional lines until the drop), nudged with the arrow keys,
// renamed by a double-click, and connected by dragging from one of its
// handles onto another box, or onto empty space for a new box. An unpinned
// diagram offers "Pin layout". Every change is a `qd dot` op
// (contracts/editor.md); the preview's next render draws the routed arrows.
// With "Route arrows while dragging" on (US4), the arrows are routed live by
// Graphviz in the browser instead; without the engine, straight lines.

import { token, el } from '/ui/run.js';
import { ask } from '/ui/sheet.js';
import { get as setting } from '/ui/settings.js';

let api = null; // { elOf, frame, op, pageRect, select, panelChanged }
const info = new Map(); // cell ref → `qd dot show` result
let sel = null; // { cell, node } or { cell, edge: 'a->b#k' }
const last = new Map(); // cell → the box selected last (Insert ▸ Box takes its look)
let drag = null;
let dragged = false; // the click ending a drag lands on the svg, not the box
const pinBtn = el('button', { type: 'button', id: 'dotpin', hidden: true, textContent: 'Pin layout', title: 'Fix every box where it is now, so boxes can be placed by hand' });
document.body.append(pinBtn);
const NS = 'http://www.w3.org/2000/svg';

/** A read-only op (`dot show`), quiet: without the editor's saving status. */
const read = (op, args) => fetch('/api/op', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-QD-Token': token }, body: JSON.stringify({ op, args }) })
  .then((r) => (r.ok ? r.json() : null));

const nameOf = (g) => g.querySelector('title')?.textContent ?? '';
const diagramOf = (node) => node?.closest?.('[data-qd-lang=dot]');
const nodeAt = (t) => t?.closest?.('g.node');
const edgeAt = (t) => t?.closest?.('g.edge');
const editable = (cellEl) => !!cellEl && info.get(cellEl.dataset.qd)?.editable === true;

const nodes = (cellEl) => [...cellEl.querySelectorAll('svg g.node')];
const nodeNamed = (cellEl, name) => nodes(cellEl).find((g) => nameOf(g) === name);
const edges = (cellEl) => [...cellEl.querySelectorAll('svg g.edge')];
/** `a->b#k` (`a--b#k` in an undirected graph). */
const op = (d) => (d?.directed === false ? '--' : '->');
const refOf = (d, e) => `${e.from}${op(d)}${e.to}#${e.index}`;
const boxInfo = (cell, name) => info.get(cell)?.nodes?.find((n) => n.name === name);

/** An edge's ends from its title (`a:p->b`), ports dropped. */
const ends = (g) => nameOf(g).split(/->|--/).map((s) => s.replace(/:[^:]*$/, ''));

/** `a->b#k`: the k-th edge with these ends, in the SVG's order (the source's). */
function edgeRef(cellEl, g) {
  const t = nameOf(g);
  const same = edges(cellEl).filter((e) => nameOf(e) === t);
  return `${ends(g).join(op(info.get(cellEl.dataset.qd)))}#${same.indexOf(g) + 1}`;
}

function edgeNamed(cellEl, ref) {
  return edges(cellEl).find((g) => edgeRef(cellEl, g) === ref);
}

/** Centre of a node in the graph group's units. */
function centre(g) {
  const b = g.getBBox();
  return [b.x + b.width / 2, b.y + b.height / 2];
}

/** A page point in the graph group's units. */
function toGraph(graph, x, y) {
  return new DOMPoint(x, y).matrixTransform(graph.getScreenCTM().inverse());
}

/** The edges touching `name`, with the name at their other end. */
function edgesOf(cellEl, name) {
  const out = [];
  for (const e of edges(cellEl)) {
    const [a, b] = ends(e);
    if (a === name) out.push([e, b]);
    else if (b === name) out.push([e, a]);
  }
  return out;
}

/** The selection's look, and a selected box's four connect handles. */
function highlight() {
  const doc = api.frame.contentDocument;
  for (const g of doc?.querySelectorAll('.qd-dot-sel') || []) g.classList.remove('qd-dot-sel');
  for (const h of doc?.querySelectorAll('.qd-dot-handle') || []) h.remove();
  if (!sel) return;
  const cellEl = api.elOf(sel.cell);
  if (!cellEl) return;
  if (sel.edge) { edgeNamed(cellEl, sel.edge)?.classList.add('qd-dot-sel'); return; }
  const g = nodeNamed(cellEl, sel.node);
  if (!g) return;
  g.classList.add('qd-dot-sel');
  const b = g.getBBox();
  const label = boxInfo(sel.cell, sel.node)?.label ?? sel.node;
  for (const [side, x, y] of [['right', b.x + b.width, b.y + b.height / 2], ['left', b.x, b.y + b.height / 2], ['top', b.x + b.width / 2, b.y], ['bottom', b.x + b.width / 2, b.y + b.height]]) {
    const h = doc.createElementNS(NS, 'circle');
    Object.entries({ class: 'qd-dot-handle', cx: x, cy: y, r: 5, role: 'button', 'aria-label': `Connect from ${label}`, 'data-side': side }).forEach(([k, v]) => h.setAttribute(k, v));
    g.closest('g.graph').append(h);
  }
}

/** Select a box (`{node}`) or an arrow (`{edge}`) of `cell`, or nothing; the cell becomes the editor's element. */
function select(cell, what) {
  sel = what ? { cell, ...what } : null;
  if (sel?.node) last.set(cell, sel.node);
  highlight();
  if (sel) api.select(cell);
}

/**
 * The Graphviz of the person's own Quarto, the build that renders the deck,
 * loaded once (`null` if it can't be had: straight lines). Quarto made it for
 * Deno, and it reads its engine with `Deno.readFileSync`: that one call is
 * answered with the engine fetched beforehand (research R6).
 */
let engine;
const graphviz = () => (engine ??= (async () => {
  const r = await fetch('/lib/quarto/graphvizlib.wasm');
  if (!r.ok) return null;
  const wasm = new Uint8Array(await r.arrayBuffer());
  globalThis.Deno ??= { env: { get: () => '' }, readFileSync: () => wasm };
  return (await import('/lib/quarto/graphviz-wasm.js')).graphviz();
})().catch(() => null));

/** A label's spot (`<text x y>`), the same in two layouts of a pinned diagram where the box stayed. */
const spot = (g) => ['x', 'y'].map((k) => Number(g?.querySelector('text')?.getAttribute(k)));

/** The cell's layout with the dragged box where it is now, as an SVG document (or null). */
async function layout(d) {
  const gv = await graphviz();
  const v = info.get(d.cell);
  const at = boxInfo(d.cell, d.node)?.pos;
  if (!gv || !v?.text || !at) return null;
  const pos = `${at[0] + d.ux / 72},${at[1] - d.uy / 72}!`;
  const text = v.text.replace(/\}\s*$/, `${JSON.stringify(d.node)} [pos="${pos}"];\n}\n`);
  return new DOMParser().parseFromString(await gv.layout(text, 'svg', v.settings?.layout ?? 'neato'), 'image/svg+xml');
}

/**
 * Live routing: lay the cell out with the dragged box where it is now, and draw
 * its arrows as routed there, at most once a frame. The saved text is untouched.
 */
function route(d) {
  if (d.frame || !setting('diagrams.live-layout')) return;
  d.frame = requestAnimationFrame(async () => {
    const svg = await layout(d).catch(() => null);
    d.frame = 0; // the next move lays out again
    if (!svg || drag !== d) return;
    // the two layouts' frames differ by the bounding box: line them up on a box that stayed
    const other = nodes(d.cellEl).find((g) => nameOf(g) !== d.node);
    const twin = nodeNamed(svg, nameOf(other));
    const [x0, y0] = spot(other);
    const [x1, y1] = spot(twin);
    if (![x0, y0, x1, y1].every(Number.isFinite)) return;
    const all = edges(d.cellEl);
    const routed = edges(svg);
    const graph = d.g.closest('g.graph');
    for (const r of d.routed) r.remove();
    d.routed = d.edges.map(([e]) => {
      const r = routed[all.indexOf(e)];
      r.classList.add('qd-dot-routed');
      r.setAttribute('transform', `translate(${x0 - x1} ${y0 - y1})`);
      graph.insertBefore(r, graph.querySelector('g.node'));
      return r;
    });
    for (const { line: l } of d.lines) l.style.display = 'none';
  });
}

/** Undo the provisional look of a drag. */
function restore(d) {
  d.g?.removeAttribute('transform');
  cancelAnimationFrame(d.frame);
  for (const r of d.routed) r.remove();
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

function line(graph, cls) {
  const l = graph.ownerDocument.createElementNS(NS, 'line');
  l.setAttribute('class', cls);
  graph.append(l);
  return l;
}

function down(e) {
  const cellEl = diagramOf(e.target);
  const g = nodeAt(e.target);
  const edge = !g && edgeAt(e.target);
  const handle = e.target.closest?.('.qd-dot-handle');
  if (!editable(cellEl) || !(g || edge || handle)) {
    if (sel) select(null); // a click anywhere else ends the selection
    return;
  }
  const cell = cellEl.dataset.qd;
  const what = g ? { node: nameOf(g) } : edge ? { edge: edgeRef(cellEl, edge) } : null;
  if (e.button === 2) { // right-click: the editor's menu, about this box or arrow
    if (what) select(cell, what);
    return;
  }
  if (e.button !== 0) return;
  // the editor would select the whole cell or open its source: this is a box
  e.stopImmediatePropagation();
  e.preventDefault();
  const graph = cellEl.querySelector('svg g.graph');
  const base = { cell, cellEl, x: e.clientX, y: e.clientY, scale: graph.getScreenCTM().a, pinned: info.get(cell).pinned, lines: [], edges: [], routed: [] };
  if (handle) { // connect: a line follows the pointer
    const from = sel.node;
    const l = line(graph, 'qd-dot-follow');
    l.setAttribute('x1', handle.getAttribute('cx'));
    l.setAttribute('y1', handle.getAttribute('cy'));
    drag = { ...base, connect: true, from, graph, lines: [{ line: l }] };
    return;
  }
  select(cell, what);
  if (g) drag = { ...base, node: what.node, g };
  if (g && setting('diagrams.live-layout')) graphviz(); // ready by the first move
}

function move(e) {
  if (!drag) return;
  const dx = e.clientX - drag.x;
  const dy = e.clientY - drag.y;
  if (!drag.far && Math.hypot(dx, dy) < 3) return;
  drag.far = dragged = true;
  if (drag.connect) {
    const p = toGraph(drag.graph, e.clientX, e.clientY);
    drag.lines[0].line.setAttribute('x2', p.x);
    drag.lines[0].line.setAttribute('y2', p.y);
    return;
  }
  if (!drag.pinned) {
    showPin(drag.cellEl);
    return;
  }
  if (!drag.moved) {
    drag.moved = true;
    drag.edges = edgesOf(drag.cellEl, drag.node);
    const graph = drag.g.closest('g.graph');
    for (const [edge, other] of drag.edges) {
      edge.style.display = 'none';
      const o = nodeNamed(drag.cellEl, other);
      drag.lines.push({ line: line(graph, 'qd-dot-provisional'), to: o ? centre(o) : centre(drag.g) });
    }
  }
  const ux = dx / drag.scale;
  const uy = dy / drag.scale;
  drag.g.setAttribute('transform', `translate(${ux} ${uy})`);
  const [cx, cy] = centre(drag.g);
  for (const { line: l, to } of drag.lines) {
    l.setAttribute('x1', cx + ux);
    l.setAttribute('y1', cy + uy);
    l.setAttribute('x2', to[0]);
    l.setAttribute('y2', to[1]);
  }
  drag.ux = ux;
  drag.uy = uy;
  route(drag);
}

/** A connect drag's drop: on a box, an arrow to it; on empty space in the diagram, a new box there. */
function drop(d, e) {
  restore(d);
  if (!d.far) return null;
  const target = api.frame.contentDocument.elementFromPoint(e.clientX, e.clientY);
  const to = nodeAt(target);
  if (to && diagramOf(to) === d.cellEl) {
    return nameOf(to) === d.from ? null : api.op('dot connect', { cell: d.cell, from: d.from, to: nameOf(to) });
  }
  if (diagramOf(target) !== d.cellEl) return null;
  const args = { cell: d.cell, from: d.from, 'look-of': d.from };
  const ref = boxInfo(d.cell, d.from);
  const g = nodeNamed(d.cellEl, d.from);
  if (d.pinned && ref?.pos && g) { // the drop point in inches, from the box it started at
    const p = toGraph(d.graph, e.clientX, e.clientY);
    const [cx, cy] = centre(g);
    args.at = `${(ref.pos[0] + (p.x - cx) / 72).toFixed(3)},${(ref.pos[1] - (p.y - cy) / 72).toFixed(3)}`;
  }
  return api.op('dot add', args).then((r) => { if (r?.node) select(d.cell, { node: r.node }); return r; });
}

async function up(e) {
  const d = drag;
  drag = null;
  if (d) dragged = d.far;
  if (d?.connect) { await drop(d, e); return; }
  if (!d?.moved || d.cancelled) return;
  // graph units are points; Graphviz's y grows upward
  const by = `${(d.ux / 72).toFixed(3)},${(-d.uy / 72).toFixed(3)}`;
  const r = await api.op('dot move', { cell: d.cell, node: d.node, by });
  if (!r) restore(d);
}

/** Double-click a box: rename it in a sheet beside it (Enter renames, Escape cancels). */
async function rename(cellEl, g) {
  const cell = cellEl.dataset.qd;
  const node = nameOf(g);
  const was = boxInfo(cell, node)?.label ?? node;
  const v = await ask({ title: 'Rename box', fields: [{ name: 'label', kind: 'text', label: 'Label', value: was }], ok: 'Rename', anchor: api.pageRect(g) });
  if (v && v.label !== was) await api.op('dot rename', { cell, node, label: v.label });
}

/** Arrange ▸ Nudge on a selected box of a pinned diagram: one grid step (0.25 in); y grows upward. */
export const nudgeable = () => !!sel?.node && !drag && !!info.get(sel.cell)?.pinned;
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
    const onBox = (e) => editable(diagramOf(e.target)) && !!(nodeAt(e.target) || edgeAt(e.target) || e.target.closest('.qd-dot-handle'));
    for (const t of ['mousedown', 'click', 'dblclick']) {
      w.addEventListener(t, (e) => {
        if (onBox(e) || (t === 'click' && dragged)) {
          e.stopImmediatePropagation();
          // a box with a link (URL=) sits in an <a>: in the editor a click selects it, never follows the link
          if (t === 'click') e.preventDefault();
        }
        if (t === 'click') dragged = false;
        if (t === 'dblclick' && onBox(e) && nodeAt(e.target)) rename(diagramOf(e.target), nodeAt(e.target));
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
    // an arrow is a thin line: a wider, invisible copy takes the clicks
    if (v?.editable === true) {
      for (const p of c.querySelectorAll('svg g.edge > path:not(.qd-dot-hit)')) {
        const hit = p.cloneNode();
        hit.setAttribute('class', 'qd-dot-hit');
        p.before(hit);
      }
    }
  }
  // the box or arrow is gone (deleted, undone, reversed elsewhere)
  const d = sel && info.get(sel.cell);
  if (sel && !(sel.node ? d?.nodes?.some((n) => n.name === sel.node) : d?.edges?.some((e) => refOf(d, e) === sel.edge))) sel = null;
  highlight();
  if (sel) api.panelChanged(); // the Properties panel shows the box as it is now
}

/** A diagram qd can edit. */
export const diagram = (cell) => info.get(cell)?.editable === true;
/** A diagram the editor can pin: readable, not pinned yet. */
export const pinnable = (cell) => diagram(cell) && !info.get(cell).pinned;

export const selected = () => sel;
/** A box or arrow of `cell` is selected. */
export const selectedIn = (cell) => sel?.cell === cell;
/** An arrow that can be reversed is selected in `cell`. */
export const flippable = (cell) => selectedIn(cell) && !!sel.edge && info.get(cell)?.directed;

/** The editor selected something else: the box or arrow is no longer selected. */
export function follow(ref) {
  if (sel && sel.cell !== ref) select(null);
}

/** Delete the selected box (with its arrows) or arrow. */
export async function remove() {
  const s = sel;
  select(null);
  return api.op('dot rm', { cell: s.cell, target: s.node ?? s.edge });
}

/** Reverse the selected arrow; it stays selected. */
export const flip = () => api.op('dot flip', { cell: sel.cell, edge: sel.edge }).then((r) => { if (r?.edge && sel) sel.edge = r.edge; return r; });

/** Insert ▸ Box: a new box in the look of the box selected last. */
export function addBox(cell) {
  const like = last.get(cell) ?? info.get(cell)?.nodes?.[0]?.name;
  return api.op('dot add', like ? { cell, 'look-of': like } : { cell });
}

const STARTERS = { tool: 'Tool (dark)', artifact: 'Artifact (light)' };

/** The Properties panel's fields for the selected box or arrow of `cell`, or null. */
export function panel(cell) {
  const d = info.get(cell);
  if (!selectedIn(cell) || !d) return null;
  const s = sel;
  const form = el('form', { className: 'dot-form' });
  const fs = el('fieldset');
  const row = (label, input) => { const l = el('label', { textContent: label }); l.append(input); fs.append(l); return input; };
  const buttons = el('div', { className: 'buttons' });
  const button = (text, run, cls = '') => { const b = el('button', { type: 'button', textContent: text, className: cls }); b.addEventListener('click', run); buttons.append(b); };
  form.append(fs, buttons);
  if (s.edge) {
    const e = d.edges.find((x) => refOf(d, x) === s.edge);
    fs.append(el('legend', { textContent: `Arrow ${s.edge.replace(/#1$/, '')}` }));
    if (d.directed) {
      const head = row('Arrowhead ', el('input', { type: 'checkbox', name: 'head', checked: e?.attrs?.arrowhead !== 'none' }));
      head.addEventListener('change', () => api.op('dot set', { cell, target: s.edge, pairs: [`arrowhead=${head.checked ? '' : 'none'}`] }));
      button('Reverse', () => flip());
    }
    button('Delete arrow', () => remove(), 'danger');
    return form;
  }
  const n = boxInfo(cell, s.node) || { name: s.node, look: {}, size: {} };
  const legend = el('legend', { textContent: 'Box ' });
  legend.append(el('code', { textContent: s.node }));
  fs.append(legend);
  const label = row('Label', el('input', { type: 'text', name: 'label', value: n.label ?? s.node }));
  const look = row('Look', el('select', { name: 'look' }));
  const key = (l) => JSON.stringify(Object.entries(l).sort());
  const seen = new Set([key(n.look)]);
  look.append(el('option', { value: '', textContent: 'As it is' }));
  for (const m of d.nodes) { // the looks in this diagram, each by its first box
    if (!Object.keys(m.look).length || seen.has(key(m.look))) continue;
    seen.add(key(m.look));
    look.append(el('option', { value: `of:${m.name}`, textContent: `Like “${m.label}”` }));
  }
  for (const { name, look: l } of d.starters) { // unless the diagram has it already
    if (!seen.has(key(l))) look.append(el('option', { value: `starter:${name}`, textContent: STARTERS[name] }));
  }
  const link = row('Link', el('input', { type: 'url', name: 'link', value: n.url ?? '', placeholder: 'https://…' }));
  // a picture at the box's left, drawn by the dot-image filter (US3)
  const image = row('Picture', el('input', { type: 'text', name: 'image', value: n.image ?? '', placeholder: 'face.png, beside the deck' }));
  const width = row('Width (in)', el('input', { type: 'number', name: 'width', step: '0.25', min: '0', value: n.size.width ?? '' }));
  const height = row('Height (in)', el('input', { type: 'number', name: 'height', step: '0.25', min: '0', value: n.size.height ?? '' }));
  buttons.append(el('button', { type: 'submit', className: 'primary', textContent: 'Apply' }));
  button('Delete box', () => remove(), 'danger');
  form.addEventListener('submit', (ev) => {
    ev.preventDefault();
    const pairs = [];
    const was = { label: n.label ?? s.node, URL: n.url ?? '', image: n.image ?? '', width: n.size.width ?? '', height: n.size.height ?? '' };
    for (const [k, v] of [['label', label.value], ['URL', link.value.trim()], ['image', image.value.trim()], ['width', width.value], ['height', height.value]]) {
      if (String(v) !== String(was[k])) pairs.push(`${k}=${v}`);
    }
    const args = { cell, target: s.node, pairs };
    const [how, which] = look.value.split(':');
    if (how === 'of') args['look-of'] = which;
    if (how === 'starter') args.look = which;
    if (pairs.length || how) api.op('dot set', args);
  });
  return form;
}

export function initDot(a) {
  api = a;
  pinBtn.addEventListener('click', () => pin(pinBtn.dataset.cell));
  addEventListener('keydown', key, true);
  a.frame.addEventListener('load', () => { drag = null; });
}
