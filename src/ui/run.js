// Running code cells (spec 004 US3): a ▶ beside each cell in the source pane,
// Run all / Stop / Restart and the kernel state in the source bar, and the
// outputs of the cell under the caret (or the one just run) in a dock below
// the source. Slides show stored outputs through the preview (research R7).
// Every request carries the session token (FR-015). It arrives in the link's
// fragment (`#t=…` or `#/slide&t=…`, from `qd serve` / `qd url`), never in the
// page: kept for this tab (reloads) and taken out of the address bar.

const store = (f) => { try { return f(sessionStorage); } catch { return null; } };
const given = location.hash.match(/(?:^#|&)t=([0-9a-f]+)/);
if (given) {
  store((s) => s.setItem('qd-token', given[1]));
  history.replaceState(null, '', location.pathname + location.search + location.hash.replace(given[0], ''));
}
export const token = given?.[1] || store((s) => s.getItem('qd-token')) || '';
const pane = document.querySelector('#srcpane');
const ta = pane.querySelector('textarea');
const gutter = pane.querySelector('.gutter');
const dock = pane.querySelector('.outdock');
const badge = pane.querySelector('.kernel');
const envBtn = pane.querySelector('[data-act=env]');

let api = null; // { say, flush }
let order = []; // [{cell, lines, runnable}]
let outs = {}; // cell → stored record
let focus = null; // cell shown in the dock
const running = new Set();
const el = (tag, props) => Object.assign(document.createElement(tag), props);
const fmt = (e) => `${e.message} — ${e.hint}`;

async function post(path, body = {}) {
  const r = await fetch(path, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-QD-Token': token }, body: JSON.stringify(body) });
  const v = await r.json().catch(() => ({}));
  if (!r.ok) api.say(v.error ? fmt(v.error) : r.status === 403 ? 'Running code needs the editor link from `qd serve` or `qd url`' : `request failed (${r.status})`, true);
  return r.ok ? v : null;
}

/** Run cells by ref (after saving the source pane, so the file has the latest text). */
export async function runCells(cells) {
  await api.flush();
  if (cells.length) focus = cells[cells.length - 1];
  return post('/api/run', { cells });
}
const runAll = async (fresh = false) => { await api.flush(); return post('/api/run', { all: true, fresh }); };

const cellAt = (line) => order.find((c) => c.lines[0] <= line && line <= c.lines[1]);
const caretLine = () => ta.value.slice(0, ta.selectionStart).split('\n').length;

async function refresh() {
  const r = await fetch('/api/outputs').then((x) => x.json()).catch(() => null);
  if (!r?.order) return;
  ({ order } = r);
  outs = r.cells || {};
  drawGutter();
  drawDock();
}

function drawGutter() {
  const css = getComputedStyle(ta);
  const lh = parseFloat(css.lineHeight) || 18;
  const pad = parseFloat(css.paddingTop) || 0;
  gutter.replaceChildren(...order.map((c) => {
    const o = outs[c.cell];
    const b = el('button', { type: 'button' });
    b.dataset.cell = c.cell;
    b.style.top = pad + (c.lines[0] - 1) * lh + 'px';
    b.disabled = !c.runnable;
    b.textContent = running.has(c.cell) ? '…' : o ? `▶ ${o.counter ?? ''}` : '▶';
    b.className = [o?.status === 'error' && 'err', o?.stale && 'stale', running.has(c.cell) && 'busy'].filter(Boolean).join(' ');
    b.title = c.runnable ? `Run this cell (⇧Enter)${o?.stale ? ' — output is stale' : ''}` : 'Not runnable here (eval: false or not Python)';
    b.setAttribute('aria-label', `Run cell ${c.cell}`);
    return b;
  }));
  syncScroll();
}

function syncScroll() {
  gutter.style.transform = `translateY(${-ta.scrollTop}px)`;
}

function drawDock() {
  const o = outs[focus];
  dock.hidden = !o;
  if (!o) return;
  const head = el('div', { className: 'head', textContent: `${focus} · [${o.counter ?? ' '}]${o.stale ? ' · stale: the cell changed since this ran' : ''}` });
  dock.replaceChildren(head, ...o.items.map((i) => {
    if (i.kind === 'image') return el('img', { src: i.url, alt: `figure from ${focus}` });
    // kernel HTML (tables, Plotly) never runs in the editor page: no scripts, own origin
    if (i.kind === 'html') return el('iframe', { sandbox: '', srcdoc: i.html, title: `HTML output of ${focus}` });
    if (i.kind === 'error') return el('pre', { className: 'error', textContent: i.traceback.join('\n') });
    const pre = el('pre', { textContent: i.text });
    if (i.full_url) pre.append(el('a', { href: i.full_url, target: '_blank', textContent: '\n… show all' }));
    return pre;
  }));
  dock.classList.toggle('stale', !!o.stale);
}

function caretMoved() {
  const c = cellAt(caretLine());
  if (c && c.cell !== focus) { focus = c.cell; drawDock(); }
}

async function envStatus() {
  const r = await fetch('/api/env');
  envBtn.hidden = r.ok || !order.length;
  envBtn.textContent = 'Create environment';
}

/** SSE events from the server (contracts/http.md). */
export function listen(es) {
  const on = (name, f) => es.addEventListener(name, (e) => f(JSON.parse(e.data)));
  on('kernel', ({ state }) => { badge.textContent = state === 'starting' ? 'starting…' : state; badge.dataset.state = state; });
  on('run_started', ({ cells }) => { cells.forEach((c) => running.add(c)); drawGutter(); });
  on('cell_output', (o) => { running.delete(o.cell); outs[o.cell] = { ...o, stale: false }; focus = o.cell; drawGutter(); drawDock(); });
  on('run_done', ({ status, error }) => {
    running.clear();
    if (error) api.say(fmt(error), true);
    else if (status === 'stopped') api.say('Stopped');
    refresh();
    envStatus();
  });
  on('changed', refresh);
}

export function initRun(a) {
  api = a;
  ta.addEventListener('scroll', syncScroll);
  ta.addEventListener('keyup', caretMoved);
  ta.addEventListener('click', caretMoved);
  ta.addEventListener('keydown', (e) => {
    if (e.key !== 'Enter' || !e.shiftKey) return;
    const c = cellAt(caretLine());
    if (!c?.runnable) return;
    e.preventDefault(); // ⇧Enter runs the cell, as in notebooks
    runCells([c.cell]);
  });
  gutter.addEventListener('click', (e) => {
    const b = e.target.closest('button[data-cell]');
    if (b) runCells([b.dataset.cell]);
  });
  pane.querySelector('.srcbar').addEventListener('click', async (e) => {
    const act = e.target.closest('button')?.dataset.act;
    if (act === 'run-all') runAll();
    else if (act === 'fresh-all') runAll(true);
    else if (act === 'stop') post('/api/run/interrupt');
    else if (act === 'restart') post('/api/run/restart');
    else if (act === 'env') {
      envBtn.disabled = true;
      envBtn.textContent = 'Creating…';
      if (await post('/api/env/init')) api.say('Environment created (uv): add packages with “qd env add”');
      envBtn.disabled = false;
      envStatus();
    } else if (act === 'add-pkg') {
      const p = prompt('Python packages to add (from PyPI), space-separated');
      if (p?.trim() && (await post('/api/env/add', { packages: p.trim().split(/\s+/) }))) api.say(`Added ${p.trim()}`);
    }
  });
  refresh().then(envStatus);
}
