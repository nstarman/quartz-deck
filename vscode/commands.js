// The palette commands (spec 006 US4): each runs one qd command; its JSON
// result becomes one line for the person. No `vscode` import.

const ARGS = {
  render: ['render'],
  export: ['export'],
  exportPdf: ['export', '--pdf'],
  checkOffline: ['check', '--offline'],
};

const json = (s) => { try { return JSON.parse(s.trim()); } catch { return null; } };

/** qd's stdout/stderr for `name` → `{ok, text}`. */
function report(name, stdout, stderr) {
  const r = json(stdout);
  if (r?.ok) {
    if (name === 'render') return { ok: true, text: `Rendered: ${r.output}` };
    if (name === 'checkOffline') return { ok: true, text: `Works offline: ${r.slides} slides` };
    const pdf = r.pdf ? `; PDF ${r.pdf.path}, ${r.pdf.pages} pages` : '';
    return { ok: true, text: `Exported: ${r.html} (${r.check.slides} slides, works offline)${pdf}` };
  }
  const [p, more] = [r?.problems?.[0], (r?.problems?.length ?? 0) - 1];
  if (p) {
    const hint = p.hint ? ` — ${p.hint}` : '';
    return { ok: false, text: `${more + 1} offline problem${more ? "s" : ""}: ${p.message}${hint}${more ? ` (and ${more} more)` : ''}` };
  }
  const e = (r || json(stderr))?.error;
  if (e) return { ok: false, text: `${e.message} — ${e.hint}` };
  return { ok: false, text: stderr.trim() || `qd ${ARGS[name][0]} failed` };
}

/** The first of `docs` that is a saved .qmd — not the Output panel, an untitled or another file. */
const deckOf = (docs) => docs.find((d) => d?.uri?.scheme === 'file' && d.fileName.endsWith('.qmd'));

module.exports = { ARGS, report, deckOf };
