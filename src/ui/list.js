// The slide list (spec 001; spec 008 US1): a thumbnail of every slide, the
// title slide first, a + in every gap, and a drag you can see (a picture of
// the slide under the pointer, the others parting where it will land). Every
// change is a qd op (add, move) through `api.op`.

/**
 * `api`: {state, frame, go(anchor), op(name, args), fetchDeck(),
 * slideMenu(id) → menu items, openMenu(items, x, y)}.
 */
export function makeList(api) {
  const list = document.querySelector('#slides');
  const nav = list.closest('nav');
  const thumbs = new Map(); // anchor → {host, html}: kept across redraws, rebuilt only when the slide changed
  const visible = new Set();
  let justDragged = false; // the click that ends a drag isn't a click on the slide
  let shown; // the slide (and its place) the list last scrolled to

  const el = (tag, cls, text) => Object.assign(document.createElement(tag), cls ? { className: cls } : {}, text ? { textContent: text } : {});
  const frameDoc = () => { try { return api.frame.contentDocument?.querySelector('.reveal .slides') ? api.frame.contentDocument : null; } catch { return null; } };
  const titleText = () => frameDoc()?.querySelector('#title-slide .title')?.textContent.trim() || 'Title slide';
  const nameOf = (s) => s.title || s.summary || 'Untitled';

  // a narrow window's slide picker (spec 008 US8): the list as a menu in the top row.
  // ponytail: names only, no thumbnails; add them if the menu proves hard to scan
  const picker = document.querySelector('#slide-picker');
  const pickable = () => [
    ...(api.state.deck?.title_slide ? [{ label: titleText(), anchor: 'title-slide' }] : []),
    ...(api.state.deck?.slides || []).map((s) => ({ label: `${s.index} ${nameOf(s)}`, anchor: s.anchor })),
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

  /** A new slide there, from a layout (the gallery opens beside `anchor`, spec 009). */
  const addAt = (where, anchor) => api.addSlide(where, anchor);

  function item(s, isTitle) {
    const li = el('li');
    li.dataset.id = s.id;
    li.dataset.anchor = s.anchor;
    const cur = s.anchor === api.state.anchor || (!api.state.anchor && isTitle);
    if (cur) li.classList.add('current');
    const b = el('button');
    b.type = 'button';
    li.append(b);
    if (!isTitle) {
      const add = el('button', 'add-here', '+');
      add.type = 'button';
      add.title = 'Add a slide here';
      add.setAttribute('aria-label', `Add a slide before ${s.title || 'this slide'}`);
      add.addEventListener('click', () => addAt({ before: s.id }, add));
      const handle = el('span', 'handle', '⠿');
      handle.title = 'Drag to move';
      handle.setAttribute('aria-hidden', 'true');
      li.append(add, handle);
    }
    const meta = el('span', 'meta');
    meta.append(el('span', 'idx', isTitle ? '' : String(s.index)), el('span', 'name', isTitle ? titleText() : nameOf(s)));
    if (s.hidden) meta.append(el('span', 'tag', 'hidden'));
    b.append(hostFor(s.anchor), meta);
    if (cur) b.setAttribute('aria-current', 'true');
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
    }
    return li;
  }

  function render() {
    const deck = api.state.deck;
    if (!deck) return;
    // redrawing must not move the keyboard: back to the same slide afterwards
    const focused = list.contains(document.activeElement) && document.activeElement.closest('li')?.dataset.id;
    // building items moves the thumbnails out of the old ones, and the list's
    // scroll collapses with them: keep it (read first, put back after)
    const { scrollLeft, scrollTop } = nav;
    // an item that shows the same as before stays the same element: replacing it
    // under a press loses the click (the release lands on a new button)
    const old = new Map([...list.children].map((li) => [li.dataset.id, li]));
    const keep = (s, isTitle) => {
      const sig = JSON.stringify([s, s.anchor === api.state.anchor || (!api.state.anchor && isTitle), isTitle && titleText()]);
      const li = old.get(s.id);
      if (li?.dataset.sig === sig) return li;
      const fresh = item(s, isTitle);
      fresh.dataset.sig = sig;
      return fresh;
    };
    const items = deck.slides.map((s) => keep(s, false));
    if (deck.title_slide) items.unshift(keep({ id: 'title-slide', anchor: 'title-slide' }, true));
    // only what changed moves: an item already in its place isn't touched
    items.forEach((li, i) => { if (list.children[i] !== li) list.insertBefore(li, list.children[i] ?? null); });
    while (list.children.length > items.length) list.lastChild.remove();
    Object.assign(nav, { scrollLeft, scrollTop });
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
    const name = list.querySelector('li[data-id=title-slide] .name');
    if (name) name.textContent = titleText();
    for (const a of visible) build(a);
  }

  document.querySelector('#add-slide').addEventListener('click', () => {
    const last = api.state.deck?.slides.at(-1);
    addAt(last ? { after: last.id } : {}, document.querySelector('#add-slide'));
  });

  // widget keys: ↑/↓ move the focus along the list, Enter adds a slide after the
  // focused one (⌥↑/⌥↓ and Delete are actions: keys.dispatch handles them)
  list.addEventListener('keydown', (e) => {
    const li = e.target.closest('li');
    if (!li || e.altKey || e.metaKey || e.ctrlKey || e.shiftKey || e.target.classList.contains('add-here')) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = e.key === 'ArrowDown' ? li.nextElementSibling : li.previousElementSibling;
      next?.querySelector(':scope > button')?.focus();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      const slides = api.state.deck?.slides || [];
      addAt(li.dataset.id === 'title-slide' ? (slides[0] ? { before: slides[0].id } : {}) : { after: li.dataset.id }, li);
    }
  });

  // ---------- dragging (pointer events: a drag image and an opening gap) ----------

  function drag(down, li, s) {
    if (down.button !== 0 || down.target.closest('.add-here')) return;
    const across = getComputedStyle(list).display === 'flex'; // narrow layout: a horizontal strip
    const pos = (e) => (across ? e.clientX : e.clientY);
    const start = { x: down.clientX, y: down.clientY };
    let d = null; // the drag, once it has moved a few points
    let last = down;
    let frameReq = 0;

    const begin = () => {
      const items = [...list.querySelectorAll('li[data-id]:not([data-id=title-slide])')];
      const from = items.indexOf(li);
      const r = li.getBoundingClientRect();
      const ghost = el('div', 'drag-ghost');
      const host = el('div', 'thumb');
      host.attachShadow({ mode: 'open' });
      ghost.append(host, el('span', 'name', s.title || 'Untitled'));
      ghost.style.width = `${r.width}px`;
      document.body.append(ghost);
      const doc = frameDoc();
      if (doc) draw(host, s.anchor, doc);
      li.classList.add('dragging');
      const scroll0 = across ? nav.scrollLeft : nav.scrollTop;
      const mids = items.map((x) => { const b = x.getBoundingClientRect(); return (across ? b.left + b.width / 2 : b.top + b.height / 2) + scroll0; });
      const step = items.length > 1 ? Math.abs(mids[1] - mids[0]) : (across ? r.width : r.height);
      d = { items, from, to: from, ghost, off: { x: start.x - r.left, y: start.y - r.top }, mids, step };
      document.body.classList.add('dragging-slide');
    };

    const place = () => {
      const scroll = across ? nav.scrollLeft : nav.scrollTop;
      const p = pos(last) + scroll;
      // where it lands: after every other slide whose middle is above the pointer
      d.to = d.items.filter((x, i) => i !== d.from && d.mids[i] < p).length;
      d.items.forEach((x, i) => {
        let shift = 0;
        if (i > d.from && i <= d.to) shift = -d.step;
        else if (i < d.from && i >= d.to) shift = d.step;
        x.style.transform = shift ? (across ? `translateX(${shift}px)` : `translateY(${shift}px)`) : '';
      });
      d.ghost.style.transform = `translate(${last.clientX - d.off.x}px, ${last.clientY - d.off.y}px)`;
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
    const up = async () => {
      const done = end();
      if (!done || done.to === done.from) return;
      // the slides that go (the picked ones, if this is one of them) land between the
      // nearest others either side of the gap
      const group = selected(s.id);
      const rest = done.items.filter((x, i) => i !== done.from).map((x) => x.dataset.id);
      const after = rest.slice(0, done.to).filter((id) => !group.includes(id)).at(-1);
      const before = rest.slice(done.to).find((id) => !group.includes(id));
      const where = after ? { after } : before ? { before } : null;
      if (!where) return;
      const r = await api.op('move', { slide: group.length > 1 ? group : s.id, ...where });
      if (r) await api.fetchDeck();
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
    if (picked.size < 2 || !picked.has(id)) return [id];
    return (api.state.deck?.slides || []).map((s) => s.id).filter((x) => picked.has(x));
  }

  return { render, refresh, selected };
}

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
.reveal .slides > section.center, .reveal .slides > section#title-slide { display: flex !important; flex-direction: column;
  justify-content: center; }
`;
