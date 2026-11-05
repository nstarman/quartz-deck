// Shared focus (spec 005): what the person has in this editor tab — slide,
// selected elements, selected text and where, source caret line, editing —
// in one object. The slide view and the source pane set it; listeners (the
// source↔slide sync) follow it; each change is posted to the server so
// `qd focus` can read it. Nothing is stored (FR-010).

import { token } from '/ui/run.js';

export const tabId = crypto.randomUUID();
const MAX_TEXT = 2000;
const focus = { slide: null, elements: [], selection: null, line: null, editing: false };
const busy = { slide: false, source: false };
const listeners = [];
let version = '';
let timer = null;
let quiet = null;

export const get = () => ({ ...focus });

/** Follow focus changes: `f(focus, changed)`. */
export const on = (f) => listeners.push(f);

/** Merge `partial` into the focus; listeners and the server hear of real changes. */
export function set(partial) {
  const changed = Object.keys(partial).filter((k) => JSON.stringify(focus[k]) !== JSON.stringify(partial[k]));
  if (!changed.length) return;
  Object.assign(focus, partial);
  listeners.forEach((f) => f(focus, partial));
  report();
}

/** The deck version the refs are computed against (re-reported on change). */
export function setVersion(v) {
  if (v !== version) { version = v; report(); }
}

/** An on-slide edit opened or closed. */
export function editingSlide(on) { busy.slide = on; set({ editing: busy.slide || busy.source }); }

/** A keystroke in the source pane: editing until 3 s of quiet (FR-008). */
export function typed() {
  busy.source = true;
  set({ editing: true });
  clearTimeout(quiet);
  quiet = setTimeout(() => { busy.source = false; set({ editing: busy.slide }); }, 3000);
}

function report() {
  clearTimeout(timer);
  timer = setTimeout(send, 100);
}

function send() {
  if (!token || !version) return; // a tab opened without the editor link can't report
  let { selection } = focus;
  if (selection && selection.text.length > MAX_TEXT) selection = { ...selection, text: selection.text.slice(0, MAX_TEXT), truncated: true };
  fetch('/api/focus', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-QD-Token': token },
    body: JSON.stringify({ tab: tabId, version, ...focus, selection }),
  }).catch(() => {});
}

// the most recently used tab is the one that reported last (FR-006)
addEventListener('focus', report);
document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') report(); });
