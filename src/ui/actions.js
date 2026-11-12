// The editor's actions (spec 003, contracts/ui.md): one table feeds the
// context menus, the element bar and the keyboard. Every action that changes
// the deck runs the same `qd` command an agent would.

import * as keys from '/ui/keys.js';
import * as settings from '/ui/settings.js';
import { ask, confirmSheet } from '/ui/sheet.js';

export const TRANSITIONS = ['none', 'fade', 'slide', 'convex', 'concave', 'zoom'];

/** Markdown for each insertable element type (FR-019); `edit` says how to open it. */
export const INSERTS = [
  { id: 'paragraph', label: 'Paragraph', md: 'New paragraph.', edit: 'typed' },
  { id: 'list', label: 'List', md: '- Item', edit: 'typed' },
  { id: 'equation', label: 'Equation', md: '$$\nx = 1\n$$', edit: 'source' },
  { id: 'image', label: 'Image…', image: true },
  { id: 'textbox', label: 'Text box', box: true },
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
 * insert(slide, where, kind), copy(t), paste(t), copySlide(t), pasteSlides(t), say, slideUrl(anchor).
 */
// what can't be placed freely, and why (spec 009 FR-012; `qd block place` says the same)
const UNPLACEABLE = {
  div: 'A div (columns, callout…) can’t be placed: place the elements inside it',
  notes: 'Speaker notes aren’t on the slide',
  pause: 'A pause has nothing to show',
};

export function makeActions(api) {
  const classesOf = (t) => t.block.attrs?.classes || [];
  const toggleClass = (t, c) => api.op('block class', { element: t.ref, [classesOf(t).includes(c) ? 'remove' : 'add']: c });
  const insertMenu = (t, where) => INSERTS.map((k) => ({ label: k.label, run: () => api.insert(t.slideId, where, k) }));

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
    { id: 'edit', label: 'Edit text', run: (t) => api.beginEditRef(t) },
    { id: 'source', label: 'Edit markdown', run: (t) => api.beginSourceEl(t) },
    '-',
    { id: 'cut', label: 'Cut', run: async (t) => { await api.copy(t); return api.op('block rm', { element: t.ref }); } },
    { id: 'copy', label: 'Copy', run: (t) => api.copy(t) },
    { id: 'paste', label: 'Paste after', run: (t) => api.paste(t) },
    { id: 'dup', label: 'Duplicate', run: (t) => api.op('block dup', { element: t.ref }) },
    { id: 'delete', label: 'Delete', run: (t) => api.op('block rm', { element: t.ref }) },
    '-',
    { id: 'up', label: 'Move up', enabled: (t) => t.index > 0, run: (t) => api.op('block move', { element: t.ref, before: `${t.slideId}/${t.index - 1}` }) },
    { id: 'down', label: 'Move down', enabled: (t) => t.index < t.count - 1, run: (t) => api.op('block move', { element: t.ref, after: `${t.slideId}/${t.index + 1}` }) },
    { id: 'insert-before', label: 'Insert before', items: (t) => insertMenu(t, { before: t.ref }) },
    { id: 'insert-after', label: 'Insert after', items: (t) => insertMenu(t, { after: t.ref }) },
    '-',
    { id: 'fragment', label: 'Appears on click', checked: (t) => classesOf(t).includes('fragment'), run: (t) => toggleClass(t, 'fragment') },
    { id: 'appearance', label: 'Appearance', items: (t) => resolve(classes, t) },
    '-',
    { id: 'place', label: 'Place freely', visible: (t) => t.block.kind !== 'box', why: (t) => UNPLACEABLE[t.block.kind], enabled: (t) => !UNPLACEABLE[t.block.kind], run: (t) => api.op('block place', api.placement(t)) },
    { id: 'unplace', label: 'Put back in the flow', visible: (t) => t.block.kind === 'box', run: (t) => api.op('block unplace', { element: t.ref }) },
  ];

  const kv = (t) => t.full?.attrs?.kv || {};
  const setAttr = (t, key, value) => api.op('set', value ? { slide: t.slideId, attr: [`${key}=${value}`] } : { slide: t.slideId, unset: [key] });
  const slide = [
    { id: 'slide-before', label: 'Add slide before…', run: (t) => api.addSlide({ before: t.slideId }) },
    { id: 'slide-after', label: 'Add slide after…', run: (t) => api.addSlide({ after: t.slideId }) },
    { id: 'slide-dup', label: 'Duplicate slide', run: (t) => api.op('dup', { slide: t.slideId }) },
    { id: 'slide-cut', label: 'Cut slide', run: async (t) => { await api.copySlide(t); return api.op('rm', { slide: t.slideId }); } },
    { id: 'slide-copy', label: 'Copy slide', run: (t) => api.copySlide(t) },
    { id: 'slide-paste', label: 'Paste slides after', run: (t) => api.pasteSlides(t) },
    { id: 'slide-hide', label: 'Hidden in the presentation', checked: (t) => kv(t).visibility === 'hidden', run: (t) => setAttr(t, 'visibility', kv(t).visibility === 'hidden' ? null : 'hidden') },
    { id: 'slide-rm', label: 'Delete slide…', run: async (t) => (!settings.get('confirm.slide_delete') || await confirmSheet('Delete slide?', `"${t.title || t.slideId}" will be deleted. Undo brings it back.`, 'Delete')) && api.op('rm', { slide: t.slideId }) },
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
    { id: 'layout-save', label: 'Save slide as layout…', run: (t) => api.saveLayout(t) },
    { id: 'slide-link', label: 'Copy link to slide', run: (t) => navigator.clipboard?.writeText(api.slideUrl(t.anchor)).then(() => api.say('Link copied')) },
  ];
  const emptyArea = [...slide, '-', { id: 'insert-end', label: 'Insert', items: (t) => insertMenu(t, { end: true }) }];

  /** Menu items for a target, with functions resolved against it. */
  const shown = (a, t) => a === '-' || !a.visible || a.visible(t);
  const resolve = (list, t) => list.filter((a) => shown(a, t)).map((a) => (a === '-' ? a : {
    id: a.id,
    label: a.label,
    shortcut: keys.shortcut(a.id), // keys come from the action list (spec 007)
    checked: typeof a.checked === 'function' ? a.checked(t) : undefined,
    disabled: a.enabled ? !a.enabled(t) : false,
    title: a.why?.(t),
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
    /** Run action `id` of a table on `t` (keyboard shortcuts, bars, the menu bar). */
    run: (id, t, table = 'element') => (can(id, t, table) ? find(id, table).run(t) : null),
    /** Whether action `id` applies to `t` now, and its checkmark (the menu bar). */
    can,
    checked: (id, t, table = 'element') => find(id, table)?.checked?.(t),
    /** Why action `id` can't act on `t` (a disabled item's tooltip). */
    why: (id, t, table = 'element') => find(id, table)?.why?.(t),
  };
}
