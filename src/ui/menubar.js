// The menu bar (spec 007 US1): File … Help, built from the action list, so
// every action is reachable without right-click or a hidden key. WAI-ARIA
// menubar pattern; the drop-downs are menu.js's menus.

import LIST from '/ui/action-list.js';
import * as keys from '/ui/keys.js';
import { el } from '/ui/run.js';
import { openMenu, closeMenu, menuOpen, onClose } from '/ui/menu.js';

/** The list as a tree: [{label, items: [...]}], separators as '-'. */
function tree() {
  const top = [];
  const node = (path) => {
    let level = top;
    let n = null;
    for (const name of path) {
      n = level.find((x) => x.label === name);
      if (!n) { n = { label: name, items: [] }; level.push(n); }
      level = n.items;
    }
    return n;
  };
  for (const e of LIST) {
    if (!e.menu) continue; // keys only (Back to the text, the menu bar itself)
    node(e.menu).items.push(e.separator ? '-' : e);
  }
  return top;
}
const MENUS = tree();

/** One menu's items for menu.js, resolved against the editor's context now. */
function resolve(items, ctx) {
  const out = [];
  for (const it of items) {
    if (it === '-') { if (out.length && out.at(-1) !== '-') out.push('-'); continue; }
    if (!it.id) { out.push({ label: it.label, items: resolve(it.items, ctx) }); continue; }
    const h = keys.handler(it.id);
    const k = keys.keysOf(it)[0];
    out.push({
      label: it.label,
      shortcut: keys.shortcut(it.id),
      aria: k && keys.aria(k),
      checked: it.toggle ? !!h?.checked?.(ctx) : undefined,
      disabled: !h || h.enabled?.(ctx) === false,
      run: () => { back(); return keys.run(it.id); },
    });
  }
  if (out.at(-1) === '-') out.pop();
  return out;
}

const bar = document.createElement('div');
bar.id = 'menubar';
bar.setAttribute('role', 'menubar');
bar.setAttribute('aria-label', 'Menu');
document.body.prepend(bar);
const buttons = MENUS.map((m, i) => {
  const b = document.createElement('div');
  b.setAttribute('role', 'menuitem');
  b.setAttribute('aria-haspopup', 'menu');
  b.setAttribute('aria-expanded', 'false');
  b.dataset.menu = m.label;
  b.tabIndex = i === 0 ? 0 : -1;
  b.textContent = m.label;
  bar.append(b);
  return b;
});

let open = -1; // index of the open menu
let before = null; // where focus was before F10
/** Focus back to where it was before F10 (after Escape, or once an item runs). */
function back() {
  const b = before;
  before = null;
  b?.focus?.();
}

// the run bar belongs to the Quarto extension in VS Code (spec 006): no Run menu then
const visible = () => buttons.filter((b) => !(b.dataset.menu === 'Run' && document.documentElement.classList.contains('run-external')));

async function show(i, { focus = true } = {}) {
  const b = buttons[i];
  const ctx = await keys.currentContext();
  buttons.forEach((x) => x.setAttribute('aria-expanded', String(x === b)));
  open = i;
  const r = b.getBoundingClientRect();
  openMenu(resolve(MENUS[i].items, ctx), r.left, r.bottom + 2, { focus, onSide: (d) => step(d, true) });
}

function step(d, opened) {
  const vis = visible();
  const at = vis.indexOf(buttons[open >= 0 ? open : buttons.indexOf(document.activeElement)]);
  const next = vis[(at + d + vis.length) % vis.length];
  next.focus();
  if (opened) show(buttons.indexOf(next));
}

onClose(() => { open = -1; buttons.forEach((x) => x.setAttribute('aria-expanded', 'false')); });

bar.addEventListener('mousedown', (e) => {
  const b = e.target.closest('[role=menuitem]');
  if (!b) return;
  e.preventDefault(); // focus stays where it is: an edit on the slide stays open
  const i = buttons.indexOf(b);
  if (open === i) closeMenu(); else show(i, { focus: false });
});
bar.addEventListener('mouseover', (e) => {
  const b = e.target.closest('[role=menuitem]');
  if (b && open >= 0 && buttons.indexOf(b) !== open) show(buttons.indexOf(b), { focus: false });
});
// widget keys: moving along the bar itself (inside an open menu, menu.js has the keys)
bar.addEventListener('keydown', (e) => {
  const b = e.target.closest('[role=menuitem]');
  if (!b) return;
  if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') { e.preventDefault(); step(e.key === 'ArrowLeft' ? -1 : 1, false); }
  else if (e.key === 'ArrowDown' || e.key === 'Enter' || e.key === ' ') { e.preventDefault(); show(buttons.indexOf(b)); }
  else if (e.key === 'Escape') { e.preventDefault(); back(); }
  else return;
  e.stopPropagation();
});

/** F10: to the menu bar (and Escape back to where focus was). */
keys.on('menubar', () => {
  if (menuOpen()) closeMenu();
  if (!bar.contains(document.activeElement)) before = document.activeElement;
  window.focus(); // out of the slide frame first, or the frame keeps the keyboard (as menu.js does)
  visible()[0].focus();
});

/** For tests: the menus as the person would see them now. */
window.qdMenus = async () => {
  const ctx = await keys.currentContext();
  const flat = (items) => items.map((x) => (x === '-' ? '-' : { label: x.label, shortcut: x.shortcut, checked: x.checked, disabled: x.disabled, items: x.items && flat(x.items) }));
  return Object.fromEntries(visible().map((b) => [b.dataset.menu, flat(resolve(MENUS[buttons.indexOf(b)].items, ctx))]));
};

// ---------- the shortcut reference (spec 007 US3): Help ▸ Keyboard shortcuts, or ? ----------

// where a key works, named only when it is restricted (on the slide generally: no note)
const PLACES = { slides: 'slide list', element: 'selected element', text: 'while typing', source: 'source pane' };
const ref = document.createElement('dialog');
ref.id = 'shortcuts';
ref.setAttribute('aria-labelledby', 'shortcuts-title');
document.body.append(ref);

/** Every action with a key here, grouped like the menus (keys only: "Other"). */
function reference() {
  // not type=search: there Escape clears the field instead of closing
  const search = el('input', { type: 'text', placeholder: 'Search', ariaLabel: 'Search shortcuts' });
  const td = (textContent) => el('td', { textContent });
  const sections = [...MENUS.map((m) => m.label), 'Other'].map((name) => {
    const rows = keys.ACTIONS.filter((a) => keys.keysOf(a).length && (a.menu?.[0] ?? 'Other') === name).map((a) => {
      const tr = el('tr');
      tr.dataset.id = a.id;
      tr.append(td(a.menu?.length > 1 ? `${a.menu.slice(1).join(' ▸ ')} ▸ ${a.label}` : a.label),
        td(keys.keysOf(a).map(keys.show).join(' or ')), td(a.where.some((w) => w === 'anywhere' || w === 'slide') ? '' : a.where.map((w) => PLACES[w]).join(', ')));
      return tr;
    });
    if (!rows.length) return null;
    const table = el('table');
    table.append(...rows);
    const sec = el('section');
    sec.append(el('h3', { textContent: name }), table);
    return sec;
  }).filter(Boolean);
  search.addEventListener('input', () => {
    const q = search.value.trim().toLowerCase();
    for (const sec of sections) {
      const rows = [...sec.querySelectorAll('tr')];
      for (const tr of rows) tr.hidden = !!q && !(tr.dataset.id + ' ' + tr.textContent).toLowerCase().includes(q);
      sec.hidden = rows.every((tr) => tr.hidden);
    }
  });
  const close = el('button', { type: 'button', textContent: 'Close' });
  close.addEventListener('click', () => ref.close());
  ref.replaceChildren(el('h2', { id: 'shortcuts-title', textContent: 'Keyboard shortcuts' }), search, ...sections, close);
  ref.showModal(); // Escape closes it; focus returns where it was
  search.focus();
}
keys.on('shortcuts', reference);
