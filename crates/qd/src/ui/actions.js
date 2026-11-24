// The editor's actions (spec 003, contracts/ui.md): one table feeds the
// context menus, the element bar and the keyboard. Every action that changes
// the deck runs the same `qd` command an agent would.

import * as keys from '/ui/keys.js';
import * as settings from '/ui/settings.js';
import { ask, confirmSheet } from '/ui/sheet.js';
import * as dot from '/ui/dot.js';
import * as arrows from '/ui/arrows.js';

export const TRANSITIONS = ['none', 'fade', 'slide', 'convex', 'concave', 'zoom'];

/** Markdown for each insertable element type (FR-019); `edit` says how to open it. */
export const INSERTS = [
  { id: 'paragraph', label: 'Paragraph', md: 'New paragraph.', edit: 'typed' },
  { id: 'list', label: 'List', md: '- Item', edit: 'typed' },
  { id: 'equation', label: 'Equation', md: '$$\nx = 1\n$$', edit: 'source' },
  { id: 'image', label: 'Image…', image: true },
  { id: 'textbox', label: 'Text box', box: true },
  { id: 'diagram', label: 'Diagram', diagram: true }, // spec 015: `qd dot new`
  { id: 'arrow', label: 'Arrow', arrow: true }, // spec 016: drag on the slide, `qd arrow add`
  // spec 008 US7: the rest a talk needs, each its markdown
  { id: 'code', label: 'Code cell', md: '```{python}\n1 + 1\n```' },
  { id: 'callout', label: 'Callout', md: '::: {.callout-note}\nA note.\n:::' },
  { id: 'columns', label: 'Columns', md: ':::: {.columns}\n::: {.column width="50%"}\nLeft\n:::\n\n::: {.column width="50%"}\nRight\n:::\n::::' },
  { id: 'notes', label: 'Speaker notes', md: '::: notes\nSay this.\n:::' },
  { id: 'table', label: 'Table', md: '| A | B |\n|---|---|\n| 1 | 2 |' },
  { id: 'quote', label: 'Quote', md: '> A quote.' },
];

/**
 * `api` is the editor: op, runCells(refs), refresh, beginEditRef, beginSourceEl, beginSlideSource,
 * insert(slide, where, kind), copy(t), paste(t), copySlide(t), pasteSlides(t), say, slideUrl(anchor), deck().
 */
/**
 * Why slide `id` can't go into the group above (`in`) or out of its group (`out`):
 * the rules `qd indent` / `qd outdent` refuse by (spec 018 research R3), for the
 * menus' tooltips and the keys' messages. Null when it can.
 */
export function groupWhy(deck, id, dir) {
  const ss = deck?.slides || [];
  const i = ss.findIndex((s) => s.id === id);
  const s = ss[i];
  if (!s) return 'The title slide always comes first';
  if (deck.group_level == null) return 'With slide-level: 1 every slide stands alone';
  if (s.opaque) return 'qd can’t restructure this slide safely (an unterminated fence or div)';
  if (dir === 'out') return s.in ? null : s.head ? 'It already heads a group' : 'It isn’t in a group';
  if (s.in) return 'It is already in a group: groups are one level deep';
  if (i === 0) return 'There is nothing above it to go under';
  const p = ss[i - 1];
  return !s.head && !p.head && p.kind === 'implicit' ? 'The slide above has no title: give it one first' : null;
}

// what can't be placed freely, and why (spec 009 FR-012; `qd block place` says the same)
const UNPLACEABLE = {
  div: 'A div (columns, callout…) can’t be placed: place the elements inside it',
  notes: 'Speaker notes aren’t on the slide',
  pause: 'A pause has nothing to show',
};

/** What holds element `ref`: its slide, or inside columns its column (spec 024). */
export const up = (r) => r.slice(0, r.lastIndexOf('/'));
/** The slide element `ref` is on. */
export const slideOf = (r) => r.split('/')[0];

export function makeActions(api) {
  const classesOf = (t) => t.block.attrs?.classes || [];
  const toggleClass = (t, c) => api.op('block class', { element: t.ref, [classesOf(t).includes(c) ? 'remove' : 'add']: c });
  // in a column (spec 025), only what goes in the flow: boxes, arrows and diagrams are the slide's
  const insertMenu = (t, where) => INSERTS.filter((k) => !(where.into && (k.box || k.arrow || k.diagram)))
    .map((k) => ({ label: k.label, run: () => api.insert(t.slideId, where, k) }));

  // Appearance ▸: also the menu bar's Edit ▸ Appearance (spec 007)
  const classes = [
    { id: 'class-smaller', label: 'Smaller text', checked: (t) => classesOf(t).includes('smaller'), run: (t) => toggleClass(t, 'smaller') },
    { id: 'class-incremental', label: 'Incremental list', checked: (t) => classesOf(t).includes('incremental'), run: (t) => toggleClass(t, 'incremental') },
    { id: 'class-custom', label: 'Custom class…', run: async (t) => {
      const v = await ask({ title: 'Custom class', fields: [{ name: 'c', kind: 'text', label: 'Class to add or remove', placeholder: 'e.g. smaller', validate: (x) => (/^\.?[A-Za-z_][\w-]*$/.test(x) ? '' : 'Letters, digits, - and _ (no spaces)') }] });
      if (v) toggleClass(t, v.c.replace(/^\./, ''));
    } },
  ];
  const element = [
    { id: 'run', label: 'Run cell', visible: (t) => t.block.kind === 'cell', run: (t) => api.runCells([t.ref]) },
    { id: 'edit', label: 'Edit text', run: (t) => (arrows.is(t.ref) ? arrows.label(t.ref) : api.beginEditRef(t)) }, // an arrow: its label (016)
    { id: 'source', label: 'Edit markdown', run: (t) => api.beginSourceEl(t) },
    '-',
    { id: 'cut', label: 'Cut', run: async (t) => { await api.copy(t); return api.op('block rm', { element: t.ref }); } },
    { id: 'copy', label: 'Copy', run: (t) => api.copy(t) },
    { id: 'paste', label: 'Paste after', run: (t) => api.paste(t) },
    { id: 'dup', label: 'Duplicate', run: (t) => api.op('block dup', { element: t.ref }) },
    { id: 'delete', label: 'Delete', run: (t) => (dot.selectedIn(t.ref) ? dot.remove() : api.op('block rm', { element: t.ref })) },
    { id: 'history-restore-element', label: 'Restore from this version', visible: () => !!api.version(), run: (t) => api.op('restore', { rev: api.version().rev, element: t.ref }) },
    '-',
    // within what holds it: the slide, or its column (spec 025)
    { id: 'up', label: 'Move up', enabled: (t) => t.index > 0, run: (t) => api.op('block move', { element: t.ref, before: `${up(t.ref)}/${t.index - 1}` }) },
    { id: 'down', label: 'Move down', enabled: (t) => t.index < t.count - 1, run: (t) => api.op('block move', { element: t.ref, after: `${up(t.ref)}/${t.index + 1}` }) },
    { id: 'insert-before', label: 'Insert before', items: (t) => insertMenu(t, { before: t.ref }) },
    { id: 'insert-after', label: 'Insert after', items: (t) => insertMenu(t, { after: t.ref }) },
    '-',
    { id: 'fragment', label: 'Appears on click', checked: (t) => (arrows.is(t.ref) ? arrows.appears(t.ref) : classesOf(t).includes('fragment')), run: (t) => (arrows.is(t.ref) ? arrows.toggleAppears(t.ref) : toggleClass(t, 'fragment')) },
    { id: 'appearance', label: 'Appearance', items: (t) => resolve(classes, t) },
    '-',
    { id: 'place', label: 'Place freely', visible: (t) => t.block.kind !== 'box', why: (t) => UNPLACEABLE[t.block.kind], enabled: (t) => !UNPLACEABLE[t.block.kind], run: (t) => api.op('block place', api.placement(t)) },
    { id: 'unplace', label: 'Put back in the flow', visible: (t) => t.block.kind === 'box', run: (t) => api.op('block unplace', { element: t.ref }) },
    { id: 'dot-rm', label: 'Delete from diagram', title: 'The selected box with its arrows, or the selected arrow', visible: (t) => dot.selectedIn(t.ref), run: () => dot.remove() },
    { id: 'dot-flip', label: 'Reverse arrow', visible: (t) => dot.flippable(t.ref), run: () => dot.flip() },
    { id: 'dot-pin', label: 'Pin layout', title: 'Fix every box where it is now, so boxes can be placed by hand', visible: (t) => dot.pinnable(t.ref), run: (t) => dot.pin(t.ref) },
    // stacking (US5): boxes stack in source order, later on top; a fragment's click order follows it too
    ...[['front', 'Bring to front', 1], ['forward', 'Bring forward', 1], ['backward', 'Send backward', -1], ['back', 'Send to back', -1]].map(([to, label, d]) => ({
      id: `box-${to}`, label, title: 'Changes the order in the file, which is also the order fragments appear in',
      visible: (t) => t.block.kind === 'box',
      enabled: (t) => t.kinds.some((k, j) => k === 'box' && (d > 0 ? j > t.index : j < t.index)),
      run: (t) => api.op(`box ${to}`, { box: t.ref }),
    })),
  ];

  /** After an indent, outdent or a move that landed indented: say what else changed (spec 018 FR-015). */
  function regrouped(r, id) {
    if (!r?.changed) return r;
    const name = (x) => api.deck()?.slides.find((s) => s.id === x)?.title;
    const others = r.changed.filter((x) => x !== id);
    if (others.length) api.say(`${others.map((x) => name(x) || 'The slide above').join(', ')} now heads a group: it shows as a section slide`);
    else if (r.changed.includes(id) && !name(id)) api.say('It now heads a group under an empty heading: give it a title');
    return r;
  }

  /** "Copy group (4 slides)" on a folded head (spec 018 US3), else the slide's own label. */
  const group = (t, verb) => { const n = api.groupSize(t.slideId); return n ? `${verb} group (${n} slides)` : `${verb} slide`; };

  const kv = (t) => t.full?.attrs?.kv || {};
  const setAttr = (t, key, value) => api.op('set', value ? { slide: t.slideId, attr: [`${key}=${value}`] } : { slide: t.slideId, unset: [key] });
  const slide = [
    { id: 'slide-before', label: 'Add slide before…', run: (t) => api.addSlide({ before: t.slideId }) },
    { id: 'slide-after', label: 'Add slide after…', run: (t) => api.addSlide({ after: t.slideId }) },
    { id: 'slide-dup', label: (t) => group(t, 'Duplicate'), run: (t) => api.op('dup', { slide: t.slideId }) },
    { id: 'slide-cut', label: (t) => group(t, 'Cut'), run: async (t) => { await api.copySlide(t); return api.op('rm', { slide: t.slideId }); } },
    { id: 'slide-copy', label: (t) => group(t, 'Copy'), run: (t) => api.copySlide(t) },
    { id: 'slide-paste', label: 'Paste slides after', run: (t) => api.pasteSlides(t) },
    ...['in', 'out'].map((dir) => ({
      id: `slide-${dir}dent`, label: dir === 'in' ? 'Indent slide' : 'Outdent slide',
      enabled: (t) => !groupWhy(api.deck(), t.slideId, dir), why: (t) => groupWhy(api.deck(), t.slideId, dir),
      run: async (t) => regrouped(await api.op(`${dir}dent`, { slide: t.slideId }), t.slideId),
    })),
    { id: 'slide-hide', label: 'Hidden in the presentation', checked: (t) => kv(t).visibility === 'hidden', run: (t) => setAttr(t, 'visibility', kv(t).visibility === 'hidden' ? null : 'hidden') },
    { id: 'slide-rm', label: (t) => `${group(t, 'Delete')}…`, run: async (t) => {
      const n = api.groupSize(t.slideId);
      const what = n ? ['Delete group?', `"${t.title || t.slideId}" and the ${n - 1} slides in its group will be deleted. Undo brings them back.`] : ['Delete slide?', `"${t.title || t.slideId}" will be deleted. Undo brings it back.`];
      return (!settings.get('confirm.slide_delete') || await confirmSheet(...what, 'Delete')) && api.op('rm', { slide: t.slideId });
    } },
    { id: 'history-restore-slide', label: 'Restore slide from this version', visible: () => !!api.version(), run: (t) => api.op('restore', { rev: api.version().rev, slide: t.slideId }) },
    '-',
    { id: 'slide-bg', label: 'Background colour…', run: async (t) => {
      const v = await ask({ title: 'Background colour', fields: [{ name: 'c', kind: 'swatches', label: 'Colour', value: kv(t)['background-color'] || '' }] });
      if (v) setAttr(t, 'background-color', v.c);
    } },
    { id: 'slide-bgimg', label: 'Background image…', run: async (t) => {
      const v = await ask({ title: 'Background image', fields: [{ name: 'src', kind: 'files', label: 'Path or URL (empty removes it)', value: kv(t)['background-image'] || '' }] });
      if (v) setAttr(t, 'background-image', v.src);
    } },
    {
      id: 'slide-transition', label: 'Transition', items: (t) => [
        { label: 'Default', checked: !kv(t).transition, run: () => setAttr(t, 'transition', null) },
        ...TRANSITIONS.map((x) => ({ label: x[0].toUpperCase() + x.slice(1), checked: kv(t).transition === x, run: () => setAttr(t, 'transition', x) })),
      ],
    },
    '-',
    { id: 'slide-notes', label: 'Speaker notes…', run: api.editNotes },
    { id: 'slide-source', label: 'Edit slide source', run: (t) => api.beginSlideSource(t) },
    { id: 'layout-save', label: 'Save slide as layout…', run: api.saveLayout },
    { id: 'slide-link', label: 'Copy link to slide', run: (t) => navigator.clipboard?.writeText(api.slideUrl(t.anchor)).then(() => api.say('Link copied')) },
  ];
  const emptyArea = [...slide, '-', { id: 'insert-end', label: 'Insert', items: (t) => insertMenu(t, { end: true }) }];

  const tip = (a, t) => a?.why?.(t) ?? a?.title;
  /** Menu items for a target, with functions resolved against it. */
  const shown = (a, t) => a === '-' || !a.visible || a.visible(t);
  const resolve = (list, t) => list.filter((a) => shown(a, t)).map((a) => (a === '-' ? a : {
    id: a.id,
    label: typeof a.label === 'function' ? a.label(t) : a.label,
    shortcut: keys.shortcut(a.id), // keys come from the action list (spec 007)
    checked: typeof a.checked === 'function' ? a.checked(t) : undefined,
    disabled: a.enabled ? !a.enabled(t) : false,
    title: tip(a, t),
    items: a.items ? a.items(t) : undefined,
    run: a.run && (() => a.run(t)),
  }));

  const tables = { element: [...element, ...classes], slide };
  const find = (id, table) => tables[table].find((x) => x.id === id);
  const can = (id, t, table = 'element') => { const a = find(id, table); return !!a && shown(a, t) && (!a.enabled || a.enabled(t)); };
  return {
    element: (t) => resolve(element, t),
    slide: (t) => resolve(slide, t),
    empty: (t) => resolve(emptyArea, t),
    insert: insertMenu,
    setAttr,
    op: api.op, // an op, with the editor's redraw after it
    /** Run action `id` of a table on `t` (keyboard shortcuts, bars, the menu bar). */
    run: (id, t, table = 'element') => (can(id, t, table) ? find(id, table).run(t) : null),
    /** Whether action `id` applies to `t` now, and its checkmark (the menu bar). */
    can,
    checked: (id, t, table = 'element') => find(id, table)?.checked?.(t),
    /** Why action `id` can't act on `t` (a disabled item's tooltip). */
    why: (id, t, table = 'element') => find(id, table)?.why?.(t),
    /** Its tooltip: why it can't act on `t`, else what to know about it. */
    tip: (id, t, table = 'element') => tip(find(id, table), t),
    regrouped,
  };
}
