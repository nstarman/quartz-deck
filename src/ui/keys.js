// The editor's one action list (spec 007, research R1): menus, right-click
// menus, tooltips, the shortcut reference and every key the editor handles
// come from `actions.json` (served as /ui/action-list.js). Handlers are
// registered by id; `dispatch` is the only place keys turn into actions.

import LIST from '/ui/action-list.js';

export const mac = /Mac|iP(hone|ad|od)/.test(navigator.platform);
const framed = new URLSearchParams(location.search).get('host') === 'vscode';

/** Every entry, separators included, in menu order. */
export const ENTRIES = LIST;
export const ACTIONS = LIST.filter((a) => a.id);
const byId = new Map(ACTIONS.map((a) => [a.id, a]));
export const action = (id) => byId.get(id);

/** The keys action `a` has here (VS Code's panel may replace or drop them). */
export function keysOf(a) {
  const k = framed && a && 'vscode_key' in a ? a.vscode_key : a?.key;
  return k == null ? [] : [].concat(k);
}

const parse = (k) => { const p = k.split('+'); return { key: p.pop(), mods: new Set(p) }; };

/**
 * Does key `k` ("Mod+Shift+N") match keydown `e`? Mod is shown as ⌘ on macOS and
 * Ctrl elsewhere, and either one works (as the editor always took both).
 */
export function matches(k, e) {
  const { key, mods } = parse(k);
  const modOk = mods.has('Mod') ? e.metaKey || e.ctrlKey : !e.metaKey && e.ctrlKey === mods.has('Ctrl');
  if (!modOk || e.altKey !== mods.has('Alt')) return false;
  const punct = key.length === 1 && !/[a-z0-9]/i.test(key); // ? and , : Shift is part of the character
  if (!punct && e.shiftKey !== mods.has('Shift')) return false;
  if (/^[0-9]$/.test(key)) return e.code === `Digit${key}`; // ⇧8 types *, ⌥8 •
  if (/^[a-z]$/i.test(key)) return e.code === `Key${key.toUpperCase()}` || e.key.toLowerCase() === key.toLowerCase();
  return e.key === key;
}

const GLYPH = { ArrowUp: '↑', ArrowDown: '↓', ArrowLeft: '←', ArrowRight: '→', Backspace: '⌫', Delete: '⌦', Escape: 'Esc', ContextMenu: 'Menu' };

/** Key `k` as people read it here: ⇧⌘N on macOS, Ctrl+Shift+N elsewhere. */
export function show(k) {
  const { key, mods } = parse(k);
  const name = GLYPH[key] || (key.length === 1 ? key.toUpperCase() : key);
  if (mac) return ['Ctrl', '⌃', 'Alt', '⌥', 'Shift', '⇧', 'Mod', '⌘'].reduce((s, m, i, a) => (i % 2 === 0 && mods.has(m) ? s + a[i + 1] : s), '') + name;
  return [...['Ctrl', 'Mod', 'Alt', 'Shift'].filter((m) => mods.has(m)).map((m) => (m === 'Mod' ? 'Ctrl' : m)), name].join('+');
}

/** `aria-keyshortcuts` form of key `k`. */
export function aria(k) {
  const { key, mods } = parse(k);
  return [...['Mod', 'Ctrl', 'Alt', 'Shift'].filter((m) => mods.has(m)).map((m) => (m === 'Mod' ? (mac ? 'Meta' : 'Control') : m === 'Ctrl' ? 'Control' : m)), key].join('+');
}

/** The shortcut of action `id` as shown here, or undefined. */
export const shortcut = (id) => { const k = keysOf(action(id))[0]; return k && show(k); };

// ---------- handlers ----------

/**
 * What each action does. A handler is `{run(ctx), enabled?(ctx), checked?(ctx), key?(e)}`;
 * `ctx` is the editor's context ({el, slide, e}), `key(e)` an extra check for the
 * keyboard (for example "only while the slide has focus"). A handler without
 * `enabled` needs no target: on a key it runs at once, with `{e}` only.
 */
const handlers = new Map();
let context = async () => ({});

export function on(id, h) {
  if (!byId.has(id)) throw new Error(`unknown action ${id}`);
  handlers.set(id, typeof h === 'function' ? { run: h } : h);
}
export const handler = (id) => handlers.get(id);
/** The editor supplies what actions act on (the selected element, the current slide). */
export function setContext(f) { context = f; }
export const currentContext = (e) => context(e);

/** Run action `id` (from a menu, a button or a key). */
export async function run(id, ctx) {
  const h = handlers.get(id);
  ctx ??= await context();
  if (h && h.enabled?.(ctx) !== false) return h.run(ctx);
  return null;
}

/**
 * Handle keydown `e` for the places it happened in, most specific first
 * (`element`, `slide`, `slides`, `text`, `source`). Returns true when an action took it.
 */
export function dispatch(e, places) {
  for (const place of [...places, 'anywhere']) {
    for (const a of ACTIONS) {
      if (!a.where.includes(place) || !keysOf(a).some((k) => matches(k, e))) continue;
      const h = handlers.get(a.id);
      if (!h || h.key?.(e) === false) continue;
      if (a.runs_code && !e.isTrusted) continue; // code runs only on the person's own key (spec 004)
      e.preventDefault();
      e.stopPropagation();
      if (!h.enabled) h.run({ e }); // at once: the next key must find focus where this one put it
      else context(e).then((ctx) => { if (h.enabled(ctx) !== false) h.run({ ...ctx, e }); });
      return true;
    }
  }
  return false;
}

/** For tests and the reference: actions without a handler, and which action each key resolves to. */
window.qdActions = () => ({
  missing: ACTIONS.filter((a) => !handlers.has(a.id)).map((a) => a.id),
  resolve: (k, places) => {
    const { key, mods } = parse(k);
    const fake = { key, code: /^[0-9]$/.test(key) ? `Digit${key}` : /^[a-z]$/i.test(key) ? `Key${key.toUpperCase()}` : key, metaKey: mac && mods.has('Mod'), ctrlKey: mods.has('Ctrl') || (!mac && mods.has('Mod')), altKey: mods.has('Alt'), shiftKey: mods.has('Shift') };
    for (const place of [...places, 'anywhere']) {
      const a = ACTIONS.find((x) => x.where.includes(place) && keysOf(x).some((kk) => matches(kk, fake)));
      if (a) return a.id;
    }
    return null;
  },
});
