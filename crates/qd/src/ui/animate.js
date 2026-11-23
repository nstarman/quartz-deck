// The column's Animate view (spec 023): how the slide comes in, and whether the
// selected element appears on a click. Each control writes through the op it
// did from the slide and element bars.

import { el } from '/ui/run.js';
import { TRANSITIONS } from '/ui/actions.js';

const label = (...kids) => { const l = el('label', {}); l.append(...kids); return l; };

/**
 * The view for an element (`fragment`: whether it appears on a click) or, with
 * none selected, the slide (`kv`: its attributes; none on the title slide).
 */
export function render({ kv, fragment }, { transition, toggleFragment }) {
  if (fragment !== undefined) {
    const box = el('input', { type: 'checkbox', name: 'fragment', checked: fragment });
    box.addEventListener('change', toggleFragment);
    return label(box, ' Appears on click');
  }
  if (!kv) return el('p', { className: 'hint', textContent: 'The title slide comes in with the deck’s own transition.' });
  const s = el('select', { name: 'transition' });
  s.append(el('option', { value: '', textContent: 'default' }), ...TRANSITIONS.map((t) => el('option', { textContent: t })));
  s.value = kv.transition || '';
  s.addEventListener('change', () => transition(s.value));
  return label('Transition ', s);
}
