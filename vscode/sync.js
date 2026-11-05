// The VS Code buffer ⇄ qd sync (spec 006 R4/R5), with no `vscode` import so
// it can be tested on its own. qd's changes arrive as line edits against a
// version; the person's typing goes to qd against the version VS Code last
// saw; when they crossed, a three-way merge keeps both, or the person
// chooses when both changed the same lines. Nothing is ever dropped silently.

/** `text` as lines, each keeping its newline (the last may have none). */
const lines = (t) => t.match(/[^\n]*\n|[^\n]+$/g) || [];

/** Apply `{start, end, text}` edits (lines of the base, in order) to `text`. */
function applyEdits(text, edits) {
  const L = lines(text);
  let out = '';
  let at = 0;
  for (const e of edits) {
    out += L.slice(at, e.start).join('') + e.text;
    at = e.end;
  }
  return out + L.slice(at).join('');
}

/**
 * The change from `a` to `b` as one line edit (common prefix and suffix kept).
 * ponytail: one block per change, not a full diff — enough for typing and for
 * qd's span edits; two separate changes on one side count as one block.
 */
function lineEdits(a, b) {
  const A = lines(a);
  const B = lines(b);
  let p = 0;
  while (p < A.length && p < B.length && A[p] === B[p]) p++;
  let s = 0;
  while (s < A.length - p && s < B.length - p && A[A.length - 1 - s] === B[B.length - 1 - s]) s++;
  if (p === A.length && p === B.length) return [];
  return [{ start: p, end: A.length - s, text: B.slice(p, B.length - s).join('') }];
}

/** Three-way merge: `{text}`, or `{conflict: true}` when both changed the same lines. */
function merge(base, mine, theirs) {
  if (mine === theirs) return { text: mine };
  const [m] = lineEdits(base, mine);
  const [t] = lineEdits(base, theirs);
  if (!m) return { text: theirs };
  if (!t) return { text: mine };
  const overlap = (m.start < t.end && t.start < m.end) || m.start === t.start;
  if (overlap) return { conflict: true };
  return { text: applyEdits(base, [m, t].sort((x, y) => x.start - y.start)) };
}

/**
 * One buffer's sync with qd. Callbacks:
 * - `buffer()` → the buffer's current text
 * - `apply(edits)` → change the buffer (VS Code: a WorkspaceEdit, undoable)
 * - `post({base, text})` → `{status, body}` from `POST /api/buffer`
 * - `source()` → `{text, version}` as qd has it
 * - `conflict(mine, theirs)` → `'mine' | 'theirs'`
 * The caller keeps one call in flight at a time.
 */
class Sync {
  constructor(cb) {
    Object.assign(this, cb);
  }

  /** The buffer was attached at `version`. */
  open(text, version) {
    this.text = text; // the buffer as qd has it
    this.version = version;
  }

  /** The buffer changed (the person typed, or an edit we applied landed). */
  async changed(buffer) {
    if (buffer === this.text) return; // nothing new for qd
    const r = await this.post({ base: this.version, text: buffer });
    if (r.status === 200) {
      this.text = buffer;
      this.version = r.body.version;
    } else if (r.status === 409) {
      await this.reconcile(buffer, r.body);
    }
  }

  /** qd changed the buffer (a panel op, an agent's edit, undo): `buffer_edit`. */
  async bufferEdit({ base, version, edits }) {
    if (base !== this.version) return this.reconcile(this.buffer(), await this.source());
    const theirs = applyEdits(this.text, edits);
    const mine = this.buffer();
    if (mine !== this.text) return this.reconcile(mine, { text: theirs, version }); // unsent typing
    this.text = theirs; // first, so the edit's own change event is seen as nothing new
    this.version = version;
    await this.apply(edits);
  }

  /** `mine` (the buffer) and qd's `{text, version}` both moved on from `this.text`. */
  async reconcile(mine, { text: theirs, version }) {
    const m = merge(this.text, mine, theirs);
    const target = m.conflict ? ((await this.conflict(mine, theirs)) === 'mine' ? mine : theirs) : m.text;
    this.text = theirs;
    this.version = version;
    if (target === mine) await this.changed(mine);
    else await this.apply(lineEdits(mine, target)); // its change event posts it if qd needs it
  }
}

module.exports = { applyEdits, lineEdits, merge, Sync };
