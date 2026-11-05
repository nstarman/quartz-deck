// Source pane (spec 004 US1/US2): the whole .qmd beside the slides.
//
// A textarea over a highlighted <pre> (research R8). Typing is saved after a
// pause as one `replace` op: the changed lines plus one line of context each
// side, so edits elsewhere never block it (R9). Changes made elsewhere are
// merged in when they don't overlap the person's unsaved text (FR-004).

import { diffRegion as region } from '/ui/inline.js';

const pane = document.querySelector('#srcpane');
const ta = pane.querySelector('textarea');
const hl = pane.querySelector('pre');
const band = pane.querySelector('.band');

let api = null; // { op, conflict, onCaretLine, say }
let base = ''; // the file text the pane last agreed with
let timer = null;
let saving = false;

const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

function inline(t) {
  return esc(t)
    .replace(/`[^`]+`/g, (m) => `<span class="c">${m}</span>`)
    .replace(/\*\*[^*]+\*\*/g, (m) => `<b>${m}</b>`)
    .replace(/(^|[^*])(\*[^*\s][^*]*\*)/g, '$1<i>$2</i>')
    .replace(/\[[^\]]*\]\([^)]*\)/g, (m) => `<span class="l">${m}</span>`);
}

/** HTML for the whole text: front matter, headings, fences, cells, inline marks. */
function highlight(text) {
  const out = [];
  let mode = null; // 'yaml' | 'code'
  let fence = '';
  text.split('\n').forEach((line, i) => {
    if (i === 0 && line === '---') { mode = 'yaml'; out.push(`<span class="y">${esc(line)}</span>`); return; }
    if (mode === 'yaml') {
      if (line === '---' || line === '...') mode = null;
      out.push(`<span class="y">${esc(line).replace(/^(\s*[\w-]+):/, '<span class="k">$1</span>:')}</span>`);
      return;
    }
    if (mode === 'code') {
      if (line.trimStart().startsWith(fence)) { mode = null; out.push(`<span class="f">${esc(line)}</span>`); return; }
      out.push(line.trimStart().startsWith('#|') ? `<span class="o">${esc(line)}</span>` : `<span class="c">${esc(line)}</span>`);
      return;
    }
    const f = line.match(/^\s*(`{3,}|~{3,})/);
    if (f) { mode = 'code'; fence = f[1]; out.push(`<span class="f">${esc(line)}</span>`); return; }
    if (/^#{1,6}\s/.test(line)) { out.push(`<span class="h">${esc(line)}</span>`); return; }
    if (/^\s*:{3,}/.test(line)) { out.push(`<span class="d">${esc(line)}</span>`); return; }
    out.push(inline(line));
  });
  return out.join('\n') + '\n';
}

let raf = 0;
function repaint() {
  cancelAnimationFrame(raf);
  raf = requestAnimationFrame(() => { hl.innerHTML = highlight(ta.value); syncScroll(); });
}
function syncScroll() {
  hl.style.transform = `translate(${-ta.scrollLeft}px, ${-ta.scrollTop}px)`;
  band.style.transform = `translateY(${-ta.scrollTop}px)`;
}

/** Three-way merge of two edits of `b`; null when they touch the same text. */
function merge(b, mine, theirs) {
  if (mine === b) return theirs;
  if (theirs === b || theirs === mine) return mine;
  const m = region(b, mine);
  const t = region(b, theirs);
  // edits on the same line(s) conflict, even if their characters don't overlap
  const [ma, mz] = lines(b, m.start, m.endA, 0);
  const [ta2, tz] = lines(b, t.start, t.endA, 0);
  if (ma < tz && ta2 < mz) return null;
  const [first, second, x, y] = m.start <= t.start ? [m, t, mine, theirs] : [t, m, theirs, mine];
  return b.slice(0, first.start) + x.slice(first.start, first.endB) + b.slice(first.endA, second.start)
    + y.slice(second.start, second.endB) + b.slice(second.endA);
}

/** Widen [s, e) of `text` to whole lines, plus `extra` lines of context each side. */
function lines(text, s, e, extra) {
  let a = text.lastIndexOf('\n', s - 1) + 1;
  for (let i = 0; i < extra && a > 0; i++) a = text.lastIndexOf('\n', a - 2) + 1;
  let z = e > 0 && text[e - 1] === '\n' ? e : text.indexOf('\n', e) + 1 || text.length;
  for (let i = 0; i < extra && z < text.length; i++) z = text.indexOf('\n', z) + 1 || text.length;
  return [a, z];
}

async function save() {
  clearTimeout(timer);
  if (saving || ta.value === base) return;
  saving = true;
  const mine = ta.value;
  const r = region(base, mine);
  for (let extra = 1; extra <= 12; extra *= 2) {
    const [a, z] = lines(base, r.start, r.endA, extra);
    const res = await api.op('replace', { old: base.slice(a, z), new: mine.slice(a, z + (mine.length - base.length)) });
    if (res) { base = mine; saving = false; if (ta.value !== base) schedule(); return; }
    const code = api.lastError()?.code;
    if (code === 'E_AMBIGUOUS') continue; // add context and retry
    if (code === 'E_CONFLICT') { await rebase(true); saving = false; return; }
    break;
  }
  saving = false;
}

function schedule() {
  clearTimeout(timer);
  timer = setTimeout(save, 700);
}

/** Pull the file; merge it with unsaved text (or report a conflict). */
async function rebase(afterRefusal = false) {
  const theirs = (await fetch('/api/source').then((r) => r.json())).text;
  if (theirs === base && !afterRefusal) return;
  const mine = ta.value;
  const merged = merge(base, mine, theirs);
  if (merged === null) {
    // both changed the same lines: show both versions of them (spec 003 conflict bar)
    const r = region(base, mine);
    const t = region(base, theirs);
    const [a, z] = lines(base, Math.min(r.start, t.start), Math.max(r.endA, t.endA), 0);
    const minePart = mine.slice(a, z + mine.length - base.length);
    const theirsPart = theirs.slice(a, z + theirs.length - base.length);
    api.conflict(theirsPart, minePart, async () => {
      if (await api.op('replace', { old: theirsPart, new: minePart })) await load();
    }, load);
    base = theirs; // the conflict bar now holds the person's text
    setText(theirs);
    return;
  }
  base = theirs;
  if (merged !== mine) setText(merged);
  if (merged !== theirs) schedule();
}

/** Replace the textarea text, keeping the caret where the person was. */
function setText(text) {
  const { selectionStart: s, selectionEnd: e, scrollTop } = ta;
  const r = region(ta.value, text);
  const shift = (p) => (p >= r.endA ? p + (r.endB - r.endA) : Math.min(p, r.endB));
  ta.value = text;
  ta.setSelectionRange(shift(s), shift(e));
  ta.scrollTop = scrollTop;
  repaint();
}

/** Save pending typing now (e.g. before running a cell). */
export const flush = () => save();

export async function load() {
  base = (await fetch('/api/source').then((r) => r.json())).text;
  setText(base);
}

/** Changes arrived from elsewhere (SSE 'changed'). */
export function changed() {
  if (!pane.hidden && !saving) rebase();
}

/** Scroll to and highlight lines [first, last] (1-based) without taking focus (US2). */
export function reveal([first, last]) {
  if (pane.hidden || !first) return;
  const css = getComputedStyle(ta);
  const lh = parseFloat(css.lineHeight) || 18;
  const pad = parseFloat(css.paddingTop) || 0;
  const top = pad + (first - 1) * lh;
  if (top < ta.scrollTop || top > ta.scrollTop + ta.clientHeight - lh * 2) ta.scrollTop = Math.max(0, top - ta.clientHeight / 3);
  Object.assign(band.style, { top: top + 'px', height: (last - first + 1) * lh + 'px' });
  band.dataset.lines = `${first}-${last}`;
  band.animate([{ opacity: 1 }, { opacity: 1, offset: 0.7 }, { opacity: 0 }], 1600);
  syncScroll();
}

let lastLine = 0;
function caretMoved() {
  const line = ta.value.slice(0, ta.selectionStart).split('\n').length;
  if (line !== lastLine) { lastLine = line; api.onCaretLine(line); }
}

export function initPane(a) {
  api = a;
  ta.addEventListener('input', () => { repaint(); schedule(); });
  ta.addEventListener('blur', save);
  ta.addEventListener('scroll', syncScroll);
  ta.addEventListener('keyup', caretMoved);
  ta.addEventListener('click', caretMoved);
  ta.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') { e.preventDefault(); save(); }
    e.stopPropagation(); // the editor's own shortcuts don't apply while typing source
  });
}
