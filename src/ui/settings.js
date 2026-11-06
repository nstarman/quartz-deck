// The person's settings (spec 007 US2): kept by qd in one file per user, so
// they are the same in every deck, after a restart and in VS Code's panel.
// The Settings window changes them; every open editor applies them live.

import { token, el } from '/ui/run.js';
import { confirmSheet } from '/ui/sheet.js';

let values = {};
let defs = [];
let changed = () => {};

/** A setting's value now. */
export const get = (key) => values[key];
/** `f(values)` once loaded and on every change (the editor's one listener). */
export function onChange(f) { changed = f; }
const apply = () => changed(values);

/** Loaded once at start; `notice(text)` tells the person about an unreadable setting. */
export const ready = (notice) => fetch('/api/settings').then((r) => r.json()).then((r) => {
  ({ values, defs } = r);
  r.notices.forEach(notice);
  apply();
});

/** Every editor hears of a change, wherever it was made (SSE `settings`). */
export function listen(es) {
  es.addEventListener('settings', (e) => { values = JSON.parse(e.data).values; apply(); });
}

/** Change settings (`{key: value}`, or `{reset: true}`); applied here at once. */
export async function set(changes) {
  const r = await fetch('/api/settings', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-QD-Token': token },
    body: JSON.stringify(changes),
  });
  if (!r.ok) return false;
  values = (await r.json()).values;
  apply();
  return true;
}

// ---------- the window ----------

const dialog = document.createElement('dialog');
dialog.id = 'settings';
dialog.setAttribute('aria-labelledby', 'settings-title');
document.body.append(dialog);

function control(d) {
  const id = `set-${d.key}`;
  const choice = d.type === 'choice';
  const input = choice ? el('select', { id }) : el('input', { id, type: 'checkbox', checked: !!values[d.key] });
  if (choice) {
    input.append(...d.values.map((v) => new Option(LABELS[v] || v, v)));
    input.value = values[d.key];
  }
  input.addEventListener('change', () => set({ [d.key]: choice ? input.value : input.checked }));
  const row = el('div', { className: 'setting' });
  row.append(el('label', { htmlFor: id, textContent: d.label }), input, el('small', { textContent: d.description }));
  return row;
}
const LABELS = { left: 'Left', right: 'Right', last: 'As last time', open: 'Open', closed: 'Closed', pause: 'As you pause', explicit: 'On ⌘S or leaving the pane', system: 'Follow the system', light: 'Light', dark: 'Dark', hc: 'High contrast' };

/** File ▸ Settings… (⌘,). */
export function open() {
  const title = el('h2', { id: 'settings-title', textContent: 'Settings' });
  const reset = el('button', { type: 'button', textContent: 'Reset to defaults…' });
  reset.addEventListener('click', async () => {
    if ((await confirmSheet('Reset settings?', 'Every setting goes back to its default.', 'Reset', reset)) && (await set({ reset: true }))) open();
  });
  const close = el('button', { type: 'button', textContent: 'Close' });
  close.addEventListener('click', () => dialog.close());
  const bar = el('div', { className: 'buttons' });
  bar.append(reset, close);
  dialog.replaceChildren(title, ...defs.map(control), bar);
  if (!dialog.open) dialog.showModal(); // Escape closes it (the browser's own)
  dialog.querySelector('select, input')?.focus();
}
