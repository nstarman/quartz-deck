// On-slide text editing: DOM <-> markdown (spec 003 research R1).
//
// serialize*() turn a rendered element back into markdown. align() maps that
// markdown onto the element's real source, allowing only differences that
// render the same (typography, whitespace runs, optional escapes). An element
// is typeable only if its unedited rendering aligns with its source (the
// round-trip gate); a save then rewrites just the source region under the
// characters that changed (splice), so everything else stays byte-identical.

export class Unsupported extends Error {}

// Pandoc's smart typography, rendered -> source.
const TYPO = { '“': '"', '”': '"', '‘': "'", '’': "'", '–': '--', '—': '---', '…': '...' };
const PUNCT = /[\\`*_{}[\]()<>#+\-.!|$@^~"'=:;,?/&%]/;
const WS = /\s/;

// ---------- serialize ----------

function escapeText(s, atLineStart) {
  let out = s.replace(/ /g, ' ').replace(/[\\`*_[\]<>$@^~]/g, (c) => '\\' + c);
  if (atLineStart) out = out.replace(/^(\s*)(#|[-+]\s|\d+[.)]\s|>)/, '$1\\$2');
  return out;
}

function fence(code) {
  const runs = code.match(/`+/g) || [];
  const n = runs.reduce((m, r) => Math.max(m, r.length), 0) + 1;
  const t = '`'.repeat(n);
  return n > 1 || /^`|`$/.test(code) ? `${t} ${code} ${t}` : `${t}${code}${t}`;
}

const SIZE_STYLE = /^font-size:\s*[\d.]+em$/;
const COLOR_STYLE = /^color:\s*#[0-9a-f]{3,8}$/i;

/** Markdown for the inline content of `node`. Throws Unsupported. */
export function serializeInline(node) {
  let out = '';
  for (const n of node.childNodes) {
    if (n.nodeType === 3) { out += escapeText(n.data, out === '' || out.endsWith('\n')); continue; }
    if (n.nodeType !== 1) continue;
    const tag = n.tagName;
    const inner = () => serializeInline(n);
    // attributes we would drop (classes, ids...) mean the source has more than we can write
    if (n.attributes.length && tag !== 'A' && tag !== 'SPAN') throw new Unsupported(tag + ' with attributes');
    switch (tag) {
      case 'STRONG': case 'B': out += `**${inner()}**`; break;
      case 'EM': case 'I': out += `*${inner()}*`; break;
      case 'DEL': case 'S': case 'STRIKE': out += `~~${inner()}~~`; break;
      case 'U': out += `[${inner()}]{.underline}`; break;
      case 'MARK': out += `[${inner()}]{.mark}`; break;
      case 'SUP': out += `^${inner()}^`; break;
      case 'SUB': out += `~${inner()}~`; break;
      case 'CODE': out += fence(n.textContent); break;
      case 'BR': out += '\\\n'; break;
      case 'A': {
        const extra = [...n.attributes].filter((a) => a.name !== 'href');
        if (!n.hasAttribute('href') || extra.length) throw new Unsupported('link attributes');
        out += `[${inner()}](${n.getAttribute('href')})`;
        break;
      }
      case 'SPAN': {
        const style = (n.getAttribute('style') || '').trim();
        const classes = [...n.classList];
        const others = [...n.attributes].filter((a) => !['style', 'class'].includes(a.name));
        const okStyle = style.split(';').filter((p) => p.trim()).every((p) => SIZE_STYLE.test(p.trim()) || COLOR_STYLE.test(p.trim()));
        if (others.length || !okStyle || classes.some((c) => c.startsWith('math') || c.startsWith('citation'))) {
          throw new Unsupported('span');
        }
        const attrs = [...classes.map((c) => '.' + c), ...(style ? [`style="${style}"`] : [])].join(' ');
        out += attrs ? `[${inner()}]{${attrs}}` : inner();
        break;
      }
      default: throw new Unsupported(tag);
    }
  }
  return out;
}

function isBlank(n) { return n.nodeType === 3 && !n.data.trim(); }

const paragraph = (n) => serializeInline(n).trim();

function listItems(list, hints) {
  const ordered = list.tagName === 'OL';
  // Pandoc writes type="1" on every numbered list; `start` comes from the source
  const known = (a) => ordered && (a.name === 'start' || (a.name === 'type' && a.value === '1'));
  if ([...list.attributes].some((a) => !known(a))) throw new Unsupported('list attributes');
  const start = Number(list.getAttribute('start') || 1);
  const items = [...list.children];
  if (items.some((li) => li.tagName !== 'LI' || li.attributes.length)) throw new Unsupported('list item');
  const loose = items.some((li) => [...li.children].some((c) => c.tagName === 'P'));
  const sep = loose ? '\n\n' : '\n';
  return items.map((li, k) => {
    const marker = ordered ? `${start + k}.` : hints.bullet || '-';
    const pad = ' '.repeat(marker.length + 1);
    const parts = [];
    let run = null; // inline content of a tight item
    const flush = () => { if (run !== null) { parts.push(run.trim()); run = null; } };
    for (const c of li.childNodes) {
      const isBlock = c.nodeType === 1 && /^(P|UL|OL|DIV|BLOCKQUOTE)$/.test(c.tagName);
      if (!isBlock) {
        if (c.nodeType !== 1 && c.nodeType !== 3) continue;
        const wrap = li.ownerDocument.createElement('span');
        wrap.append(c.cloneNode(true));
        run = (run ?? '') + serializeInline(wrap);
        continue;
      }
      flush();
      parts.push(c.tagName === 'UL' || c.tagName === 'OL' ? listItems(c, hints) : block(c, hints));
    }
    flush();
    const body = parts.filter((x) => x !== '').join(sep);
    return marker + ' ' + body.split('\n').map((l, n) => (n && l ? pad + l : l)).join('\n');
  }).join(sep);
}

function block(n, hints) {
  switch (n.tagName) {
    case 'P':
      if (n.attributes.length) throw new Unsupported('p attributes');
      return paragraph(n);
    case 'DIV':
      // a line Chrome inserted while typing (no attributes): treat as a paragraph
      if (n.attributes.length || [...n.children].some((c) => /^(P|DIV|UL|OL|BLOCKQUOTE|H\d)$/.test(c.tagName))) {
        throw new Unsupported('div');
      }
      return paragraph(n);
    case 'UL': case 'OL': return listItems(n, hints);
    case 'BLOCKQUOTE':
      if (n.attributes.length) throw new Unsupported('blockquote attributes');
      return serializeBlocks(n, hints).replace(/^/gm, '> ').replace(/^> $/gm, '>');
    case 'H1': case 'H2': case 'H3': case 'H4': case 'H5': case 'H6':
      if (n.attributes.length) throw new Unsupported('heading attributes');
      return '#'.repeat(Number(n.tagName[1])) + ' ' + paragraph(n);
    default: throw new Unsupported(n.tagName);
  }
}

/** Markdown for the block content of `root` (its children). Throws Unsupported. */
export function serializeBlocks(root, hints = {}) {
  const out = [];
  for (const n of root.childNodes) {
    if (isBlank(n)) continue;
    if (n.nodeType === 3) throw new Unsupported('loose text');
    if (n.nodeType !== 1) continue;
    if (n.tagName === 'BR') continue; // Chrome's placeholder in an emptied block
    out.push(block(n, hints));
  }
  return out.filter((b) => b !== '').join('\n\n');
}

// ---------- source regions ----------

/**
 * Split a block's source into the part the rendered element shows (`inner`)
 * and the markup around it. `mode` says how to serialize the element.
 */
export function region(kind, source) {
  const nl = source.includes('\r\n') ? '\r\n' : '\n';
  const lines = source.split(nl);
  if (/^\s*:{3,}/.test(lines[0]) && lines.length >= 2 && /^\s*:{3,}\s*$/.test(lines[lines.length - 1])) {
    return {
      mode: 'blocks',
      prefix: lines[0] + nl,
      inner: lines.slice(1, -1).join(nl),
      suffix: nl + lines[lines.length - 1],
    };
  }
  if (kind === 'heading') {
    const m = source.match(/^(#+[ \t]+)(.*?)([ \t]+\{[^{}]*\}[ \t]*)?$/s);
    if (!m) return null;
    return { mode: 'inline', prefix: m[1], inner: m[2], suffix: m[3] || '' };
  }
  return { mode: 'blocks', prefix: '', inner: source, suffix: '' };
}

/** Bullet marker used in the source, so a rewritten list keeps it. */
export function hintsFor(source) {
  const m = source.match(/^\s*([-*+])\s/m);
  return { bullet: m ? m[1] : '-' };
}

// ---------- align + splice ----------

/**
 * Map positions of rendered markdown `S` onto source `T`. Returns an array of
 * length S.length + 1 with the T index at each S index that starts a token
 * (-1 inside multi-character tokens), or null when they don't correspond.
 */
export function align(S, T) {
  const pos = new Array(S.length + 1).fill(-1);
  let i = 0;
  let j = 0;
  while (i < S.length || j < T.length) {
    const s = S[i];
    const t = T[j];
    if (i < S.length && j < T.length && WS.test(s) && WS.test(t)) {
      pos[i] = j;
      while (i < S.length && WS.test(S[i])) i++;
      for (;;) {
        while (j < T.length && WS.test(T[j])) j++;
        // a soft break rendered as a space: the source's continuation line may
        // start with a block-quote marker that the rendering doesn't repeat
        if (T[j] === '>' && S[i] !== '>' && /\n[ \t]*$/.test(T.slice(Math.max(0, j - 8), j))) { j++; continue; }
        break;
      }
      continue;
    }
    if (i === S.length || j === T.length) {
      // trailing whitespace on either side
      if (i < S.length && WS.test(s)) { pos[i] = j; while (i < S.length && WS.test(S[i])) i++; continue; }
      if (j < T.length && WS.test(t)) { j = T.length; continue; }
      return null;
    }
    if (s === t) { pos[i] = j; i++; j++; continue; }
    const typo = TYPO[s];
    if (typo && T.startsWith(typo, j)) { pos[i] = j; i++; j += typo.length; continue; }
    if (s === '\\' && i + 1 < S.length && S[i + 1] === t && PUNCT.test(t)) { pos[i] = j; i += 2; j++; continue; }
    if (t === '\\' && j + 1 < T.length && T[j + 1] === s && PUNCT.test(s)) { pos[i] = j; i++; j += 2; continue; }
    return null;
  }
  pos[S.length] = T.length;
  return pos;
}

/** New source for `T` after the rendered markdown changed from S0 to S1. */
/** Changed part of a→b: `start`, its end in a (`endA`) and in b (`endB`). */
export function diffRegion(a, b) {
  let p = 0;
  while (p < a.length && p < b.length && a[p] === b[p]) p++;
  let q = 0;
  while (q < a.length - p && q < b.length - p && a[a.length - 1 - q] === b[b.length - 1 - q]) q++;
  return { start: p, endA: a.length - q, endB: b.length - q };
}

export function splice(T, S0, S1, pos) {
  if (S0 === S1) return T;
  const r = diffRegion(S0, S1);
  let a = r.start;
  let b = r.endA;
  // widen to whole tokens so a mapped source sequence is never split
  while (a > 0 && pos[a] === -1) a--;
  while (b < S0.length && pos[b] === -1) b++;
  const ta = pos[a];
  const tb = pos[b];
  let text = S1.slice(a, S1.length - (S0.length - b));
  text = text.replace(/ /g, ' ');
  // keep the source's typography style: ASCII sources get ASCII back
  if (!/[“”‘’–—…]/.test(T)) {
    text = text.replace(/[“”‘’–—…]/g, (c) => TYPO[c]);
  }
  return T.slice(0, ta) + text + T.slice(tb);
}

/**
 * Round-trip gate: the serialized element and the map onto `inner`, or null if
 * the element can't be edited by typing (spec FR-006).
 */
export function gate(root, reg, hints) {
  try {
    const S0 = reserialize(root, reg, hints);
    const pos = align(S0, reg.inner);
    return pos ? { S0, pos } : null;
  } catch (e) {
    if (e instanceof Unsupported) return null;
    throw e;
  }
}

/** Serialize the edited element again (same mode as the gate). */
export function reserialize(root, reg, hints) {
  const t = tidy(root);
  return reg.mode === 'inline' ? serializeInline(t).trim() : serializeBlocks(t, hints);
}

/**
 * A copy of `root` without what the browser's own editing adds and nobody chose
 * (a joined line's computed font-size and rgb() colour, an indent's styled
 * blockquote, <font>): what markdown can hold and someone chose is kept (a #hex
 * colour from the colour menu, an em size, the text). The browser writes the
 * colours it copies from styling as rgb(), never #hex, so those go. Markup that already serializes is left exactly as it is, so the source
 * around an edit never changes.
 */
function tidy(root) {
  const c = root.cloneNode(true);
  for (const n of [...c.querySelectorAll('*')].reverse()) { // deepest first
    if (n.tagName === 'FONT') {
      const span = n.ownerDocument.createElement('span');
      const col = n.getAttribute('color');
      if (/^#[0-9a-f]{3,8}$/i.test(col || '')) span.setAttribute('style', `color:${col}`);
      span.append(...n.childNodes);
      n.replaceWith(span);
      if (!span.attributes.length) span.replaceWith(...span.childNodes);
      continue;
    }
    const style = n.getAttribute('style');
    if (style !== null) {
      const decls = style.split(';').map((d) => d.trim()).filter(Boolean);
      if (n.tagName !== 'SPAN' || !decls.every((d) => SIZE_STYLE.test(d) || COLOR_STYLE.test(d))) {
        const kept = n.tagName !== 'SPAN' ? [] : decls.flatMap((d) => {
          const [k, ...v] = d.split(':');
          const key = k.trim().toLowerCase();
          const val = v.join(':').trim();
          if (key === 'color') return /^#[0-9a-f]{3,8}$/i.test(val) ? [`color:${val}`] : [];
          return key === 'font-size' && /^[\d.]+em$/.test(val) ? [`font-size:${val}`] : [];
        });
        if (kept.length) n.setAttribute('style', kept.join(';')); else n.removeAttribute('style');
      }
    }
    if (n.getAttribute('class') === '') n.removeAttribute('class');
    if (n.tagName === 'SPAN' && !n.attributes.length) n.replaceWith(...n.childNodes);
  }
  return c;
}
