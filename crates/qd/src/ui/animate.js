// The column's Animate view (spec 023): how the slide comes in, how the
// selected element appears and leaves on a click, and the slide's builds in
// the order they show. Each control writes through a CLI op (`qd set`,
// `qd build set`, `qd build order`), so the deck's text is the op's.

import { el } from '/ui/run.js';
import { TRANSITIONS } from '/ui/actions.js';
const SPEEDS = ['fast', 'slow'];
const APPEARS = [['fade-in', 'Fade in'], ['fade-up', 'Fade up'], ['fade-down', 'Fade down'], ['fade-left', 'Fade left'], ['fade-right', 'Fade right'],
  ['grow', 'Grow'], ['shrink', 'Shrink'], ['strike', 'Strike through'], ['highlight-red', 'Turn red'], ['highlight-green', 'Turn green'],
  ['highlight-blue', 'Turn blue'], ['highlight-current-red', 'Red while current'], ['highlight-current-green', 'Green while current'],
  ['highlight-current-blue', 'Blue while current']];
const LEAVES = [['fade-out', 'Fade out'], ['semi-fade-out', 'Fade part way']];
const NAMES = Object.fromEntries([...APPEARS, ...LEAVES]);

export const label = (...kids) => { const l = el('label', {}); l.append(...kids); return l; };
export function select(name, options, value, change) {
  const s = el('select', { name });
  s.append(...options.map(([v, t]) => el('option', { value: v, textContent: t })));
  s.value = value;
  s.addEventListener('change', () => change(s.value));
  return s;
}
export function check(name, text, on, change) {
  const box = el('input', { type: 'checkbox', name, checked: on });
  box.addEventListener('change', () => change(box.checked));
  return label(box, ' ' + text);
}

/** The slide's builds, in the order they show; drag a row, or ⌥↑ ⌥↓, to reorder. */
function buildList(builds, current, order) {
  const wrap = el('section', { className: 'builds' });
  wrap.append(el('h3', { textContent: 'Builds' }));
  if (!builds.length) {
    wrap.append(el('p', { className: 'hint', textContent: 'Nothing on this slide appears on a click.' }));
    return wrap;
  }
  const movable = builds.filter((b) => !b.items).map((b) => b.ref);
  const move = (ref, to) => {
    const refs = movable.filter((r) => r !== ref);
    refs.splice(Math.max(0, Math.min(to, refs.length)), 0, ref);
    if (refs.join() !== movable.join()) order(refs);
  };
  const ol = el('ol', {});
  ol.setAttribute('aria-label', 'Builds in the order they show');
  for (const b of builds) {
    const how = [NAMES[b.appear], b.leave && NAMES[b.leave].toLowerCase()].filter(Boolean).join(', then ');
    const li = el('li', { textContent: `${b.name}${b.items ? ' (one item at a time)' : ''} · ${how}` });
    li.dataset.ref = b.ref;
    if (b.ref === current) li.setAttribute('aria-current', 'true');
    if (!b.items) {
      li.tabIndex = 0;
      li.draggable = true;
      li.title = 'Drag, or ⌥↑ ⌥↓, to change when it shows';
      li.addEventListener('dragstart', (e) => e.dataTransfer.setData('text/plain', b.ref));
      li.addEventListener('dragover', (e) => e.preventDefault());
      li.addEventListener('drop', (e) => { e.preventDefault(); move(e.dataTransfer.getData('text/plain'), movable.indexOf(b.ref)); });
      li.addEventListener('keydown', (e) => {
        const d = e.altKey && { ArrowUp: -1, ArrowDown: 1 }[e.key];
        if (!d) return;
        e.preventDefault();
        e.stopPropagation(); // the row's, not the selected element's ⌥↓
        move(b.ref, movable.indexOf(b.ref) + d);
      });
    }
    ol.append(li);
  }
  wrap.append(ol);
  return wrap;
}

/**
 * The view for the slide (`kv`: its attributes; none on the title slide) or,
 * with one selected, the element (`element`: its appear and leave, whether
 * it's a list, or an arrow's own draw-in).
 */
export function render({ kv, element, builds = [], current }, api) {
  const out = el('div', {});
  if (element?.arrow) {
    out.append(check('fragment', 'Draws in on a click', element.appears, api.toggleArrow));
    return out;
  }
  if (element) {
    out.append(
      label('Appear ', select('appear', [['none', 'None'], ...APPEARS], element.appear || 'none', (v) => api.build(v, null))),
      label('Leave ', select('leave', [['none', 'None'], ...LEAVES], element.leave || 'none', (v) => api.build(null, v))),
    );
    // one effect per click: a leave follows fading in, or nothing (`qd build set`)
    if (!['none', 'fade-in', undefined].includes(element.appear)) {
      out.querySelector('[name=leave]').disabled = true;
      out.append(el('p', { className: 'hint', textContent: 'Only an element that fades in, or has no appear effect, can also leave.' }));
    }
    if (element.list) out.append(check('incremental', 'One item at a time', element.incremental, api.incremental));
  } else if (kv) {
    out.append(
      label('Transition ', select('transition', [['', 'default'], ...TRANSITIONS.map((t) => [t, t])], kv.transition || '', (v) => api.setAttr('transition', v))),
      label('Speed ', select('speed', [['', 'default'], ...SPEEDS.map((t) => [t, t])], kv['transition-speed'] || '', (v) => api.setAttr('transition-speed', v))),
      check('morph', 'Morph into the next slide', kv['auto-animate'] === 'true', (on) => api.setAttr('auto-animate', on ? 'true' : '')),
    );
  } else {
    out.append(el('p', { className: 'hint', textContent: 'The title slide comes in with the deck’s own transition.' }));
    return out;
  }
  out.append(buildList(builds, current, api.order));
  return out;
}
