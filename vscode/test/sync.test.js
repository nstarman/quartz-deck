// Spec 006 (T007): the buffer ⇄ qd sync logic, against fakes.
const test = require('node:test');
const assert = require('node:assert/strict');
const { applyEdits, lineEdits, merge, Sync } = require('../sync.js');

const BASE = 'a\nb\nc\nd\ne\n';

test('applyEdits replaces line ranges of the base', () => {
  const edits = [{ start: 1, end: 2, text: 'B\n' }, { start: 4, end: 4, text: 'x\n' }];
  assert.equal(applyEdits(BASE, edits), 'a\nB\nc\nd\nx\ne\n');
  assert.equal(applyEdits(BASE, lineEdits(BASE, 'a\nc\nZ\n')), 'a\nc\nZ\n');
  assert.equal(applyEdits('a', lineEdits('a', 'a\nb')), 'a\nb', 'no trailing newline');
});

test('merge keeps both sides apart, reports the same lines as a conflict', () => {
  assert.deepEqual(merge(BASE, 'a\nB\nc\nd\ne\n', 'a\nb\nc\nd\nE\n'), { text: 'a\nB\nc\nd\nE\n' });
  assert.deepEqual(merge(BASE, BASE, 'a\nb\nC\nd\ne\n'), { text: 'a\nb\nC\nd\ne\n' });
  const c = merge(BASE, 'a\nb\nmine\nd\ne\n', 'a\nb\ntheirs\nd\ne\n');
  assert.equal(c.conflict, true);
});

/** A fake buffer + server around a Sync. */
function rig() {
  const ver = (t) => 'v:' + t; // distinct per text, like qd's hash
  const log = [];
  const w = { buffer: BASE, server: { text: BASE, version: ver(BASE) }, choice: 'theirs', log, ver };
  w.qd = (t) => (w.server = { text: t, version: ver(t) }); // qd's side moved on to `t`
  w.sync = new Sync({
    buffer: () => w.buffer,
    apply: async (edits) => { w.buffer = applyEdits(w.buffer, edits); log.push('apply'); w.sync.changed(w.buffer); },
    post: async ({ base, text }) => {
      log.push('post');
      if (base !== w.server.version) return { status: 409, body: { version: w.server.version, text: w.server.text } };
      w.server = { text, version: ver(text) };
      return { status: 200, body: { version: w.server.version } };
    },
    source: async () => ({ ...w.server }),
    conflict: async () => { log.push('conflict'); return w.choice; },
  });
  w.sync.open(BASE, ver(BASE));
  return w;
}

test('typing is posted against the version VS Code last saw', async () => {
  const w = rig();
  w.buffer = 'a\nB\nc\nd\ne\n';
  await w.sync.changed(w.buffer);
  assert.equal(w.server.text, w.buffer);
  assert.deepEqual(w.log, ['post']);
});

test("a qd edit is applied once and its echo isn't posted back", async () => {
  const w = rig();
  const next = 'a\nb\nc\nD\ne\n';
  w.qd(next);
  await w.sync.bufferEdit({ base: w.ver(BASE), version: w.ver(next), edits: lineEdits(BASE, next) });
  assert.equal(w.buffer, next);
  assert.deepEqual(w.log, ['apply'], 'no post: the buffer already matches qd');
});

test('typing that crossed a qd edit is merged, then posted', async () => {
  const w = rig();
  const theirs = 'a\nb\nc\nd\nE\n';
  w.qd(theirs);
  w.buffer = 'a\nB\nc\nd\ne\n';
  await w.sync.changed(w.buffer);
  assert.equal(w.buffer, 'a\nB\nc\nd\nE\n', 'both changes in the buffer');
  assert.equal(w.server.text, 'a\nB\nc\nd\nE\n');
  assert.ok(!w.log.includes('conflict'));
});

test('the same lines changed on both sides: the person chooses', async () => {
  for (const [choice, want] of [['mine', 'a\nb\nmine\nd\ne\n'], ['theirs', 'a\nb\ntheirs\nd\ne\n']]) {
    const w = rig();
    w.choice = choice;
    const theirs = 'a\nb\ntheirs\nd\ne\n';
    w.qd(theirs);
    w.buffer = 'a\nb\nmine\nd\ne\n';
    await w.sync.changed(w.buffer);
    assert.ok(w.log.includes('conflict'));
    assert.equal(w.buffer, want, choice);
    assert.equal(w.server.text, want, choice);
  }
});

test('a qd edit while the person has unsent typing is merged, not misapplied', async () => {
  const w = rig();
  w.buffer = 'a\nB\nc\nd\ne\n'; // typed, not yet posted
  const theirs = 'a\nb\nc\nd\nE\n';
  w.qd(theirs);
  await w.sync.bufferEdit({ base: w.ver(BASE), version: w.ver(theirs), edits: lineEdits(BASE, theirs) });
  assert.equal(w.buffer, 'a\nB\nc\nd\nE\n');
  assert.equal(w.server.text, 'a\nB\nc\nd\nE\n', 'the typing reaches qd too');
});

test('an edit against an unknown base resyncs from the full text', async () => {
  const w = rig();
  const theirs = 'z\nb\nc\nd\ne\n';
  w.qd(theirs);
  await w.sync.bufferEdit({ base: 'elsewhere', version: w.ver(theirs), edits: [] });
  assert.equal(w.buffer, theirs);
});
