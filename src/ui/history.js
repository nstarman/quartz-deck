// History (spec 012 US1): the deck's versions from git, newest first, and one
// of them rendered beside the current deck, with the slides that changed
// since then marked in the slide list; Compare lists those slides, with
// each one's source diff (US3).

import { token, el } from '/ui/run.js';

const $ = (s, r = document) => r.querySelector(s);
const panel = $('#history');
const then = $('#then');
const thenFrame = $('iframe', then);
let open = null; // the version shown beside the current deck
let changed = [];
const changes = $('.changes', panel);
const diffView = $('pre.diff', panel);

const post = (path, body) => fetch(path, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-QD-Token': token }, body: JSON.stringify(body) });

function ago(t) {
  const s = Date.now() / 1000 - t;
  for (const [n, unit] of [[86400 * 365, 'year'], [86400 * 30, 'month'], [86400, 'day'], [3600, 'hour'], [60, 'minute']]) {
    if (s >= n) { const k = Math.floor(s / n); return `${k} ${unit}${k > 1 ? 's' : ''} ago`; }
  }
  return 'just now';
}

const say = (t) => { $('.status', panel).textContent = t; };

/** Mark the slide list's slides that differ from the open version (again after the list re-renders). */
export function mark() {
  for (const li of document.querySelectorAll('#slides li[data-id]')) {
    li.classList.toggle('changed', !!open && changed.includes(li.dataset.id));
  }
}

async function load() {
  const list = $('.versions', panel);
  say('Loading…');
  const res = await fetch('/api/history?limit=200');
  const body = await res.json().catch(() => ({}));
  list.replaceChildren();
  if (!res.ok) {
    if (body.error?.code !== 'E_NO_REPO') return say(body.error?.message || res.statusText);
    say('This deck isn’t in git yet, so it has no history.');
    const b = el('button', { type: 'button', textContent: 'Start a history here', className: 'start' });
    b.addEventListener('click', async () => {
      const r = await post('/api/history/init', {});
      if (r.ok) load(); else say((await r.json().catch(() => ({}))).error?.message || 'Could not start a history');
    });
    return list.append(b);
  }
  say(body.versions.length ? '' : 'No versions yet.');
  for (const v of body.versions) {
    const li = document.createElement('li');
    const b = document.createElement('button');
    b.type = 'button';
    b.dataset.rev = v.rev || '';
    b.setAttribute('aria-pressed', String(!!open && open.rev === v.rev));
    const msg = el('span', { className: 'msg', textContent: v.message });
    if (v.qd) msg.append(el('span', { className: 'qd', textContent: 'qd' }));
    const meta = el('span', { className: 'meta', textContent: v.rev ? `${v.author} · ${ago(v.time)} · ${v.short}` : 'the deck as it is now' });
    b.append(msg, meta);
    if (v.rev) b.addEventListener('click', () => (open?.rev === v.rev ? close() : show(v)));
    else b.disabled = true;
    li.append(b);
    list.append(li);
  }
}

function clearCompare() {
  changes.replaceChildren();
  diffView.hidden = true;
}

async function show(v) {
  clearCompare();
  say(`Rendering ${v.short}…`);
  const res = await post('/api/history/render', { rev: v.rev });
  const body = await res.json().catch(() => ({}));
  if (!res.ok) return say(body.error?.message || res.statusText);
  open = v;
  changed = body.changed;
  say(changed.length ? `${changed.length} slide${changed.length > 1 ? 's' : ''} changed since then (marked).` : 'No slide changed since then.');
  $('figcaption', then).textContent = `${v.message} — ${v.author}, ${ago(v.time)} (${v.short})`;
  thenFrame.src = body.href + location.hash;
  then.hidden = false;
  for (const b of panel.querySelectorAll('button[data-rev]')) b.setAttribute('aria-pressed', String(b.dataset.rev === v.rev));
  mark();
}

function close() {
  open = null;
  changed = [];
  clearCompare();
  then.hidden = true;
  thenFrame.removeAttribute('src');
  for (const b of panel.querySelectorAll('button[data-rev]')) b.setAttribute('aria-pressed', 'false');
  mark();
}

/** The current deck went to slide `anchor`: the version beside it follows. */
export function follow(anchor) {
  if (open && thenFrame.contentWindow) thenFrame.contentWindow.location.hash = '#/' + anchor;
}

export const versionOpen = () => !!open;

/** Compare the open version with the deck now: the slides that changed (US3). */
export async function compare() {
  if (!open) return;
  clearCompare();
  const res = await fetch(`/api/history/diff?from=${encodeURIComponent(open.rev)}`);
  const body = await res.json().catch(() => ({}));
  if (!res.ok) return say(body.error?.message || res.statusText);
  const n = body.slides.length;
  say(n ? `${n} slide${n > 1 ? 's' : ''} changed since ${open.short}:` : `No slide changed since ${open.short}.`);
  for (const c of body.slides) {
    const b = el('button', { type: 'button', textContent: `${c.kind} · ${c.title || c.id}` });
    b.addEventListener('click', () => showChange(c));
    const li = el('li', {});
    li.append(b);
    changes.append(li);
  }
}

/** One changed slide: its source diff, and both previews at that slide. */
function showChange(c) {
  diffView.replaceChildren(...(c.diff || '').split('\n').filter((l) => l && !/^(---|\+\+\+|@@)/.test(l)).map((l) =>
    el('span', { className: { '+': 'add', '-': 'del' }[l[0]] || '', textContent: l + '\n' })));
  diffView.hidden = !c.diff;
  // the version beside follows the current deck (follow); a removed slide is only there
  if (c.kind === 'removed') thenFrame.contentWindow.location.hash = '#/' + c.id;
  else $('#preview').contentWindow.location.hash = '#/' + c.id;
}

export const isOpen = () => !panel.hidden;

export function toggle(on = panel.hidden) {
  panel.hidden = !on;
  if (on) load(); else close();
}
