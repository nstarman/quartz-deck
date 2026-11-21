// Dock the source pane by dragging its title bar (spec 022 US1): a zone over
// the window for each place it can go; a drop on one sets `source.side`
// (research R4). The pane itself never moves in the DOM: CSS lays it out.

import { el } from '/ui/run.js';

const main = document.querySelector('main');
const pane = document.querySelector('#srcpane');
const bar = pane.querySelector('.srcbar');
const zones = document.querySelector('#dock-zones');
const narrow = matchMedia('(max-width: 700px)'); // the editor stacks its columns
const STRIP = 96; // px: a zone's depth

/** The zones, placed from where everything is now. */
function show() {
  const m = main.getBoundingClientRect();
  const stage = document.querySelector('#stage').getBoundingClientRect();
  const s = stage.width ? stage : pane.getBoundingClientRect(); // Full width: the pane has the slide's room
  const at = {
    'far-left': ['Before the slide list', m.left, m.top, STRIP, m.height],
    left: ['Beside the slide list', s.left, s.top, STRIP, s.height - STRIP],
    right: ['Right of the slide', m.right - STRIP, m.top, STRIP, m.height],
    below: ['Below the slide', s.left, s.bottom - STRIP, s.width, STRIP],
  };
  zones.replaceChildren(...Object.entries(at).filter(([p]) => !narrow.matches || p === 'below').map(([p, [label, x, y, w, h]]) => {
    const z = el('div', { textContent: label });
    z.dataset.place = p;
    Object.assign(z.style, { left: x + 'px', top: y + 'px', width: w + 'px', height: h + 'px' });
    return z;
  }));
  zones.hidden = false;
}

const inside = (z, e) => {
  const r = z.getBoundingClientRect();
  return e.clientX >= r.left && e.clientX < r.right && e.clientY >= r.top && e.clientY < r.bottom;
};

/** `set(place)` is called on a drop that changes the place. */
export function init(set) {
  bar.addEventListener('pointerdown', (e) => {
    if (e.button !== 0 || e.target.closest('button, input, select, a')) return;
    let dragging = false;
    let over = null;
    const ac = new AbortController(); // ends every listener of this drag
    const on = (target, type, fn, capture = false) => target.addEventListener(type, fn, { signal: ac.signal, capture });
    // captured, so the moves still come here over the slide's iframe
    bar.setPointerCapture(e.pointerId);
    const move = (ev) => {
      if (!dragging) {
        if (Math.hypot(ev.clientX - e.clientX, ev.clientY - e.clientY) < 4) return;
        dragging = true;
        document.body.classList.add('docking');
        show();
      }
      over = [...zones.children].find((z) => inside(z, ev)) ?? null;
      for (const z of zones.children) z.classList.toggle('over', z === over);
    };
    const end = (drop) => {
      ac.abort();
      document.body.classList.remove('docking');
      zones.hidden = true;
      const place = drop && over?.dataset.place;
      if (place && place !== main.dataset.source) set(place);
    };
    const up = () => end(true);
    const cancel = () => end(false);
    const key = (ev) => {
      if (ev.key !== 'Escape') return;
      ev.preventDefault();
      ev.stopPropagation(); // Esc ends the drag, nothing else
      cancel();
    };
    on(bar, 'pointermove', move);
    on(bar, 'pointerup', up);
    on(bar, 'lostpointercapture', cancel); // another app, a dialog
    on(window, 'keydown', key, true);
  });
}
