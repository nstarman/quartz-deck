// The column's Deck view (spec 023): the deck as a whole. Each control writes
// under `format: revealjs:` through a CLI op (`qd meta set`, `qd meta size`,
// `qd navigation`), so the deck's text is the op's.

import { el } from '/ui/run.js';
import { TRANSITIONS } from '/ui/actions.js';
import { label, select, check } from '/ui/animate.js';

// Quarto's built-in reveal.js themes
const THEMES = ['default', 'beige', 'blood', 'dark', 'league', 'moon', 'night', 'serif', 'simple', 'sky', 'solarized'];
const SIZES = [['1050x700', 'Standard, 1050 × 700'], ['1050x590', 'Wide, 16:9'], ['1050x788', '4:3'], ['custom', 'Custom']];
const MODES = [['linear', 'Linear'], ['vertical', 'Vertical'], ['grid', 'Grid']];

/**
 * `meta`: the front matter; `width`, `height`, `navigation`: the deck's.
 * `api.set(key, value)` sets `format.revealjs.KEY` (`null` removes it).
 */
export function render({ meta, width, height, navigation }, api) {
  const opt = (k) => meta.format?.revealjs?.[k] ?? meta[k];
  const set = (k, v) => (v != null || opt(k) !== undefined) && api.set(k, v); // nothing to remove
  // a theme may list the deck's own .scss after the built-in one: those are kept
  const theme = [opt('theme') ?? []].flat().map(String);
  const own = theme.filter((t) => t.endsWith('.scss'));
  const base = theme.find((t) => !t.endsWith('.scss')) || 'default';
  const themes = THEMES.includes(base) ? THEMES : [...THEMES, base];
  const setTheme = (v) => set('theme', own.length ? JSON.stringify([v, ...own]) : v === 'default' ? null : v);

  const size = SIZES.some(([v]) => v === `${width}x${height}`) ? `${width}x${height}` : 'custom';
  const px = (name, v) => el('input', { type: 'number', name, min: 1, max: 10000, value: v });
  const w = px('width', width), h = px('height', height);
  const custom = () => api.size(Number(w.value), Number(h.value));
  for (const i of [w, h]) i.addEventListener('change', custom);

  const out = el('div', {});
  out.append(
    label('Theme ', select('theme', themes.map((t) => [t, t]), base, setTheme)),
    ...(own.length ? [el('p', { className: 'hint', textContent: `with ${own.join(', ')}` })] : []),
    label('Slide size ', select('size', SIZES, size, (v) => v !== 'custom' && api.size(...v.split('x').map(Number)))),
    label('Width ', w),
    label('Height ', h),
    check('slide-number', 'Slide numbers', !!opt('slide-number'), (on) => set('slide-number', on ? 'true' : null)),
    label('Transition ', select('transition', [['', 'default'], ...TRANSITIONS.map((t) => [t, t])], opt('transition') || '', (v) => set('transition', v || null))),
    label('Navigation ', select('navigation', MODES, navigation, api.navigation)),
  );
  return out;
}
