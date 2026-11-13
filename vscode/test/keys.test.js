// Spec 007 US5 (T021): in VS Code's panel no editor shortcut takes a key VS Code
// itself uses while the panel has focus (default-keys.json, from VS Code's reference).
const test = require('node:test');
const assert = require('node:assert/strict');

const actions = require('../../crates/qd/src/ui/actions.json').filter((a) => a.id);
const { keys: vscode } = require('../default-keys.json');

/** One spelling per key: modifiers sorted, letters upper-case. */
const norm = (k) => { const p = k.split('+'); const key = p.pop(); return [...p.sort(), key.length === 1 ? key.toUpperCase() : key].join('+'); };
const taken = new Map(Object.entries(vscode).map(([k, cmd]) => [norm(k), cmd]));
/** The keys an action has in the panel: its `vscode_key` (null = menu only), else its key. */
const panelKeys = (a) => [].concat(('vscode_key' in a ? a.vscode_key : a.key) ?? []);

test("no action's key in the panel is one of VS Code's", () => {
  const clashes = actions.flatMap((a) => panelKeys(a).filter((k) => taken.has(norm(k))).map((k) => `${a.id}: ${k} is VS Code's ${taken.get(norm(k))}`));
  assert.deepEqual(clashes, []);
});
