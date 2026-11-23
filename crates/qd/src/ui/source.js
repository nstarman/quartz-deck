import { textPos } from '/ui/pane.js';

// In-place source editor (spec 003 US2): a markdown text box drawn over the
// element (or slide) it edits, for anything that can't be typed into on the
// rendered slide — math, code cells, callouts, raw HTML — or on request.

const box = document.createElement('div');
box.id = 'srcedit';
box.hidden = true;
box.innerHTML = '<textarea spellcheck="false" aria-label="Markdown source"></textarea>'
  + '<div class="bar"><span class="err" role="alert"></span><span class="hint">⌘Enter or Esc saves</span>'
  + '<button type="button" class="run" hidden title="Save, run this cell and keep editing">Run ⇧Enter</button><button type="button" class="cancel">Cancel</button></div>';
document.body.append(box);
const ta = box.querySelector('textarea');
const err = box.querySelector('.err');
const cancel = box.querySelector('.cancel');
const runB = box.querySelector('.run');

let current = null;
let cursorTimer = 0;

/**
 * The caret's line on screen (a text box has no element per line): from the
 * line height and the scroll, the box not wrapping lines while it reports
 * (openSource). `beside`: the drawing goes right of the box, off the text typed.
 */
function lineBox(pos) {
  const cs = getComputedStyle(ta);
  const lh = parseFloat(cs.lineHeight);
  const r = ta.getBoundingClientRect();
  const top = r.top + parseFloat(cs.paddingTop) + parseFloat(cs.borderTopWidth)
    + (ta.value.slice(0, pos).split('\n').length - 1) * lh - ta.scrollTop;
  return { left: r.left, top, bottom: top + lh, beside: box.getBoundingClientRect().right };
}

let hovering = false; // a resting pointer wins over the caret until it leaves, or the keyboard is used

/** Tell `on.cursor` about position `pos` (the caret's, or the resting pointer's) a moment after it stops (spec 021 US3). */
function report(pos, wait = 60) {
  clearTimeout(cursorTimer);
  if (current?.cursor) cursorTimer = setTimeout(() => current.cursor(ta.value, pos(), lineBox(pos())), wait);
}
const reportCursor = () => { hovering = false; report(() => ta.selectionStart); };

/** The text position under the pointer, the box not wrapping: a row from the line height, a column from the font's width. */
function posAt(x, y) {
  const cs = getComputedStyle(ta);
  const i = Math.floor((y - lineBox(0).top) / parseFloat(cs.lineHeight));
  const dx = x - ta.getBoundingClientRect().left - parseFloat(cs.paddingLeft) - parseFloat(cs.borderLeftWidth) + ta.scrollLeft;
  return textPos(ta, i, dx);
}

function fit() {
  const lines = ta.value.split('\n').length;
  ta.rows = Math.max(3, Math.min(lines + 1, 24));
}

/**
 * Open the editor over `rect` (page coordinates) with `text`.
 * `on.save(text)` is called on ⌘/Ctrl+Enter, Escape or blur; `on.cancel()` on Cancel
 * (or Escape after a refused save). With `on.run(text)` (a code cell, spec 013),
 * Run and ⇧Enter save without closing and run the cell. With `on.cursor(text,
 * pos, line)` (a diagram, spec 021), the caret is reported as it moves.
 */
export function openSource(rect, text, on) {
  current = on;
  runB.hidden = !on.run;
  ta.wrap = on.cursor ? 'off' : 'soft'; // a diagram's lines, one per row: the caret's row is its line
  ta.value = text;
  err.textContent = '';
  fit();
  const width = Math.min(Math.max(rect.width, 420), innerWidth - 16);
  box.style.width = width + 'px';
  box.style.left = Math.max(8, Math.min(rect.left, innerWidth - width - 8)) + 'px';
  box.hidden = false;
  box.style.top = Math.max(8, Math.min(rect.top, innerHeight - box.offsetHeight - 8)) + 'px';
  ta.focus();
  ta.setSelectionRange(text.length, text.length);
  reportCursor();
}

export function closeSource() {
  clearTimeout(cursorTimer);
  hovering = false;
  current = null;
  box.hidden = true;
  err.textContent = '';
}

export function sourceError(message) {
  err.textContent = message;
  ta.focus();
}

export function sourceText() {
  return ta.value;
}

ta.addEventListener('input', () => { err.textContent = ''; fit(); reportCursor(); });
ta.addEventListener('keyup', reportCursor);
ta.addEventListener('scroll', () => { if (!hovering) reportCursor(); }); // the drawing follows its line
ta.addEventListener('mousemove', (e) => {
  const pos = current?.cursor && posAt(e.clientX, e.clientY);
  if (pos == null) return; // not a diagram, or below the text
  hovering = true;
  report(() => pos, 300);
});
ta.addEventListener('mouseleave', () => { if (hovering) reportCursor(); });
ta.addEventListener('click', reportCursor);
ta.addEventListener('keydown', (e) => {
  if (!current) return;
  e.stopPropagation();
  if (e.key === 'Escape') { e.preventDefault(); if (err.textContent) current.cancel(); else current.save(ta.value); return; }
  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); current.save(ta.value); }
  // code runs only on the person's own key or click (spec 004), never a synthetic event
  if (e.key === 'Enter' && e.shiftKey && current.run && e.isTrusted) { e.preventDefault(); current.run(ta.value); }
});
ta.addEventListener('blur', () => {
  // clicking elsewhere saves, unless the last save was refused (fix it or Esc)
  setTimeout(() => { if (current && !err.textContent && document.activeElement !== ta) current.save(ta.value); }, 0);
});
for (const b of [cancel, runB]) b.addEventListener('mousedown', (e) => e.preventDefault()); // keep the text focused: no blur save
cancel.addEventListener('click', () => current?.cancel());
runB.addEventListener('click', (e) => e.isTrusted && current?.run?.(ta.value));
