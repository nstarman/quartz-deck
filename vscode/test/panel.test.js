// Spec 006 (T009): the panel's page, theme mapping and message filter.
const test = require('node:test');
const assert = require('node:assert/strict');
const { editorUrl, themeKind, html, fromEditor, follow } = require('../panel.js');

const LINK = 'http://127.0.0.1:5123/#t=abc123';

test('the editor link gains the framed-mode parameters, token kept in the fragment', () => {
  assert.equal(editorUrl(LINK, { theme: 'dark' }), 'http://127.0.0.1:5123/?host=vscode&theme=dark#t=abc123');
  assert.equal(
    editorUrl('http://localhost:9000/#t=abc123', { theme: 'light', run: 'external' }),
    'http://localhost:9000/?host=vscode&theme=light&run=external#t=abc123',
  );
});

test("VS Code's theme kinds map to the editor's", () => {
  assert.deepEqual([1, 2, 3, 4].map(themeKind), ['light', 'dark', 'hc', 'hc']);
});

test('the page is one full-size iframe of the editor, and its CSP allows only that origin', () => {
  const page = html(editorUrl(LINK, { theme: 'dark' }), 'n0nce');
  assert.match(page, /<iframe [^>]*src="http:\/\/127\.0\.0\.1:5123\/\?host=vscode&amp;theme=dark#t=abc123"/);
  assert.match(page, /frame-src http:\/\/127\.0\.0\.1:5123;/);
  assert.match(page, /default-src 'none'/);
  assert.match(page, /<script nonce="n0nce">/);
});

test('only a well-formed reveal comes through from the editor', () => {
  assert.deepEqual(fromEditor({ type: 'reveal', lines: [3, 5] }), { type: 'reveal', lines: [3, 5] });
  for (const bad of [{ type: 'ready' }, { type: 'run' }, { type: 'reveal', lines: ['x', 1] }, { type: 'reveal' }, null, 'reveal', { type: 'op', op: 'rm' }]) {
    assert.equal(fromEditor(bad), null, JSON.stringify(bad));
  }
});

test("VS Code's selection becomes a follow message: 1-based lines, text bounded", () => {
  const sel = { start: { line: 4 }, end: { line: 6 }, active: { line: 6 } };
  assert.deepEqual(follow(sel, 'abc'), { type: 'follow', line: 7, first: 5, last: 7, text: 'abc' });
  assert.equal(follow(sel, 'x'.repeat(10000)).text.length, 2001, 'qd still sees it was cut');
});
