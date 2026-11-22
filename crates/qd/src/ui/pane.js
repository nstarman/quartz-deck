// Source pane (spec 004 US1/US2): the whole .qmd beside the slides.
//
// A textarea over a highlighted <pre> (research R8). Typing is saved after a
// pause as one `replace` op: the changed lines plus one line of context each
// side, so edits elsewhere never block it (R9). Changes made elsewhere are
// merged in when they don't overlap the person's unsaved text (FR-004).

import { diffRegion as region } from '/ui/inline.js';
import * as keys from '/ui/keys.js';

const pane = document.querySelector('#srcpane');
const ta = pane.querySelector('textarea');
const hl = pane.querySelector('pre');
const band = pane.querySelector('.band');

let api = null; // { op, conflict, onCaret, typed, say }
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

let painted = null; // the coloured lines on screen, one element each
/** Recolour only the lines that changed: a keystroke touches one or two, not the whole file's layout. */
function paint() {
  const now = highlight(ta.value).split('\n').slice(0, -1);
  if (!painted) { hl.textContent = ''; painted = []; }
  const { start: a, endA, endB } = region(painted, now);
  for (let k = a; k < endA; k++) hl.children[a].remove();
  const fresh = now.slice(a, endB).map((l) => `<div>${l || ' '}</div>`).join('');
  hl.insertBefore(document.createRange().createContextualFragment(fresh), hl.children[a] ?? null);
  painted = now;
}

let raf = 0;
function repaint() {
  cancelAnimationFrame(raf);
  raf = requestAnimationFrame(() => { paint(); syncScroll(); });
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

/** Save pending typing now (also before running a cell). */
export async function save() {
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

let autosave = true;
/** Setting `source.save` (spec 007): as you pause (default), or only on ⌘S and leaving the pane. */
export function setAutosave(on) {
  autosave = on;
  pane.querySelector('.hint').textContent = on ? 'saves as you pause · ⌘S saves now' : 'saves on ⌘S and when you leave the pane';
}

function schedule() {
  clearTimeout(timer);
  if (autosave) timer = setTimeout(save, 700);
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

// ---------- a diagram's source follows the cursor (spec 021, research R5) ----------

/** A `{dot}` cell: its opening fence line, its body, its closing fence (the editor's reload rule uses it too). */
export const DOT_CELL = /^((`{3,})[ \t]*\{dot\b[^\n]*\n)([\s\S]*?)^\2[ \t]*$/gm;
let followTimer = 0;
let followSeq = 0; // an answer for an older cursor is dropped
let hovering = false; // a resting pointer wins over the caret until it leaves, or the keyboard is used

/** The `{dot}` cell around text position `pos`: its body, the position in it, and its fence's line; or null. */
function dotAt(pos) {
  for (const m of ta.value.matchAll(DOT_CELL)) {
    const start = m.index + m[1].length;
    if (pos >= start && pos <= start + m[3].length) {
      return { body: m[3], offset: pos - start, line: ta.value.slice(0, m.index).split('\n').length };
    }
  }
  return null;
}

/** Follow text position `pos` (the caret's, or the resting pointer's) a moment after it stops. */
function follow(pos, wait = 60) {
  clearTimeout(followTimer);
  followTimer = setTimeout(async () => {
    const n = ++followSeq;
    const c = dotAt(pos);
    const cell = c && (await api.dotCell(c.line));
    if (n !== followSeq) return;
    if (!cell) { api.unfollow(); return; }
    // `dot at` counts bytes of the text as UTF-8
    api.follow(cell, c.body, new TextEncoder().encode(c.body.slice(0, c.offset)).length);
  }, wait);
}

function unfollow() {
  clearTimeout(followTimer);
  followSeq++;
  api.unfollow();
}

/** The keyboard was used: the caret is what's followed again. */
function followCaret() {
  hovering = false;
  follow(ta.selectionStart);
}

/** The text position under the pointer: a line from the coloured lines' boxes, a column from the font's width. */
let charWidth = 0;
function posAt(x, y) {
  const lines = [...hl.children];
  const i = lines.findIndex((l) => l.getBoundingClientRect().bottom > y);
  if (i < 0) return null;
  if (!charWidth) {
    const ctx = document.createElement('canvas').getContext('2d');
    ctx.font = getComputedStyle(ta).font;
    charWidth = ctx.measureText('0').width;
  }
  const row = ta.value.split('\n');
  if (i >= row.length) return null; // the colouring is a frame behind the text
  const col = Math.max(0, Math.round((x - lines[i].getBoundingClientRect().left) / charWidth));
  return row.slice(0, i).reduce((n, l) => n + l.length + 1, 0) + Math.min(col, row[i].length);
}

/** Caret line and selection → the editor's shared focus (spec 005). */
function caretMoved(e) {
  const first = ta.value.slice(0, ta.selectionStart).split('\n').length;
  const text = ta.value.slice(ta.selectionStart, ta.selectionEnd);
  const line = first + text.split('\n').length - 1;
  api.onCaret({ line, first, last: line, text });
  if (e?.key !== 'Escape') followCaret(); // Esc took the outline off: its keyup doesn't put it back
}

export function initPane(a) {
  api = a;
  ta.addEventListener('input', () => { repaint(); schedule(); api.typed(); followCaret(); });
  ta.addEventListener('blur', () => { save(); if (!hovering) unfollow(); });
  ta.addEventListener('mousemove', (e) => {
    const pos = posAt(e.clientX, e.clientY);
    if (pos === null) return;
    hovering = true;
    follow(pos, 300);
  });
  ta.addEventListener('mouseleave', () => {
    hovering = false;
    if (document.activeElement === ta) followCaret(); else unfollow();
  });
  ta.addEventListener('scroll', syncScroll);
  ta.addEventListener('keyup', caretMoved);
  ta.addEventListener('click', caretMoved);
  ta.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') unfollow();
    keys.dispatch(e, ['source']); // only the source pane's own actions (⌘S, ⇧Enter)
    e.stopPropagation(); // the editor's slide shortcuts don't apply while typing source
  });
  keys.on('source-save', { enabled: () => !pane.hidden, run: save });
}
