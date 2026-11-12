// Run on the slide (spec 013 US1): a ▶ on the cell under the mouse (or the
// selected one) runs it, and ■ stops it. Each cell's state is a class on it in
// the preview, and a run's output is swapped into the slide as soon as the
// cell ends, before the preview renders again (research R6). Editor only: the
// presented deck and a page without the session token have no ▶ (FR-008).

import { token, el, runCells, stopRun, cellState, runnable } from '/ui/run.js';

const STATES = ['running', 'error', 'stopped', 'stale'];
const btn = el('button', { type: 'button', id: 'runbtn', hidden: true });
document.body.append(btn);
let api = null; // { elOf(ref), frame, pageRect(el), selected() }
let hovered = null;

/** The ▶ (■ while running) at the top right of the hovered cell, else the selected one. */
export function place() {
  if (!api) return;
  const ref = [hovered, api.selected()].find((r) => r && runnable(r));
  const c = token && ref && api.elOf(ref);
  btn.hidden = !c;
  if (!c) return;
  const busy = cellState(ref) === 'running';
  btn.dataset.cell = ref;
  btn.textContent = busy ? '■' : '▶';
  btn.title = busy ? 'Stop this cell' : 'Run this cell (⇧Enter)';
  btn.setAttribute('aria-label', `${busy ? 'Stop' : 'Run'} cell ${ref}`);
  const r = api.pageRect(c);
  btn.style.left = r.left + r.width - btn.offsetWidth - 4 + 'px';
  btn.style.top = r.top + 4 + 'px';
}

/** Each cell's state as a class on it (again after every render), then the ▶. */
export function mark() {
  if (!api) return;
  for (const c of api.frame.contentDocument?.querySelectorAll('[data-qd]') || []) {
    const s = runnable(c.dataset.qd) ? cellState(c.dataset.qd) : '';
    for (const k of STATES) c.classList.toggle(`qd-${k}`, k === s);
  }
  place();
}

/** The mouse is over `node` in the preview. */
export function hover(node) {
  hovered = node?.closest?.('[data-qd]')?.dataset.qd ?? null;
  place();
}

/** A cell ended: its output block, swapped in place (or added) at once. */
function swap({ cell, html }) {
  const c = html && api.elOf(cell);
  if (!c) return;
  const old = c.querySelector('.qd-output');
  if (old) old.outerHTML = html;
  else c.insertAdjacentHTML('beforeend', html);
}

export function initRunSlide(a, es) {
  api = a;
  btn.addEventListener('click', () => {
    const ref = btn.dataset.cell;
    if (cellState(ref) === 'running') stopRun();
    else runCells([ref]);
  });
  a.frame.addEventListener('mouseleave', (e) => { if (e.relatedTarget !== btn) hover(null); });
  es.addEventListener('cell_output', (e) => swap(JSON.parse(e.data)));
}
