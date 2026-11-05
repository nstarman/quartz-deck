// quartz-deck for VS Code (spec 006): the quartz-deck editor in a panel beside
// the .qmd. While the panel is open the VS Code buffer *is* the deck for qd
// (the buffer overlay): edits on the slides, agents' `qd` edits and undo land
// in the buffer as ordinary unsaved, undoable edits; the file changes when
// the person saves.

const vscode = require('vscode');
const { spawn } = require('node:child_process');
const path = require('node:path');
const crypto = require('node:crypto');
const { Sync } = require('./sync.js');
const { editorUrl, themeKind, html, fromEditor, follow } = require('./panel.js');

/** Open decks: fsPath → { panel, doc, dispose() }. */
const open = new Map();
let out;

/** `qd serve` for `file`: resolves to `{child, port, token, link}` once it prints its link. */
function serve(file) {
  const qd = vscode.workspace.getConfiguration('quartz-deck').get('qdPath') || 'qd';
  return new Promise((resolve, reject) => {
    const child = spawn(qd, ['--deck', file, 'serve', '--port', '0'], { cwd: path.dirname(file) });
    let buf = '';
    let err = '';
    child.on('error', (e) => reject(new Error(e.code === 'ENOENT'
      ? `qd not found ("${qd}"): install quartz-deck, or set quartz-deck.qdPath`
      : e.message)));
    child.stderr.on('data', (d) => { err += d; });
    child.stdout.on('data', (d) => {
      buf += d;
      const nl = buf.indexOf('\n');
      if (nl < 0) return;
      try {
        const v = JSON.parse(buf.slice(0, nl));
        resolve({ child, port: v.port, token: new URL(v.url).hash.replace('#t=', ''), link: v.url });
      } catch (e) { reject(e); }
    });
    child.on('exit', () => {
      // qd's errors are JSON with a hint (e.g. E_NO_QUARTO: install Quarto)
      let msg = err.trim();
      try { const e = JSON.parse(msg).error; msg = `${e.message} — ${e.hint}`; } catch {}
      reject(new Error(msg || 'qd serve stopped'));
    });
  });
}

/** A small client for that server (the extension runs where qd does). */
function client({ port, token }) {
  const base = `http://127.0.0.1:${port}`;
  const headers = { 'Content-Type': 'application/json', 'X-QD-Token': token };
  return {
    post: async (p, body) => {
      const r = await fetch(base + p, { method: 'POST', headers, body: JSON.stringify(body) });
      return { status: r.status, body: await r.json().catch(() => ({})) };
    },
    get: async (p) => (await fetch(base + p, { headers })).json(),
    /** `on(event, data)` for each server-sent event until `signal` aborts. */
    events: async (on, signal) => {
      const r = await fetch(base + '/events', { signal });
      const reader = r.body.pipeThrough(new TextDecoderStream()).getReader();
      let buf = '';
      for (;;) {
        const { value, done } = await reader.read();
        if (done) return;
        buf += value;
        let i;
        while ((i = buf.indexOf('\n\n')) >= 0) {
          const block = buf.slice(0, i);
          buf = buf.slice(i + 2);
          const ev = /^event: ?(.*)$/m.exec(block)?.[1];
          const data = /^data: ?(.*)$/m.exec(block)?.[1];
          if (ev && data) { try { on(ev, JSON.parse(data)); } catch {} }
        }
      }
    },
  };
}

/** One call at a time against qd (typing, qd's edits, merges). */
function serial() {
  let tail = Promise.resolve();
  return (fn) => { tail = tail.then(fn).catch((e) => out.appendLine(String(e?.stack || e))); return tail; };
}

/** Line edits → one WorkspaceEdit on `doc` (undoable, marks it unsaved). */
async function applyEdits(doc, edits) {
  const we = new vscode.WorkspaceEdit();
  for (const e of edits) {
    we.replace(doc.uri, doc.validateRange(new vscode.Range(e.start, 0, e.end, 0)), e.text);
  }
  await vscode.workspace.applyEdit(we);
}

/** Both sides changed the same lines: the person chooses (never silently). */
async function choose(doc, mine, theirs) {
  for (;;) {
    const pick = await vscode.window.showWarningMessage(
      `quartz-deck: you and qd changed the same lines in ${path.basename(doc.fileName)}.`,
      { modal: true, detail: 'Keep your version, or take the one from the slides / an agent.' },
      'Keep mine', 'Take theirs', 'Compare',
    );
    if (pick === 'Keep mine') return 'mine';
    if (pick === 'Take theirs') return 'theirs';
    if (pick === 'Compare') {
      const [a, b] = await Promise.all([mine, theirs].map((content) => vscode.workspace.openTextDocument({ content, language: 'quarto' })));
      await vscode.commands.executeCommand('vscode.diff', a.uri, b.uri, 'Yours ↔ qd’s');
    }
    // dismissed or compared: ask again; both versions stay intact meanwhile
  }
}

/** Open (or reveal) the panel for `doc`; `panel` is given when VS Code restores one. */
async function openDeck(doc, panel) {
  const file = doc.uri.fsPath;
  if (open.has(file)) return open.get(file).panel.reveal(vscode.ViewColumn.Beside);
  if (doc.isUntitled || path.extname(file) !== '.qmd') {
    throw new Error('quartz-deck needs a saved .qmd file');
  }
  const srv = await serve(file);
  const api = client(srv);
  const run = serial();
  const ext = await vscode.env.asExternalUri(vscode.Uri.parse(`http://127.0.0.1:${srv.port}/`));
  const link = `${ext.toString(true).replace(/\/$/, '')}/#t=${srv.token}`;
  const quarto = !!vscode.extensions.getExtension('quarto.quarto');

  panel ??= vscode.window.createWebviewPanel('quartz-deck', `Slides: ${path.basename(file)}`,
    { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
    { enableScripts: true, retainContextWhenHidden: true });
  const page = () => html(editorUrl(link, { theme: themeKind(vscode.window.activeColorTheme.kind), run: quarto ? 'external' : undefined }),
    crypto.randomBytes(16).toString('base64url'), { path: file });
  panel.webview.html = page();

  const attached = await api.post('/api/buffer/open', { text: doc.getText() });
  const sync = new Sync({
    buffer: () => doc.getText(),
    apply: (edits) => applyEdits(doc, edits),
    post: (b) => api.post('/api/buffer', b),
    source: () => api.get('/api/source'),
    conflict: (mine, theirs) => choose(doc, mine, theirs),
  });
  sync.open(doc.getText(), attached.body.version);

  const stop = new AbortController();
  api.events((ev, data) => { if (ev === 'buffer_edit') run(() => sync.bufferEdit(data)); }, stop.signal)
    .catch((e) => { if (!stop.signal.aborted) out.appendLine(`events: ${e}`); });

  let timer;
  let moved = null; // the cursor move we made for a reveal: not the person's, don't echo it
  const subs = [
    vscode.window.onDidChangeTextEditorSelection((e) => {
      if (e.textEditor.document !== doc) return;
      const sel = e.selections[0];
      const ours = moved?.isEqual(sel.active);
      moved = null;
      if (ours) return;
      panel.webview.postMessage(follow(sel, doc.getText(sel)));
    }),
    vscode.workspace.onDidChangeTextDocument((e) => {
      if (e.document !== doc || !e.contentChanges.length) return;
      clearTimeout(timer);
      timer = setTimeout(() => run(() => sync.changed(doc.getText())), 150);
    }),
    vscode.window.onDidChangeActiveColorTheme((t) => panel.webview.postMessage({ type: 'theme', kind: themeKind(t.kind) })),
    panel.webview.onDidReceiveMessage((m) => {
      const msg = fromEditor(m);
      if (msg?.type === 'reveal') reveal(msg.lines);
    }),
    vscode.workspace.onDidCloseTextDocument((d) => { if (d === doc) panel.dispose(); }),
  ];
  /** A panel selection or an agent's pointing: cursor to the source's start, shown, keyboard focus kept (FR-012). */
  const reveal = ([first, last]) => {
    const ed = vscode.window.visibleTextEditors.find((x) => x.document === doc);
    if (!ed) return;
    const at = doc.validatePosition(new vscode.Position(first - 1, 0));
    moved = at;
    ed.selection = new vscode.Selection(at, at);
    ed.revealRange(new vscode.Range(at, new vscode.Position(last - 1, 0)), vscode.TextEditorRevealType.InCenterIfOutsideViewport);
  };
  const entry = {
    panel, doc, api, run, sync,
    dispose: async () => {
      open.delete(file);
      clearTimeout(timer);
      subs.forEach((s) => s.dispose());
      stop.abort();
      await api.post('/api/buffer/close', {}).catch(() => {});
      srv.child.kill('SIGINT'); // qd serve shuts down cleanly (and detaches)
    },
  };
  open.set(file, entry);
  panel.onDidDispose(() => entry.dispose());
  return entry;
}

function activate(context) {
  out = vscode.window.createOutputChannel('quartz-deck');
  const active = () => {
    const doc = vscode.window.activeTextEditor?.document;
    if (!doc) throw new Error('Open a .qmd first');
    return doc;
  };
  const cmd = (name, fn) => context.subscriptions.push(vscode.commands.registerCommand(name, async () => {
    try { await fn(); } catch (e) { vscode.window.showErrorMessage(`quartz-deck: ${e.message}`); }
  }));
  cmd('quartz-deck.open', () => openDeck(active()));
  cmd('quartz-deck.focusPanel', () => open.get(active().uri.fsPath)?.panel.reveal(undefined, false));
  cmd('quartz-deck.focusText', async () => {
    const e = [...open.values()].find((x) => x.panel.active);
    if (e) await vscode.window.showTextDocument(e.doc, { preview: false });
  });
  context.subscriptions.push(out, vscode.window.registerWebviewPanelSerializer('quartz-deck', {
    async deserializeWebviewPanel(panel, state) {
      if (!state?.path) return panel.dispose();
      const doc = await vscode.workspace.openTextDocument(state.path);
      await openDeck(doc, panel).catch((e) => { panel.dispose(); out.appendLine(String(e)); });
    },
  }));
}

async function deactivate() {
  await Promise.all([...open.values()].map((e) => e.dispose()));
}

module.exports = { activate, deactivate };
