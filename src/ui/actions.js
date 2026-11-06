// The editor's actions (spec 003, contracts/ui.md): one table feeds the
// context menus, the element bar and the keyboard. Every action that changes
// the deck runs the same `qd` command an agent would.

import * as keys from '/ui/keys.js';
import * as settings from '/ui/settings.js';

export const TRANSITIONS = ['none', 'fade', 'slide', 'convex', 'concave', 'zoom'];

/** Markdown for each insertable element type (FR-019); `edit` says how to open it. */
export const INSERTS = [
  { id: 'paragraph', label: 'Paragraph', md: 'New paragraph.', edit: 'typed' },
  { id: 'list', label: 'List', md: '- Item', edit: 'typed' },
  { id: 'equation', label: 'Equation', md: '$$\nx = 1\n$$', edit: 'source' },
  { id: 'image', label: 'Image…', image: true },
  { id: 'textbox', label: 'Text box', box: true },
];

/**
 * `api` is the editor: op, runCells(refs), refresh, beginEditRef, beginSourceEl, beginSlideSource,
 * insert(slide, where, kind), copy(t), paste(t), copySlide(t), pasteSlides(t), say, slideUrl(anchor).
 */
export function makeActions(api) {
  const classesOf = (t) => t.block.attrs?.classes || [];
  const toggleClass = (t, c) => api.op('block class', { element: t.ref, [classesOf(t).includes(c) ? 'remove' : 'add']: c });
  const insertMenu = (t, where) => INSERTS.map((k) => ({ label: k.label, run: () => api.insert(t.slideId, where, k) }));

  // Appearance ▸: also the menu bar's Edit ▸ Appearance (spec 007)
  const classes = [
    { id: 'class-smaller', label: 'Smaller text', checked: (t) => classesOf(t).includes('smaller'), run: (t) => toggleClass(t, 'smaller') },
    { id: 'class-incremental', label: 'Incremental list', checked: (t) => classesOf(t).includes('incremental'), run: (t) => toggleClass(t, 'incremental') },
    { id: 'class-custom', label: 'Custom class…', run: (t) => { const c = prompt('Class to add or remove (without the dot)'); if (c?.trim()) toggleClass(t, c.trim().replace(/^\./, '')); } },
  ];
  const element = [
    { id: 'run', label: 'Run cell', visible: (t) => t.block.kind === 'cell', run: (t) => api.runCells([t.ref]) },
    { id: 'edit', label: 'Edit text', run: (t) => api.beginEditRef(t) },
    { id: 'source', label: 'Edit source', run: (t) => api.beginSourceEl(t) },
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
  ];

  const kv = (t) => t.full?.attrs?.kv || {};
  const setAttr = (t, key, value) => api.op('set', value ? { slide: t.slideId, attr: [`${key}=${value}`] } : { slide: t.slideId, unset: [key] });
  const slide = [
    { id: 'slide-before', label: 'Add slide before', run: (t) => api.op('add', { before: t.slideId }) },
    { id: 'slide-after', label: 'Add slide after', run: (t) => api.op('add', { after: t.slideId }) },
    { id: 'slide-dup', label: 'Duplicate slide', run: (t) => api.op('dup', { slide: t.slideId }) },
    { id: 'slide-cut', label: 'Cut slide', run: (t) => (api.copySlide(t), api.op('rm', { slide: t.slideId })) },
    { id: 'slide-copy', label: 'Copy slide', run: (t) => api.copySlide(t) },
    { id: 'slide-paste', label: 'Paste slides after', run: (t) => api.pasteSlides(t) },
    { id: 'slide-hide', label: 'Hidden in the presentation', checked: (t) => kv(t).visibility === 'hidden', run: (t) => setAttr(t, 'visibility', kv(t).visibility === 'hidden' ? null : 'hidden') },
    { id: 'slide-rm', label: 'Delete slide…', run: (t) => (!settings.get('confirm.slide_delete') || confirm(`Delete slide "${t.title || t.slideId}"? (Undo can restore it.)`)) && api.op('rm', { slide: t.slideId }) },
    '-',
    { id: 'slide-bg', label: 'Background colour…', run: (t) => { const v = prompt('Background colour (e.g. #fdf6e3; empty removes it)', kv(t)['background-color'] || ''); if (v !== null) setAttr(t, 'background-color', v.trim()); } },
    { id: 'slide-bgimg', label: 'Background image…', run: (t) => { const v = prompt('Background image path or URL (empty removes it)', kv(t)['background-image'] || ''); if (v !== null) setAttr(t, 'background-image', v.trim()); } },
    {
      id: 'slide-transition', label: 'Transition', items: (t) => [
        { label: 'Default', checked: !kv(t).transition, run: () => setAttr(t, 'transition', null) },
        ...TRANSITIONS.map((x) => ({ label: x[0].toUpperCase() + x.slice(1), checked: kv(t).transition === x, run: () => setAttr(t, 'transition', x) })),
      ],
    },
    '-',
    { id: 'slide-notes', label: 'Speaker notes…', run: api.editNotes },
    { id: 'slide-source', label: 'Edit slide source', run: (t) => api.beginSlideSource(t) },
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
  };
}
