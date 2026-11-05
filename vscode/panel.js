// The quartz-deck panel's page (spec 006 R2): one full-size iframe of the
// editor that `qd serve` serves, in framed mode; a tiny relay passes the few
// allowed messages between the editor and the extension. No `vscode` import.

/** `link` (`…/#t=<token>`) with the framed-mode query; the token stays in the fragment. */
function editorUrl(link, { theme, run } = {}) {
  const u = new URL(link);
  u.search = new URLSearchParams({ host: 'vscode', theme, ...(run ? { run } : {}) }).toString();
  return u.toString();
}

/** VS Code's ColorThemeKind (1 light, 2 dark, 3 high contrast, 4 high contrast light). */
const themeKind = (k) => ({ 1: 'light', 2: 'dark' })[k] || 'hc';

const esc = (s) => s.replace(/&/g, '&amp;').replace(/"/g, '&quot;');

/** The webview's page; `nonce` lets only its own relay script run; `state` is
 * kept by VS Code to restore the panel after a restart. */
function html(url, nonce, state = {}) {
  const origin = new URL(url).origin;
  return `<!doctype html><html><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; frame-src ${origin}; script-src 'nonce-${nonce}'; style-src 'unsafe-inline';">
<style>html,body,iframe{margin:0;padding:0;border:0;width:100%;height:100%;overflow:hidden}</style></head>
<body><iframe id="qd" src="${esc(url)}" title="quartz-deck slides"></iframe>
<script nonce="${nonce}">
const vscode = acquireVsCodeApi();
vscode.setState(${JSON.stringify(state).replace(/</g, "\\u003c")});
const frame = document.getElementById('qd');
const origin = ${JSON.stringify(origin)};
addEventListener('message', (e) => {
  if (e.source === frame.contentWindow) vscode.postMessage(e.data); // the editor → the extension (filtered there)
  else if (e.origin.startsWith('vscode-webview:') || e.source === null) frame.contentWindow.postMessage(e.data, origin); // the extension → the editor
});
</script></body></html>`;
}

/** A message from the editor, if it is one the extension acts on; else null. */
function fromEditor(m) {
  if (m?.type === 'ready') return { type: 'ready' };
  if (m?.type === 'reveal' && Array.isArray(m.lines) && m.lines.length === 2 && m.lines.every(Number.isInteger)) {
    return { type: 'reveal', lines: m.lines };
  }
  return null;
}

module.exports = { editorUrl, themeKind, html, fromEditor };
