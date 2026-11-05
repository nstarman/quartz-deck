// The person's settings (spec 007 US2): kept by qd in one file per user, so
// they are the same in every deck, after a restart and in VS Code's panel.
// The Settings window changes them; every open editor applies them live.

import { token } from '/ui/run.js';

let values = {};
let defs = [];
const listeners = [];

/** A setting's value now. */
export const get = (key) => values[key];
/** `f(values)` now (once loaded) and on every change. */
export function onChange(f) { listeners.push(f); }
const apply = () => listeners.forEach((f) => f(values));

/** Loaded once at start; `notice(text)` tells the person about an unreadable setting. */
export const ready = (notice) => fetch('/api/settings').then((r) => r.json()).then((r) => {
  ({ values, defs } = r);
  r.notices.forEach((n) => notice(n));
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
  const input = d.type === 'choice'
    ? Object.assign(document.createElement('select'), { id })
    : Object.assign(document.createElement('input'), { id, type: 'checkbox' });
  if (d.type === 'choice') {
    for (const v of d.values) input.append(new Option(LABELS[v] || v, v));
    input.value = values[d.key];
  } else {
    input.checked = !!values[d.key];
  }
  input.addEventListener('change', () => set({ [d.key]: d.type === 'choice' ? input.value : input.checked }));
  const row = document.createElement('div');
  row.className = 'setting';
  const label = Object.assign(document.createElement('label'), { htmlFor: id, textContent: d.label });
  const help = Object.assign(document.createElement('small'), { textContent: d.description });
  row.append(label, input, help);
  return row;
}
const LABELS = { left: 'Left', right: 'Right', last: 'As last time', open: 'Open', closed: 'Closed', pause: 'As you pause', explicit: 'On ⌘S or leaving the pane', system: 'Follow the system', light: 'Light', dark: 'Dark', hc: 'High contrast' };

/** File ▸ Settings… (⌘,). */
export function open() {
  const title = Object.assign(document.createElement('h2'), { id: 'settings-title', textContent: 'Settings' });
  const reset = Object.assign(document.createElement('button'), { type: 'button', textContent: 'Reset to defaults…' });
  reset.addEventListener('click', async () => {
    if (confirm('Put every setting back to its default?') && (await set({ reset: true }))) open();
  });
  const close = Object.assign(document.createElement('button'), { type: 'button', textContent: 'Close' });
  close.addEventListener('click', () => dialog.close());
  const bar = document.createElement('div');
  bar.className = 'buttons';
  bar.append(reset, close);
  dialog.replaceChildren(title, ...defs.map(control), bar);
  if (!dialog.open) dialog.showModal(); // Escape closes it (the browser's own)
  dialog.querySelector('select, input')?.focus();
}
