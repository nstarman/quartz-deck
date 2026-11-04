// Formatting toolbar for on-slide typing (spec 003 FR-003a, research R2).
// It lives in the editor page and acts on the selection inside the preview
// iframe; buttons never take focus, so the edit stays open while formatting.

const bar = document.createElement('div');
bar.id = 'fmtbar';
bar.setAttribute('role', 'toolbar');
bar.setAttribute('aria-label', 'Formatting');
bar.hidden = true;
document.body.append(bar);

const SWATCHES = ['#c0392b', '#d35400', '#b7950b', '#1e8449', '#2a5bd7', '#7d3c98', '#555555'];
const SIZES = [0.6, 0.8, 1.2, 1.5, 2];

let session = null;
const doc = () => session?.root.ownerDocument;
const sel = () => doc()?.getSelection();

function inRoot(node) {
  return node && session.root.contains(node.nodeType === 1 ? node : node.parentNode);
}

/** Nearest ancestor of the selection (inside the root) matching `test`. */
function enclosing(test) {
  const s = sel();
  if (!s?.rangeCount) return null;
  for (let n = s.getRangeAt(0).commonAncestorContainer; n && n !== session.root; n = n.parentNode) {
    if (n.nodeType === 1 && test(n)) return n;
  }
  return null;
}

function unwrap(el) {
  const parent = el.parentNode;
  while (el.firstChild) parent.insertBefore(el.firstChild, el);
  el.remove();
}

/** Wrap the selection in a new element, or unwrap an enclosing one (toggle). */
function wrap(test, make) {
  const existing = enclosing(test);
  if (existing) { unwrap(existing); return; }
  const s = sel();
  if (!s?.rangeCount || s.isCollapsed) return;
  const r = s.getRangeAt(0);
  if (!inRoot(r.startContainer) || !inRoot(r.endContainer)) return;
  const el = make(doc());
  el.append(r.extractContents());
  r.insertNode(el);
  s.removeAllRanges();
  const nr = doc().createRange();
  nr.selectNodeContents(el);
  s.addRange(nr);
}

function styled(prop) {
  return (n) => n.tagName === 'SPAN' && n.style[prop] && n.attributes.length === 1;
}

function setStyle(prop, value) {
  const existing = enclosing(styled(prop));
  if (existing) {
    if (value) existing.style[prop] = value; else unwrap(existing);
    return;
  }
  if (!value) return;
  wrap(() => false, (d) => {
    const span = d.createElement('span');
    span.setAttribute('style', `${prop === 'fontSize' ? 'font-size' : 'color'}:${value}`);
    return span;
  });
}

function stepSize(dir) {
  const existing = enclosing(styled('fontSize'));
  const cur = existing ? parseFloat(existing.style.fontSize) : 1;
  const steps = [...SIZES, 1].sort((a, b) => a - b);
  const i = steps.indexOf(cur) === -1 ? steps.indexOf(1) : steps.indexOf(cur);
  const next = steps[Math.max(0, Math.min(steps.length - 1, i + dir))];
  setStyle('fontSize', next === 1 ? null : `${next}em`);
}

function exec(cmd, value) {
  doc().execCommand(cmd, false, value);
}

const TAGS = { bold: ['B', 'STRONG'], italic: ['I', 'EM'], underline: ['U'], strikeThrough: ['S', 'STRIKE', 'DEL'] };

/** Bold/italic/underline/strike: remove an enclosing tag, else apply (Chrome's own toggle is unreliable). */
function toggle(cmd) {
  const on = enclosing((n) => TAGS[cmd].includes(n.tagName) && !n.attributes.length);
  if (on) unwrap(on); else exec(cmd);
}

function link() {
  const s = sel();
  const range = s?.rangeCount ? s.getRangeAt(0).cloneRange() : null;
  const existing = enclosing((n) => n.tagName === 'A');
  session.hold = true; // the prompt takes focus; don't save on that blur
  const url = prompt('Link address (empty removes the link)', existing?.getAttribute('href') || 'https://');
  session.hold = false;
  session.root.focus();
  if (range) { s.removeAllRanges(); s.addRange(range); }
  if (url === null) return;
  if (!url.trim() || url.trim() === 'https://') { if (existing) unwrap(existing); return; }
  if (existing) existing.setAttribute('href', url.trim()); else exec('createLink', url.trim());
}

const BLOCK_ONLY = new Set(['ul', 'ol', 'indent', 'outdent', 'left', 'center', 'right']);

// [id, label, title (with shortcut), action]
const BUTTONS = [
  ['bold', 'B', 'Bold (⌘B)', () => toggle('bold')],
  ['italic', 'I', 'Italic (⌘I)', () => toggle('italic')],
  ['underline', 'U', 'Underline (⌘U)', () => toggle('underline')],
  ['strike', 'S', 'Strikethrough (⇧⌘X)', () => toggle('strikeThrough')],
  ['code', '</>', 'Inline code (⌘E)', () => wrap((n) => n.tagName === 'CODE', (d) => d.createElement('code'))],
  ['link', '🔗', 'Link (⌘K)', link],
  '|',
  ['ul', '•', 'Bulleted list (⇧⌘8)', () => exec('insertUnorderedList')],
  ['ol', '1.', 'Numbered list (⇧⌘7)', () => exec('insertOrderedList')],
  ['outdent', '⇤', 'Outdent (⇧Tab)', () => exec('outdent')],
  ['indent', '⇥', 'Indent (Tab in a list)', () => exec('indent')],
  '|',
  ['mark', '▇', 'Highlight', () => wrap((n) => n.tagName === 'MARK', (d) => d.createElement('mark'))],
  'swatches',
  '|',
  ['smaller', 'A−', 'Smaller text', () => stepSize(-1)],
  ['larger', 'A+', 'Larger text', () => stepSize(1)],
  '|',
  ['left', '⯇', 'Align left', () => setAlign(null)],
  ['center', '≡', 'Centre', () => setAlign('center')],
  ['right', '⯈', 'Align right', () => setAlign('right')],
];

function setAlign(value) {
  session.align = value || 'left';
  session.root.style.textAlign = value || '';
}

function button(id, label, title, run) {
  const b = document.createElement('button');
  b.type = 'button';
  b.dataset.fmt = id;
  b.textContent = label;
  b.title = title;
  b.setAttribute('aria-label', title);
  b.addEventListener('mousedown', (e) => e.preventDefault()); // keep focus in the slide
  b.addEventListener('click', () => { if (session) { run(); session.root.focus(); } });
  return b;
}

for (const item of BUTTONS) {
  if (item === '|') { bar.append(Object.assign(document.createElement('span'), { className: 'sep' })); continue; }
  if (item === 'swatches') {
    for (const c of SWATCHES) {
      const b = button('color', '', `Text colour ${c}`, () => setStyle('color', c));
      b.className = 'swatch';
      b.style.background = c;
      bar.append(b);
    }
    bar.append(button('nocolor', '⌀', 'Remove text colour', () => setStyle('color', null)));
    continue;
  }
  bar.append(button(...item));
}

/** Show the toolbar for an edit session; `frame` is the preview iframe. */
export function showToolbar(s, frame) {
  session = s;
  for (const b of bar.querySelectorAll('button')) b.hidden = s.reg.mode === 'inline' && BLOCK_ONLY.has(b.dataset.fmt);
  bar.hidden = false;
  place(frame);
}

export function place(frame) {
  if (!session || bar.hidden) return;
  const f = frame.getBoundingClientRect();
  const r = session.root.getBoundingClientRect();
  const top = f.top + r.top - bar.offsetHeight - 8;
  bar.style.top = Math.max(4, top < f.top ? f.top + r.bottom + 8 : top) + 'px';
  bar.style.left = Math.max(4, Math.min(f.left + r.left, innerWidth - bar.offsetWidth - 4)) + 'px';
}

export function hideToolbar() {
  session = null;
  bar.hidden = true;
}

/** Keyboard shortcuts while typing; returns true when handled. */
export function formatKey(e) {
  if (!session || !(e.metaKey || e.ctrlKey)) return false;
  const k = e.key.toLowerCase();
  const block = session.reg.mode !== 'inline';
  const run = {
    b: !e.shiftKey && (() => toggle('bold')),
    i: !e.shiftKey && (() => toggle('italic')),
    u: !e.shiftKey && (() => toggle('underline')),
    x: e.shiftKey && (() => toggle('strikeThrough')),
    e: !e.shiftKey && BUTTONS.find((b) => b[0] === 'code')[3],
    k: !e.shiftKey && link,
    8: e.shiftKey && block && (() => exec('insertUnorderedList')),
    '*': e.shiftKey && block && (() => exec('insertUnorderedList')),
    7: e.shiftKey && block && (() => exec('insertOrderedList')),
    '&': e.shiftKey && block && (() => exec('insertOrderedList')),
  }[k];
  if (!run) return false;
  e.preventDefault();
  run();
  return true;
}
