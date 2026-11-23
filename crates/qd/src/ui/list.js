// The slide list (spec 001; spec 008 US1): a thumbnail of every slide, the
// title slide first, a slide menu on right-click, and a drag you can see (a picture of
// the slide under the pointer, the others parting where it will land). Every
// change is a qd op (add, move) through `api.op`. Groups (spec 018): a sub-slide
// sits indented under its head, and a head folds its sub-slides away; which
// heads are folded is view state, kept by qd beside the deck, never in it.

import { token } from '/ui/run.js';

/**
 * `api`: {state, frame, go(anchor), op(name, args), fetchDeck(), addSlide, say(msg, isError), regrouped(r, id),
 * slideMenu(id) → menu items, openMenu(items, x, y)}.
 */
export function makeList(api) {
  const list = document.querySelector('#slides');
  const nav = list.closest('nav');
  const thumbs = new Map(); // anchor → {host, html}: kept across redraws, rebuilt only when the slide changed
  const visible = new Set();
  let justDragged = false; // the click that ends a drag isn't a click on the slide
  let shown; // the slide (and its place) the list last scrolled to
  let folded = new Set(); // the heads whose sub-slides are hidden (spec 018 US1)
  let touched = false; // the person's folds before the stored state arrives win over it
  fetch('/api/view').then((r) => r.json()).then((v) => { if (!touched) { folded = new Set(v.collapsed || []); render(); } }).catch(() => {});
  /** Keep `folded` (only heads still there), and redraw. */
  let leaving = null; // the head of a group folded while one of its slides was shown: going to the head
  function fold(id, on) {
    touched = true;
    if (on) folded.add(id); else folded.delete(id);
    // folding the group of the shown slide shows its head, as Keynote does
    const slides = api.state.deck?.slides || [];
    if (on && slides.find((s) => s.anchor === api.state.anchor)?.in === id) {
      leaving = id;
      api.go(slides.find((s) => s.id === id).anchor);
    }
    const heads = new Set(slides.filter((s) => s.head).map((s) => s.id));
    folded = new Set([...folded].filter((h) => heads.has(h)));
    fetch('/api/view', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-QD-Token': token }, body: JSON.stringify({ collapsed: [...folded] }) })
      .then((r) => { if (!r.ok) throw new Error(); }).catch(() => api.say('Could not keep which groups are folded', true));
    render();
  }
  const subsOf = (id) => (api.state.deck?.slides || []).filter((s) => s.in === id);
  const byId = (id) => api.state.deck?.slides.find((s) => s.id === id);

  const el = (tag, cls, text) => Object.assign(document.createElement(tag), cls ? { className: cls } : {}, text ? { textContent: text } : {});
  const frameDoc = () => { try { return api.frame.contentDocument?.querySelector('.reveal .slides') ? api.frame.contentDocument : null; } catch { return null; } };
  const titleText = () => frameDoc()?.querySelector('#title-slide .title')?.textContent.trim() || 'Title slide';
  // the title as the slide shows it (`*Highlights*` → Highlights), the source's before the first render
  // a new slide after `id` (after the title slide: before the first slide)
  const addAfter = (id, at) => {
    const first = api.state.deck?.slides[0];
    return api.addSlide(byId(id) ? { after: id } : first ? { before: first.id } : {}, at);
  };
  const nameOf = (s) => frameDoc()?.getElementById(s.anchor)?.querySelector(':scope > :is(h1, h2, h3)')?.textContent.trim() || s.title || s.summary || 'Untitled';

  // a narrow window's slide picker (spec 008 US8): the list as a menu in the top row.
  // ponytail: names only, no thumbnails; add them if the menu proves hard to scan
  const picker = document.querySelector('#slide-picker');
  const pickable = () => [
    ...(api.state.deck?.title_slide ? [{ label: titleText(), anchor: 'title-slide' }] : []),
    ...(api.state.deck?.slides || []).map((s) => ({ label: `${s.in ? '\u2003' : ''}${s.index} ${nameOf(s)}`, anchor: s.anchor })),
  ];
  picker.addEventListener('click', () => {
    const r = picker.getBoundingClientRect();
    api.openMenu(pickable().map(({ label, anchor }) => ({ label, checked: anchor === api.state.anchor, run: () => api.go(anchor) })), r.left, r.bottom + 2);
  });

  // ---------- thumbnails ----------

  const io = new IntersectionObserver((entries) => {
    for (const e of entries) {
      const a = e.target.dataset.anchor;
      if (e.isIntersecting) { visible.add(a); build(a); } else visible.delete(a);
    }
  }, { root: nav, rootMargin: '200px' });

  function hostFor(anchor) {
    let t = thumbs.get(anchor);
    if (!t) {
      const host = el('div', 'thumb');
      host.dataset.anchor = anchor;
      host.attachShadow({ mode: 'open' });
      t = { host, html: null };
      thumbs.set(anchor, t);
      io.observe(host);
    }
    return t.host;
  }

  // The deck's styles, shared by every thumbnail (one per loaded render). @font-face
  // only works at document level, so fonts go to the editor's document.
  const sheets = new WeakMap();
  let fonts = null;
  const URL_RE = /url\((['"]?)([^'")]+)\1\)/g;
  function deckSheet(doc) {
    let sheet = sheets.get(doc);
    if (sheet) return sheet;
    let css = '';
    let faces = '';
    for (const s of doc.styleSheets) {
      let rules;
      try { rules = s.cssRules; } catch { continue; } // another origin's: skip
      const base = s.href || doc.baseURI;
      for (const r of rules) {
        let text = r.cssText.replace(URL_RE, (m, q, u) => (/^([a-z]+:|#)/i.test(u) ? m : `url("${new URL(u, base).href}")`));
        // the theme's sizes and colours hang off :root/html/body, which a shadow root doesn't have
        if (r.constructor.name === 'CSSStyleRule') text = text.replace(/^[^{]+/, (sel) => sel.replace(/(^|,\s*)(:root|html|body)(?=[\s,.:[#]|$)/g, '$1:host'));
        // (by name: the rules come from the frame's own window, so instanceof is false)
        if (r.constructor.name === 'CSSFontFaceRule') faces += text + '\n'; else css += text + '\n';
      }
    }
    sheet = new CSSStyleSheet();
    sheet.replaceSync(css + THUMB_CSS);
    sheets.set(doc, sheet);
    const f = new CSSStyleSheet();
    f.replaceSync(faces);
    document.adoptedStyleSheets = [...document.adoptedStyleSheets.filter((x) => x !== fonts), f];
    fonts = f;
    return sheet;
  }

  /** A copy of the slide's section from the canvas's own render, in `host`'s shadow root. */
  function draw(host, anchor, doc) {
    const sec = doc.getElementById(anchor);
    if (!sec) return;
    const copy = sec.cloneNode(true);
    for (const n of copy.querySelectorAll('[src], [href], [poster], [data-src]')) {
      for (const a of ['src', 'href', 'poster']) if (n.hasAttribute(a)) n.setAttribute(a, new URL(n.getAttribute(a), doc.baseURI).href);
      if (n.dataset.src && !n.getAttribute('src')) n.setAttribute('src', new URL(n.dataset.src, doc.baseURI).href);
    }
    for (const n of copy.querySelectorAll('.qd-sel, .qd-editing')) n.classList.remove('qd-sel', 'qd-editing');
    copy.removeAttribute('hidden');
    copy.removeAttribute('style');
    copy.classList.add('present');
    const reveal = el('div', 'reveal');
    const slides = el('div', 'slides');
    slides.append(copy);
    reveal.append(slides);
    const { width = 1050, height = 700 } = api.state.deck || {};
    host.style.setProperty('--w', `${width}px`);
    host.style.setProperty('--h', `${height}px`);
    host.style.aspectRatio = `${width} / ${height}`;
    const bodyBg = getComputedStyle(doc.body).backgroundColor;
    const img = sec.dataset.backgroundImage;
    host.style.background = sec.dataset.backgroundColor || bodyBg;
    if (img) host.style.background += ` url("${new URL(img, doc.baseURI).href}") center / cover no-repeat`;
    host.shadowRoot.adoptedStyleSheets = [deckSheet(doc)];
    host.shadowRoot.replaceChildren(reveal);
    scale(host);
  }

  function build(anchor) {
    const t = thumbs.get(anchor);
    const doc = frameDoc();
    if (!t || !doc) return;
    const sec = doc.getElementById(anchor);
    if (!sec) return;
    const html = sec.innerHTML + JSON.stringify(sec.dataset);
    if (t.html === html) return;
    t.html = html;
    draw(t.host, anchor, doc);
    t.host.dataset.built = String(+(t.host.dataset.built || 0) + 1);
  }

  const scale = (host) => host.style.setProperty('--s', String(host.clientWidth / parseFloat(host.style.getPropertyValue('--w') || 1050)));
  new ResizeObserver(() => { for (const { host } of thumbs.values()) scale(host); }).observe(list);

  // ---------- the list ----------


  function item(s, isTitle) {
    const li = el('li');
    li.dataset.id = s.id;
    li.dataset.anchor = s.anchor;
    li.classList.toggle('made-title', isTitle); // Quarto's, from the front matter: not a slide of the file
    const b = el('button');
    b.type = 'button';
    li.append(b);
    if (!isTitle) {
      const handle = el('span', 'handle', '⠿');
      handle.title = 'Drag to move';
      handle.setAttribute('aria-hidden', 'true');
      li.append(handle);
    }
    const meta = el('span', 'meta');
    meta.append(el('span', 'idx', isTitle ? '' : String(s.index)), el('span', 'name', isTitle ? titleText() : nameOf(s)));
    if (s.hidden) meta.append(el('span', 'tag', 'hidden'));
    b.append(hostFor(s.anchor), meta);
    if (s.in) li.classList.add('sub');
    const subs = s.head ? subsOf(s.id).length : 0;
    if (subs) {
      const shut = folded.has(s.id);
      li.classList.add('head');
      const t = el('button', 'disclose', shut ? '▸' : '▾');
      t.type = 'button';
      t.tabIndex = -1; // ← → on the row do the same from the keyboard
      t.setAttribute('aria-expanded', String(!shut));
      t.setAttribute('aria-label', `${shut ? 'Expand' : 'Collapse'} ${nameOf(s)}`);
      t.addEventListener('pointerdown', (e) => e.stopPropagation()); // not a drag
      t.addEventListener('click', () => fold(s.id, !shut));
      li.append(t);
      b.setAttribute('aria-expanded', String(!shut));
      if (shut) meta.append(el('span', 'tag count', `+${subs}`));
    }
    b.addEventListener('click', (e) => {
      if (justDragged || (!isTitle && pick(e, s.id))) return;
      api.state.sel = null;
      api.go(s.anchor);
    });
    if (!isTitle) {
      li.addEventListener('contextmenu', async (e) => {
        e.preventDefault();
        api.openMenu(await api.slideMenu(s.id), e.clientX, e.clientY);
      });
      li.addEventListener('pointerdown', (e) => drag(e, li, s));
    } else {
      // Quarto builds the title slide from the front matter and always shows it first
      li.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        api.openMenu([
          { label: 'Add slide after…', run: () => addAfter(s.id, li) },
          '-',
          { label: 'Always first (made from the title at the top)', disabled: true },
        ], e.clientX, e.clientY);
      });
    }
    return li;
  }

  function render() {
    const deck = api.state.deck;
    if (!deck) return;
    // the shown slide is never folded away (FR-008)
    const showing = deck.slides.find((s) => s.anchor === api.state.anchor);
    if (showing?.in !== leaving) leaving = null;
    if (showing?.in && !leaving && folded.has(showing.in)) { fold(showing.in, false); return; }
    // redrawing must not move the keyboard: back to the same slide afterwards
    const focused = list.contains(document.activeElement) && document.activeElement.closest('li')?.dataset.id;
    // building items moves the thumbnails out of the old ones, and the list's
    // scroll collapses with them: keep it (read first, put back after)
    const { scrollLeft, scrollTop } = nav;
    // an item that shows the same as before stays the same element: replacing it
    // under a press loses the click (the release lands on a new button)
    const old = new Map([...list.children].map((li) => [li.dataset.id, li]));
    const keep = (s, isTitle) => {
      const { lines, ...drawn } = s; // where it sits in the file isn't drawn: an edit above it needn't rebuild it
      const sig = JSON.stringify([drawn, isTitle && titleText(), folded.has(s.id), s.head && subsOf(s.id).length]);
      let li = old.get(s.id);
      if (li?.dataset.sig !== sig) {
        li = item(s, isTitle);
        li.dataset.sig = sig;
      }
      // the shown slide is marked in place, not by rebuilding its item
      const cur = s.anchor === api.state.anchor || (!api.state.anchor && isTitle);
      li.classList.toggle('current', cur);
      if (cur) li.firstElementChild.setAttribute('aria-current', 'true'); else li.firstElementChild.removeAttribute('aria-current');
      return li;
    };
    const items = deck.slides.map((s) => Object.assign(keep(s, false), { hidden: folded.has(s.in) }));
    if (deck.title_slide) items.unshift(keep({ id: 'title-slide', anchor: 'title-slide' }, true));
    // only what changed moves: an item already in its place isn't touched
    let moved = false;
    items.forEach((li, i) => { if (list.children[i] !== li) { list.insertBefore(li, list.children[i] ?? null); moved = true; } });
    while (list.children.length > items.length) { list.lastChild.remove(); moved = true; }
    if (moved) Object.assign(nav, { scrollLeft, scrollTop }); // writing it lays the whole list out: only when needed
    const current = list.querySelector('li.current');
    // only when the shown slide changed or moved (undo, an agent's edit): a redraw
    // must not scroll the list away from where the person scrolled it. On the next
    // frame: the new items have their full height by then.
    const at = `${api.state.anchor}@${items.indexOf(current)}`;
    // (looked up again then: a second redraw in between replaces the items)
    if (shown !== at) requestAnimationFrame(() => list.querySelector('li.current')?.scrollIntoView({ block: 'nearest', inline: 'nearest' }));
    shown = at;
    // the set lasts while the same slide is shown (and only for slides still there)
    if (api.state.anchor !== pickedOn) picked.clear();
    pickedOn = api.state.anchor;
    for (const id of picked) if (!items.some((li) => li.dataset.id === id)) picked.delete(id);
    mark();
    const ps = pickable();
    const here = ps.find((p) => p.anchor === api.state.anchor) ?? ps[0];
    picker.textContent = here ? `${here.label} ▾` : '';
    if (focused) (list.querySelector(`li[data-id="${CSS.escape(focused)}"]`) || current)?.querySelector(':scope > button')?.focus({ preventScroll: true });
  }

  /** A new render is on the canvas: redraw the thumbnails in view (the rest as they scroll in). */
  function refresh() {
    for (const li of list.children) {
      const s = byId(li.dataset.id);
      const name = li.querySelector('.name');
      if (name) name.textContent = s ? nameOf(s) : titleText();
    }
    for (const a of visible) build(a);
  }

  document.querySelector('#add-slide').addEventListener('click', () => {
    const last = api.state.deck?.slides.at(-1);
    api.addSlide(last ? { after: last.id } : {}, document.querySelector('#add-slide'));
  });

  // widget keys: ↑/↓ move the focus along the visible rows, ← folds a head (or goes
  // up to it from a sub-slide) and → unfolds it, Enter adds a slide after the
  // focused one (⌥↑/⌥↓ and Delete are actions: keys.dispatch handles them)
  list.addEventListener('keydown', (e) => {
    const li = e.target.closest('li');
    if (!li || e.altKey || e.metaKey || e.ctrlKey || e.shiftKey) return;
    const s = byId(li.dataset.id);
    const way = { ArrowUp: 'previousElementSibling', ArrowDown: 'nextElementSibling', ArrowLeft: 'left', ArrowRight: 'right', Enter: 'add', Escape: 'leave' }[e.key];
    const turn = way === 'left' || way === 'right';
    // ← → act only on a group (elsewhere they keep scrolling a narrow window's strip)
    const folds = turn && s?.head && subsOf(s.id).length && folded.has(s.id) === (way === 'right');
    if (!way || (turn && !folds && !(way === 'left' && s?.in))) return;
    e.preventDefault();
    if (way === 'leave') {
      api.frame.focus(); // Tab indents here, so Escape is the way out of the list (spec 018 R7)
    } else if (way === 'add') {
      e.stopPropagation();
      addAfter(li.dataset.id, li);
    } else if (folds) {
      fold(s.id, way === 'left');
    } else if (turn) {
      list.querySelector(`li[data-id="${CSS.escape(s.in)}"] > button`)?.focus();
    } else {
      let next = li[way];
      while (next?.hidden) next = next[way];
      next?.querySelector(':scope > button')?.focus();
    }
  });

  // ---------- dragging (pointer events: a drag image and an opening gap) ----------

  function drag(down, li, s) {
    if (down.button !== 0) return;
    const across = getComputedStyle(list).display === 'flex'; // narrow layout: a horizontal strip
    const pos = (e) => (across ? e.clientX : e.clientY);
    const start = { x: down.clientX, y: down.clientY };
    let d = null; // the drag, once it has moved a few points
    let last = down;
    let frameReq = 0;

    const begin = () => {
      const items = [...list.querySelectorAll('li[data-id]:not(.made-title):not([hidden])')];
      const from = items.indexOf(li);
      const r = li.getBoundingClientRect();
      const ghost = el('div', 'drag-ghost');
      const host = el('div', 'thumb');
      host.attachShadow({ mode: 'open' });
      ghost.append(host, el('span', 'name', `${s.title || 'Untitled'}${groupSize(s.id) ? ` +${groupSize(s.id) - 1}` : ''}`));
      ghost.style.width = `${r.width}px`;
      document.body.append(ghost);
      const doc = frameDoc();
      if (doc) draw(host, s.anchor, doc);
      li.classList.add('dragging');
      const scroll0 = across ? nav.scrollLeft : nav.scrollTop;
      const mids = items.map((x) => { const b = x.getBoundingClientRect(); return (across ? b.left + b.width / 2 : b.top + b.height / 2) + scroll0; });
      const step = items.length > 1 ? Math.abs(mids[1] - mids[0]) : (across ? r.width : r.height);
      const rest = items.filter((x, i) => i !== from).map((x) => x.dataset.id); // the others, in order
      d = { items, from, rest, lead: byId(selected(s.id)[0]), to: from, ghost, off: { x: start.x - r.left, y: start.y - r.top }, mids, step };
      document.body.classList.add('dragging-slide');
    };

    const place = () => {
      const scroll = across ? nav.scrollLeft : nav.scrollTop;
      const p = pos(last) + scroll;
      // where it lands: after every other slide whose middle is above the pointer
      d.to = d.items.filter((x, i) => i !== d.from && d.mids[i] < p).length;
      // sideways in the list (not the strip): it lands indented or outdented (spec 018 R8)
      const dx = across ? 0 : last.clientX - start.x;
      d.depth = dx >= INDENT ? 'indent' : dx <= -INDENT ? 'outdent' : null;
      if (d.depth && !depthOk(d)) d.depth = null; // a drag that wanders sideways is still a plain move
      d.ghost.dataset.depth = d.depth || '';
      // a slide that is a head where it lands goes only before a head or at the end (FR-017)
      if (d.to !== d.from && (d.depth === 'outdent' || (d.lead?.head && d.depth !== 'indent'))) d.to = headGap(d); // what moves first, not the row held
      d.items.forEach((x, i) => {
        let shift = 0;
        if (i > d.from && i <= d.to) shift = -d.step;
        else if (i < d.from && i >= d.to) shift = d.step;
        x.style.transform = shift ? (across ? `translateX(${shift}px)` : `translateY(${shift}px)`) : '';
      });
      // kept on screen: dragged left, its hint (← out of the group) stays readable
      d.ghost.style.transform = `translate(${Math.max(0, last.clientX - d.off.x)}px, ${last.clientY - d.off.y}px)`;
    };

    // near the list's edges, scroll it (and keep the landing place under the pointer)
    const autoScroll = () => {
      if (!d) return;
      const n = nav.getBoundingClientRect();
      const [lo, hi] = across ? [n.left, n.right] : [n.top, n.bottom];
      const p = pos(last);
      const v = p < lo + 40 ? -8 : p > hi - 40 ? 8 : 0;
      if (v) { if (across) nav.scrollLeft += v; else nav.scrollTop += v; place(); }
      frameReq = requestAnimationFrame(autoScroll);
    };

    const end = () => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
      removeEventListener('keydown', key, true);
      cancelAnimationFrame(frameReq);
      if (!d) return null;
      d.ghost.remove();
      d.items.forEach((x) => { x.style.transform = ''; });
      li.classList.remove('dragging');
      document.body.classList.remove('dragging-slide');
      justDragged = true;
      setTimeout(() => { justDragged = false; }, 0);
      const done = d;
      d = null;
      return done;
    };

    const move = (e) => {
      last = e;
      if (!d) {
        if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < 4) return;
        begin();
        frameReq = requestAnimationFrame(autoScroll);
      }
      place();
    };
    // can it land one step in or out there (the rules of `qd indent`/`qd outdent`, research R3)?
    const depthOk = (d) => {
      if (api.state.deck?.group_level == null || s.opaque) return false; // no groups, or a slide qd won't restructure
      const above = byId(d.rest[d.to - 1]);
      const sub = !s.head && !!above && (above.head || !!above.in); // where it lands, unchanged
      return d.depth === 'outdent' ? sub : !sub && !!above && (s.head || above.kind !== 'implicit');
    };
    // the landing place nearest `d.to` that is before a head, or the end
    const headGap = (d) => {
      const ok = (to) => to === d.rest.length || byId(d.rest[to])?.head;
      for (let k = 0; k <= d.rest.length; k++) for (const to of [d.to - k, d.to + k]) if (ok(to)) return to;
      return d.rest.length;
    };
    const up = async () => {
      const done = end();
      if (!done) return;
      const depth = done.depth;
      if (done.to === done.from) {
        // dragged sideways in place: only the depth changes
        const r = depth && await api.op(depth, { slide: s.id });
        if (r) { await api.fetchDeck(); api.regrouped(r, s.id); }
        return;
      }
      // the slides that go (the picked ones, if this is one of them) land between the
      // nearest others either side of the gap
      const group = selected(s.id);
      const { rest } = done;
      // after a folded head means after its whole group (its sub-slides aren't rows)
      const prev = rest.slice(0, done.to).filter((id) => !group.includes(id)).at(-1);
      const after = prev && folded.has(prev) ? groupEnd(prev) : prev;
      const behind = group.length === 1 ? leftBehind(s.id) : [];
      const before = rest.slice(done.to).find((id) => !group.includes(id));
      const where = after ? { after } : before ? { before } : null;
      if (!where) return;
      const r = await api.op('move', { slide: group.length > 1 ? group : s.id, ...where, ...(depth ? { [depth]: true } : {}) });
      if (r) { await api.fetchDeck(); api.regrouped(r, group[0]); adopted(behind); }
    };
    const key = (e) => {
      if (e.key !== 'Escape' || !d) return; // Escape puts it back
      e.preventDefault();
      e.stopPropagation();
      end();
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
    addEventListener('keydown', key, true);
  }

  // several slides (spec 008 US9): ⇧-click picks a range, ⌘/Ctrl-click one more;
  // dragging, Delete, copy and cut then act on all of them
  const picked = new Set();
  let pickedOn = null; // the slide shown when the set was made
  const mark = () => {
    for (const li of list.children) {
      const on = picked.has(li.dataset.id);
      li.classList.toggle('picked', on);
      li.querySelector(':scope > button')?.setAttribute('aria-selected', String(on));
    }
  };
  /** A click with ⇧ or ⌘/Ctrl changes the set (true); a plain click empties it. */
  function pick(e, id) {
    const ids = (api.state.deck?.slides || []).map((x) => x.id);
    const here = api.state.deck?.slides.find((s) => s.anchor === api.state.anchor)?.id;
    if (e.shiftKey && here) {
      const [a, b] = [ids.indexOf(here), ids.indexOf(id)].sort((x, y) => x - y);
      picked.clear();
      ids.slice(a, b + 1).forEach((x) => picked.add(x));
    } else if (e.metaKey || e.ctrlKey) {
      if (!picked.size && here) picked.add(here);
      if (picked.has(id)) picked.delete(id); else picked.add(id);
    } else {
      picked.clear();
    }
    mark();
    return e.shiftKey || e.metaKey || e.ctrlKey;
  }
  /** The slides an action on `id` applies to: the picked ones in deck order, if `id` is one. */
  function selected(id) {
    const pick = picked.size < 2 || !picked.has(id) ? new Set([id]) : picked;
    // a folded head stands for its whole group (spec 018 US3), in deck order
    return (api.state.deck?.slides || []).filter((s) => pick.has(s.id) || (folded.has(s.in) && pick.has(s.in))).map((s) => s.id);
  }

  /** The slides in slide `id`'s group when it is a folded head (its count for the menus), else 0. */
  const groupSize = (id) => (folded.has(id) ? 1 + subsOf(id).length : 0);
  /** The last slide of `id`'s group: after it, a group lands whole (FR-017). */
  const groupEnd = (id) => {
    const slides = api.state.deck?.slides || [];
    const s = byId(id);
    const head = s?.head ? s.id : s?.in;
    if (head) return subsOf(head).at(-1)?.id ?? head;
    const top = slides.findIndex((x) => x.head); // a top-level slide: the last one before the first head
    return top > 0 ? slides[top - 1].id : slides.at(-1)?.id ?? id;
  };
  /** After an open head moved or was deleted alone: where its sub-slides went (FR-018). */
  function adopted(subs) {
    if (!subs.length) return;
    const first = byId(subs[0]);
    const n = subs.length, one = n === 1;
    const what = `Its ${n} slide${one ? '' : 's'}`;
    api.say(first?.in ? `${what} now ${one ? 'belongs' : 'belong'} to ${byId(first.in)?.title || 'the group before'}` : `${what} now ${one ? 'stands on its' : 'stand on their'} own`);
  }
  /** The sub-slides of `id` if it is an open head acting alone (they stay behind). */
  const leftBehind = (id) => (byId(id)?.head && !folded.has(id) ? subsOf(id).map((s) => s.id) : []);

  return { render, refresh, selected, groupSize, groupEnd, adopted, leftBehind };
}

// how far sideways a drag goes to land a slide one step in or out (spec 018 R8)
const INDENT = 16;

// A thumbnail is a still copy of the slide: no reveal.js runs in it. ponytail:
// outputs a script draws (Plotly, widgets) show as their static markup.

const THUMB_CSS = `
.reveal { position: absolute; left: 0; top: 0; width: var(--w); height: var(--h); transform: scale(var(--s));
  transform-origin: 0 0; overflow: hidden; pointer-events: none; }
.reveal .slides { position: absolute; inset: 0; width: 100% !important; height: 100% !important; margin: 0 !important;
  transform: none !important; zoom: 1 !important; }
.reveal .slides > section { display: block !important; position: absolute !important; inset: 0; transform: none !important;
  opacity: 1 !important; visibility: visible !important; width: 100% !important; height: 100% !important; }
.reveal .fragment { opacity: 1 !important; visibility: visible !important; transform: none !important; }
/* reveal.js centres these slides with script, which a copy doesn't run */
.reveal .slides > section.center, .reveal .slides > section#title-slide:has(> .title) { display: flex !important; flex-direction: column;
  justify-content: center; }
`;
