// The palette commands (spec 006 US4): each runs one qd command; its JSON
// result becomes one line for the person. No `vscode` import.

const ARGS = {
  render: ['render'],
  export: ['export'],
  exportPdf: ['export', '--pdf'],
  checkOffline: ['check', '--offline'],
};

const json = (s) => { try { return JSON.parse(s); } catch { return null; } };

/** qd's JSON error (`{"error": {message, hint}}`) as one line, or null. */
const qdError = (s) => { const e = json(s)?.error; return e ? `${e.message} — ${e.hint}` : null; };

/** qd's stdout/stderr for `name` → `{ok, text}`. */
function report(name, stdout, stderr) {
  const r = json(stdout);
  if (r?.ok) {
    if (name === 'render') return { ok: true, text: `Rendered: ${r.output}` };
    if (name === 'checkOffline') return { ok: true, text: `Works offline: ${r.slides} slides` };
    const pdf = r.pdf ? `; PDF ${r.pdf.path}, ${r.pdf.pages} pages` : '';
    return { ok: true, text: `Exported: ${r.html} (${r.check.slides} slides, works offline)${pdf}` };
  }
  const ps = r?.problems ?? [];
  if (ps.length) {
    const [p, n] = [ps[0], ps.length];
    const hint = p.hint ? ` — ${p.hint}` : '';
    return { ok: false, text: `${n} offline problem${n > 1 ? 's' : ''}: ${p.message}${hint}${n > 1 ? ` (and ${n - 1} more)` : ''}` };
  }
  return { ok: false, text: qdError(stdout) || qdError(stderr) || stderr.trim() || `qd ${ARGS[name][0]} failed` };
}

/** The first of `docs` that is a saved .qmd — not the Output panel, an untitled or another file. */
const deckOf = (docs) => docs.find((d) => d?.uri?.scheme === 'file' && d.fileName.endsWith('.qmd'));

module.exports = { ARGS, report, deckOf, qdError };
