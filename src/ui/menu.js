// Context menu (spec 003 US6): one ARIA menu with submenus, arrow keys,
// Enter/Space, Escape, disabled items shown greyed, kept inside the window.
//
// items: [{ label, shortcut?, checked?, disabled?, run?, items? }, '-', ...]

const root = document.createElement('div');
root.id = 'ctxmenu';
root.hidden = true;
document.body.append(root);
let restoreFocus = null;

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
    if (it.disabled) li.setAttribute('aria-disabled', 'true');
    if (it.items) {
      li.setAttribute('aria-haspopup', 'menu');
      li.append(build(it.items));
      li.addEventListener('mouseenter', () => openSub(li));
    } else if (!it.disabled) {
      li.addEventListener('click', () => { closeMenu(); it.run?.(); });
    }
    li.addEventListener('mouseenter', () => li.focus());
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
  } else if (e.key === 'Escape' || e.key === 'Tab') { e.preventDefault(); closeMenu(); } else if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    if (li?.getAttribute('aria-haspopup')) items(openSub(li))[0]?.focus(); else li?.click();
  }
});

/** Open `items` at page point (x, y). */
export function openMenu(menuItems, x, y) {
  restoreFocus = document.activeElement;
  root.replaceChildren(build(menuItems));
  root.hidden = false;
  root.style.left = Math.max(4, Math.min(x, innerWidth - root.offsetWidth - 4)) + 'px';
  root.style.top = Math.max(4, Math.min(y, innerHeight - root.offsetHeight - 4)) + 'px';
  window.focus(); // take keyboard focus from the preview iframe (right-click there)
  items(root.firstChild)[0]?.focus();
}

export function closeMenu() {
  if (root.hidden) return;
  root.hidden = true;
  root.replaceChildren();
  restoreFocus?.focus?.();
}

export function menuOpen() {
  return !root.hidden;
}

document.addEventListener('mousedown', (e) => { if (!root.contains(e.target)) closeMenu(); }, true);
addEventListener('blur', () => setTimeout(() => { if (!document.hasFocus()) closeMenu(); }, 0));
