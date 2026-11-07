// Formatting toolbar for on-slide typing (spec 003 FR-003a, research R2).
// It lives in the editor page and acts on the selection inside the preview
// iframe; buttons never take focus, so the edit stays open while formatting.

import * as keys from '/ui/keys.js';
import { ask } from '/ui/sheet.js';

const bar = document.createElement('div');
bar.id = 'fmtbar';
bar.setAttribute('role', 'toolbar');
bar.setAttribute('aria-label', 'Formatting');
bar.hidden = true;
document.body.append(bar);

const SIZES = [0.6, 0.8, 1, 1.2, 1.5, 2]; // em; 1 = no size span

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
  let i = SIZES.indexOf(cur);
  if (i < 0) i = SIZES.indexOf(1);
  const next = SIZES[Math.max(0, Math.min(SIZES.length - 1, i + dir))];
  setStyle('fontSize', next === 1 ? null : `${next}em`);
}

const code = () => wrap((n) => n.tagName === 'CODE', (d) => d.createElement('code'));

function exec(cmd, value) {
  doc().execCommand(cmd, false, value);
}

const TAGS = { bold: ['B', 'STRONG'], italic: ['I', 'EM'], underline: ['U'], strikeThrough: ['S', 'STRIKE', 'DEL'] };

/** Bold/italic/underline/strike: remove an enclosing tag, else apply (Chrome's own toggle is unreliable). */
function toggle(cmd) {
  const on = enclosing((n) => TAGS[cmd].includes(n.tagName) && !n.attributes.length);
  if (on) unwrap(on); else exec(cmd);
}

/** A web or mail address, a slide (#id) or a path; '' removes the link. */
function badLink(x) {
  if (!x || /^(#|\.{0,2}\/)/.test(x)) return '';
  try { if (['http:', 'https:', 'mailto:'].includes(new URL(x).protocol)) return ''; } catch {}
  return 'An address such as https://example.org, a slide such as #results, or a path';
}

async function link() {
  const s = sel();
  const range = s?.rangeCount ? s.getRangeAt(0).cloneRange() : null;
  const existing = enclosing((n) => n.tagName === 'A');
  session.hold = true; // the sheet takes focus; don't save on that blur
  const v = await ask({
    title: 'Link',
    anchor: bar.querySelector('[data-fmt=link]'),
    fields: [{ name: 'href', kind: 'text', label: 'Address (empty removes the link)', value: existing?.getAttribute('href') || '', placeholder: 'https://', validate: badLink }],
  });
  session.hold = false;
  session.root.focus();
  if (range) { s.removeAllRanges(); s.addRange(range); }
  if (!v) return;
  if (!v.href) { if (existing) unwrap(existing); return; }
  if (existing) existing.setAttribute('href', v.href); else exec('createLink', v.href);
}

const BLOCK_ONLY = new Set(['ul', 'ol', 'indent', 'outdent', 'left', 'center', 'right']);

// [id, label, title, action]; the key in a title comes from the action list (spec 007)
const BUTTONS = [
  ['bold', 'B', 'Bold', () => toggle('bold')],
  ['italic', 'I', 'Italic', () => toggle('italic')],
  ['underline', 'U', 'Underline', () => toggle('underline')],
  ['strike', 'S', 'Strikethrough', () => toggle('strikeThrough')],
  ['code', '</>', 'Inline code', code],
  ['link', '🔗', 'Link', link],
  ['ul', '•', 'Bulleted list', () => exec('insertUnorderedList')],
  ['ol', '1.', 'Numbered list', () => exec('insertOrderedList')],
  ['outdent', '⇤', 'Outdent (⇧Tab)', () => exec('outdent')],
  ['indent', '⇥', 'Indent (Tab in a list)', () => exec('indent')],
  ['mark', '▇', 'Highlight', () => wrap((n) => n.tagName === 'MARK', (d) => d.createElement('mark'))],
  ['smaller', 'A−', 'Smaller text', () => stepSize(-1)],
  ['larger', 'A+', 'Larger text', () => stepSize(1)],
  ['left', '⯇', 'Align left', () => setAlign(null)],
  ['center', '≡', 'Centre', () => setAlign('center')],
  ['right', '⯈', 'Align right', () => setAlign('right')],
];
const COLOURS = [['#c0392b', 'Red'], ['#d35400', 'Orange'], ['#b7950b', 'Gold'], ['#1e8449', 'Green'], ['#2a5bd7', 'Blue'], ['#7d3c98', 'Purple'], ['#555555', 'Grey']];

function setAlign(value) {
  session.align = value || 'left';
  session.root.style.textAlign = value || '';
}

const noColor = () => setStyle('color', null);

/** Toolbar button id → its action in the list (the rest share the id). */
const ACTION = { mark: 'highlight', nocolor: 'no-color', left: 'align-left', center: 'align-center', right: 'align-right' };

function button(id, label, title, run) {
  const b = document.createElement('button');
  b.type = 'button';
  b.dataset.fmt = id;
  b.textContent = label;
  const key = keys.shortcut(ACTION[id] || id);
  b.dataset.hint = title; // the setting hints.shortcuts adds or drops the key (editor.js)
  if (key) b.dataset.key = key;
  title = key ? `${title} (${key})` : title;
  b.title = title;
  b.setAttribute('aria-label', title);
  b.addEventListener('mousedown', (e) => e.preventDefault()); // keep focus in the slide
  b.addEventListener('click', () => { if (session) after(run()); });
  return b;
}

// the common few up front, the rest in two pop-overs (spec 008 US7, research R12)
const sep = () => Object.assign(document.createElement('span'), { className: 'sep' });
const make = (id) => button(...BUTTONS.find((b) => b[0] === id));
const closePops = () => { for (const p of bar.querySelectorAll('.pop')) p.hidden = true; };
function pop(id, label, title, children) {
  const p = Object.assign(document.createElement('div'), { className: 'pop', hidden: true });
  p.append(...children);
  p.addEventListener('click', (e) => { if (e.target.closest('button')) closePops(); });
  const b = button(id, label, title, () => { const open = p.hidden; closePops(); p.hidden = !open; });
  b.setAttribute('aria-haspopup', 'true');
  return [b, p];
}
const swatches = COLOURS.map(([c, name]) => {
  const b = button('color', '', `Text colour: ${name}`, () => setStyle('color', c));
  b.className = 'swatch';
  b.style.background = c;
  return b;
});
const segment = Object.assign(document.createElement('span'), { className: 'seg' });
segment.append(...['left', 'center', 'right'].map(make));
bar.append(
  ...['bold', 'italic', 'underline', 'strike', 'link', 'ul'].map(make), sep(),
  ...pop('color', 'A', 'Text colour', [...swatches, button('nocolor', '⌀', 'Remove text colour', noColor)]),
  ...pop('more', 'Aa', 'More formatting', [...['code', 'ol', 'outdent', 'indent', 'mark'].map(make), sep(), make('smaller'), make('larger'), sep(), segment]),
);

/** Show the toolbar for an edit session; `frame` is the preview iframe. */
export function showToolbar(s, frame) {
  session = s;
  for (const b of bar.querySelectorAll('button')) b.hidden = s.reg.mode === 'inline' && BLOCK_ONLY.has(b.dataset.fmt);
  bar.hidden = false;
  place(frame);
}

export function place(frame) {
  if (session && !bar.hidden) placeNear(bar, session.root, frame);
}

/**
 * Put a bar for element `el` (in the preview `frame`) where it covers nothing else
 * on the slide (spec 008 US2): above the element, else below it (each lined up with
 * its left edge, else its right), else just inside its own top edge at its right end
 * (over the end of itself, never another element). Not shown while
 * the canvas is (full-width source pane).
 */
export function placeNear(b, el, frame) {
  const f = frame.getBoundingClientRect();
  b.style.visibility = f.width && f.height ? '' : 'hidden';
  const r = el.getBoundingClientRect();
  const [w, h] = [b.offsetWidth, b.offsetHeight];
  const clamp = (x) => Math.max(4, Math.min(x, innerWidth - w - 4));
  const [atLeft, atRight] = [clamp(f.left + r.left), clamp(f.left + r.right - w)];
  const others = [...el.ownerDocument.querySelectorAll('section.present [data-qd], section.present > :is(h1, h2, h3)')]
    .filter((o) => !o.contains(el) && !el.contains(o))
    .map((o) => { // what it shows, not its block: a one-word heading leaves its row free
      const range = o.ownerDocument.createRange();
      range.selectNodeContents(o);
      const c = range.getBoundingClientRect();
      return c.width && c.height ? c : o.getBoundingClientRect();
    });
  const free = ([left, top]) => top >= f.top + 2 && top + h <= f.bottom - 2
    && !others.some((o) => f.left + o.left < left + w && f.left + o.right > left && f.top + o.top < top + h && f.top + o.bottom > top);
  const [left, top] = [f.top + r.top - h - 8, f.top + r.bottom + 8].flatMap((t) => [[atLeft, t], [atRight, t]]).find(free)
    ?? [clamp(f.left + r.right - w - 4), Math.max(f.top + 2, f.top + r.top + 4)];
  b.style.top = `${top}px`;
  b.style.left = `${left}px`;
}

/** Back to the text after an action; one that opens a sheet (link) does that itself. */
const after = (r) => { if (!r?.then) session.root.focus(); };

export function hideToolbar() {
  closePops();
  session = null;
  bar.hidden = true;
}

// the formatting actions (menus and keys) while typing on a slide
for (const [id, , , run] of [...BUTTONS.filter(Array.isArray), ['nocolor', '', '', noColor]]) {
  keys.on(ACTION[id] || id, {
    enabled: () => !!session && !(session.reg.mode === 'inline' && BLOCK_ONLY.has(id)),
    run: () => after(run()),
  });
}
