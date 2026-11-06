// Context menu (spec 003 US6): one ARIA menu with submenus, arrow keys,
// Enter/Space, Escape, disabled items shown greyed, kept inside the window.
//
// items: [{ label, shortcut?, aria?, checked?, disabled?, run?, items? }, '-', ...]
// The keys here operate the open menu itself (widget keys, spec 007).

const root = document.createElement('div');
root.id = 'ctxmenu';
root.hidden = true;
document.body.append(root);
let restoreFocus = null;
let onSide = null; // the menu bar's: ← / → at the top level move to the next menu

function build(items) {
  const ul = document.createElement('ul');
  ul.setAttribute('role', 'menu');
  for (const it of items) {
    const li = document.createElement('li');
    if (it === '-') { li.setAttribute('role', 'separator'); ul.append(li); continue; }
    li.setAttribute('role', it.checked === undefined ? 'menuitem' : 'menuitemcheckbox');
    if (it.checked !== undefined) li.setAttribute('aria-checked', String(!!it.checked));
    li.tabIndex = -1;
    li.textContent = it.label;
    if (it.shortcut) li.append(Object.assign(document.createElement('kbd'), { textContent: it.shortcut }));
    if (it.aria) li.setAttribute('aria-keyshortcuts', it.aria);
    if (it.disabled) li.setAttribute('aria-disabled', 'true');
    if (it.items) {
      li.setAttribute('aria-haspopup', 'menu');
      li.append(build(it.items));
      li.addEventListener('mouseenter', () => openSub(li));
    } else if (!it.disabled) {
      li.addEventListener('click', () => { closeMenu(); it.run?.(); });
    }
    // the mouse never takes focus: an edit open on the slide stays open (Format items)
    li.addEventListener('mousedown', (e) => e.preventDefault());
    li.addEventListener('mouseenter', () => { if (root.contains(document.activeElement)) li.focus(); });
    ul.append(li);
  }
  return ul;
}

function openSub(li) {
  li.parentElement.querySelectorAll(':scope > li.open').forEach((o) => o !== li && o.classList.remove('open'));
  li.classList.add('open');
  const sub = li.querySelector(':scope > ul');
  // flip to the left when it would leave the window
  sub.classList.toggle('left', li.getBoundingClientRect().right + sub.offsetWidth > innerWidth - 4);
  return sub;
}

function items(ul) {
  return [...ul.querySelectorAll(':scope > li[tabindex]')].filter((li) => li.getAttribute('aria-disabled') !== 'true');
}

root.addEventListener('keydown', (e) => {
  const li = document.activeElement?.closest?.('li');
  const ul = li?.parentElement || root.firstChild;
  const list = items(ul);
  const i = list.indexOf(li);
  const move = (d) => list[(i + d + list.length) % list.length]?.focus();
  e.stopPropagation();
  if (e.key === 'ArrowDown') { e.preventDefault(); move(1); } else if (e.key === 'ArrowUp') { e.preventDefault(); move(-1); } else if (e.key === 'ArrowRight' && li?.getAttribute('aria-haspopup')) {
    e.preventDefault();
    items(openSub(li))[0]?.focus();
  } else if ((e.key === 'ArrowLeft' || e.key === 'Escape') && ul !== root.firstChild) {
    e.preventDefault();
    const parent = ul.parentElement;
    parent.classList.remove('open');
    parent.focus();
  } else if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && onSide) {
    e.preventDefault();
    onSide(e.key === 'ArrowLeft' ? -1 : 1);
  } else if (e.key === 'Escape' || e.key === 'Tab') { e.preventDefault(); closeMenu(); } else if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    if (li?.getAttribute('aria-haspopup')) items(openSub(li))[0]?.focus(); else li?.click();
  } else if (e.key.length === 1 && /\S/.test(e.key) && !e.metaKey && !e.ctrlKey) {
    // type-ahead: the next item starting with that letter
    const k = e.key.toLowerCase();
    const next = [...list.slice(i + 1), ...list.slice(0, i + 1)].find((x) => x.firstChild.textContent.trim().toLowerCase().startsWith(k));
    next?.focus();
  }
});

/**
 * Open `items` at page point (x, y). `opts.focus: false` leaves focus where it is
 * (the menu bar opened by the mouse, so an edit on the slide stays open);
 * `opts.onSide(dir)` takes ← / → at the top level.
 */
export function openMenu(menuItems, x, y, opts = {}) {
  if (root.hidden) restoreFocus = document.activeElement;
  onSide = opts.onSide || null;
  root.replaceChildren(build(menuItems));
  root.hidden = false;
  root.style.left = Math.max(4, Math.min(x, innerWidth - root.offsetWidth - 4)) + 'px';
  root.style.top = Math.max(4, Math.min(y, innerHeight - root.offsetHeight - 4)) + 'px';
  if (opts.focus === false) return;
  window.focus(); // take keyboard focus from the preview iframe (right-click there)
  items(root.firstChild)[0]?.focus();
}

export function closeMenu() {
  if (root.hidden) return;
  root.hidden = true;
  root.replaceChildren();
  onSide = null;
  restoreFocus?.focus?.();
  closed?.();
}
let closed = null;
/** Called after any menu closes (the menu bar resets its state). */
export function onClose(f) { closed = f; }

export function menuOpen() {
  return !root.hidden;
}

// a click elsewhere closes it; the menu bar handles its own clicks (its name again closes, another switches)
document.addEventListener('mousedown', (e) => { if (!e.target.closest?.('#ctxmenu, #menubar')) closeMenu(); }, true);
addEventListener('blur', () => setTimeout(() => { if (!document.hasFocus()) closeMenu(); }, 0));
