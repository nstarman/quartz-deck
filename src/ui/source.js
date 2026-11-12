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

function fit() {
  const lines = ta.value.split('\n').length;
  ta.rows = Math.max(3, Math.min(lines + 1, 24));
}

/**
 * Open the editor over `rect` (page coordinates) with `text`.
 * `on.save(text)` is called on ⌘/Ctrl+Enter, Escape or blur; `on.cancel()` on Cancel
 * (or Escape after a refused save). With `on.run(text)` (a code cell, spec 013),
 * Run and ⇧Enter save without closing and run the cell.
 */
export function openSource(rect, text, on) {
  current = on;
  runB.hidden = !on.run;
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
}

export function closeSource() {
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

ta.addEventListener('input', () => { err.textContent = ''; fit(); });
ta.addEventListener('keydown', (e) => {
  if (!current) return;
  e.stopPropagation();
  if (e.key === 'Escape') { e.preventDefault(); if (err.textContent) current.cancel(); else current.save(ta.value); return; }
  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) { e.preventDefault(); current.save(ta.value); }
  if (e.key === 'Enter' && e.shiftKey && current.run) { e.preventDefault(); current.run(ta.value); }
});
ta.addEventListener('blur', () => {
  // clicking elsewhere saves, unless the last save was refused (fix it or Esc)
  setTimeout(() => { if (current && !err.textContent && document.activeElement !== ta) current.save(ta.value); }, 0);
});
for (const b of [cancel, runB]) b.addEventListener('mousedown', (e) => e.preventDefault()); // keep the text focused: no blur save
cancel.addEventListener('click', () => current?.cancel());
runB.addEventListener('click', () => current?.run?.(ta.value));
