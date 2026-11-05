// Spec 006 (T018): the palette commands — which qd command, and what the person is told.
const test = require('node:test');
const assert = require('node:assert/strict');
const { ARGS, report } = require('../commands.js');

test('each palette command runs the matching qd command', () => {
  assert.deepEqual(ARGS, {
    render: ['render'],
    export: ['export'],
    exportPdf: ['export', '--pdf'],
    checkOffline: ['check', '--offline'],
  });
});

test("qd's JSON becomes a one-line result", () => {
  assert.deepEqual(report('render', '{"ok":true,"op":"render","output":"/d/talk.html"}', ''),
    { ok: true, text: 'Rendered: /d/talk.html' });
  assert.deepEqual(report('export', '{"kind":"export","ok":true,"html":"/d/export/talk.html","check":{"slides":8}}', ''),
    { ok: true, text: 'Exported: /d/export/talk.html (8 slides, works offline)' });
  assert.deepEqual(report('exportPdf', '{"kind":"export","ok":true,"html":"/d/e/t.html","check":{"slides":2},"pdf":{"path":"/d/e/t.pdf","pages":3}}', ''),
    { ok: true, text: 'Exported: /d/e/t.html (2 slides, works offline); PDF /d/e/t.pdf, 3 pages' });
  assert.deepEqual(report('checkOffline', '{"kind":"offline_check","ok":true,"problems":[],"slides":5}', ''),
    { ok: true, text: 'Works offline: 5 slides' });
});

test('failures say what went wrong, from the problems or the error', () => {
  const problems = '{"kind":"offline_check","ok":false,"problems":[{"message":"tried to load https://x/y.js","hint":"export it"},{"message":"b"}],"slides":2}';
  const err = '{"error":{"code":"E_OFFLINE","message":"the deck does not work offline (2 problem(s))","hint":"see the problems"}}';
  assert.deepEqual(report('checkOffline', problems, err),
    { ok: false, text: '2 offline problems: tried to load https://x/y.js — export it (and 1 more)' });
  assert.deepEqual(report('render', '', '{"error":{"code":"E_RENDER","message":"quarto render failed","hint":"see details for the Quarto log"}}'),
    { ok: false, text: 'quarto render failed — see details for the Quarto log' });
  assert.deepEqual(report('render', '', 'Segmentation fault'), { ok: false, text: 'Segmentation fault' });
  assert.deepEqual(report('render', '', ''), { ok: false, text: 'qd render failed' });
});
