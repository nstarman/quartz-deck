// The editor's actions (spec 003, contracts/ui.md): one table feeds the
// context menus, the element bar and the keyboard. Every action that changes
// the deck runs the same `qd` command an agent would.

const TRANSITIONS = ['none', 'fade', 'slide', 'convex', 'concave', 'zoom'];

/** Markdown for each insertable element type (FR-019); `edit` says how to open it. */
export const INSERTS = [
  { label: 'Paragraph', md: 'New paragraph.', edit: 'typed' },
  { label: 'List', md: '- Item', edit: 'typed' },
  { label: 'Equation', md: '$$\nx = 1\n$$', edit: 'source' },
  { label: 'Image…', image: true },
  { label: 'Text box', box: true },
];

/**
 * `api` is the editor: op, refresh, beginEditRef, beginSourceEl, beginSlideSource,
 * insert(slide, where, kind), copy(t), paste(t), say, slideUrl(anchor).
 */
export function makeActions(api) {
  const classesOf = (t) => t.block.attrs?.classes || [];
  const toggleClass = (t, c) => api.op('block class', { element: t.ref, [classesOf(t).includes(c) ? 'remove' : 'add']: c });
  const insertMenu = (t, where) => INSERTS.map((k) => ({ label: k.label, run: () => api.insert(t.slideId, where, k) }));

  const element = [
    { id: 'edit', label: 'Edit text', shortcut: 'Enter', run: (t) => api.beginEditRef(t) },
    { id: 'source', label: 'Edit source', shortcut: '⌥Enter', run: (t) => api.beginSourceEl(t) },
    '-',
    { id: 'cut', label: 'Cut', shortcut: '⌘X', run: async (t) => { await api.copy(t); return api.op('block rm', { element: t.ref }); } },
    { id: 'copy', label: 'Copy', shortcut: '⌘C', run: (t) => api.copy(t) },
    { id: 'paste', label: 'Paste after', shortcut: '⌘V', run: (t) => api.paste(t) },
    { id: 'dup', label: 'Duplicate', shortcut: '⌘D', run: (t) => api.op('block dup', { element: t.ref }) },
    { id: 'delete', label: 'Delete', shortcut: '⌫', run: (t) => api.op('block rm', { element: t.ref }) },
    '-',
    { id: 'up', label: 'Move up', shortcut: '⌥↑', enabled: (t) => t.index > 0, run: (t) => api.op('block move', { element: t.ref, before: `${t.slideId}/${t.index - 1}` }) },
    { id: 'down', label: 'Move down', shortcut: '⌥↓', enabled: (t) => t.index < t.count - 1, run: (t) => api.op('block move', { element: t.ref, after: `${t.slideId}/${t.index + 1}` }) },
    { id: 'insert-before', label: 'Insert before', items: (t) => insertMenu(t, { before: t.ref }) },
    { id: 'insert-after', label: 'Insert after', items: (t) => insertMenu(t, { after: t.ref }) },
    '-',
    { id: 'fragment', label: 'Appears on click', checked: (t) => classesOf(t).includes('fragment'), run: (t) => toggleClass(t, 'fragment') },
    {
      id: 'appearance', label: 'Appearance', items: (t) => [
        { label: 'Smaller text', checked: classesOf(t).includes('smaller'), run: () => toggleClass(t, 'smaller') },
        { label: 'Incremental list', checked: classesOf(t).includes('incremental'), run: () => toggleClass(t, 'incremental') },
        { label: 'Custom class…', run: () => { const c = prompt('Class to add or remove (without the dot)'); if (c?.trim()) toggleClass(t, c.trim().replace(/^\./, '')); } },
      ],
    },
  ];

  const kv = (t) => t.full?.attrs?.kv || {};
  const setAttr = (t, key, value) => api.op('set', value ? { slide: t.slideId, attr: [`${key}=${value}`] } : { slide: t.slideId, unset: [key] });
  const slide = [
    { id: 'slide-before', label: 'Add slide before', run: (t) => api.op('add', { title: 'New slide', before: t.slideId }) },
    { id: 'slide-after', label: 'Add slide after', shortcut: '⇧⌘N', run: (t) => api.op('add', { title: 'New slide', after: t.slideId }) },
    { id: 'slide-dup', label: 'Duplicate slide', shortcut: '⇧⌘D', run: (t) => api.op('dup', { slide: t.slideId }) },
    { id: 'slide-hide', label: 'Hidden in the presentation', checked: (t) => kv(t).visibility === 'hidden', run: (t) => setAttr(t, 'visibility', kv(t).visibility === 'hidden' ? null : 'hidden') },
    { id: 'slide-rm', label: 'Delete slide…', run: (t) => confirm(`Delete slide "${t.title || t.slideId}"? (Undo can restore it.)`) && api.op('rm', { slide: t.slideId }) },
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
  const resolve = (list, t) => list.map((a) => (a === '-' ? a : {
    id: a.id,
    label: a.label,
    shortcut: a.shortcut,
    checked: typeof a.checked === 'function' ? a.checked(t) : undefined,
    disabled: a.enabled ? !a.enabled(t) : false,
    items: a.items ? a.items(t) : undefined,
    run: a.run && (() => a.run(t)),
  }));

  const tables = { element, slide };
  return {
    element: (t) => resolve(element, t),
    slide: (t) => resolve(slide, t),
    empty: (t) => resolve(emptyArea, t),
    insert: insertMenu,
    setAttr,
    /** Run action `id` of a table on `t` (keyboard shortcuts, bars). */
    run: (id, t, table = 'element') => {
      const a = tables[table].find((x) => x.id === id);
      return a && (!a.enabled || a.enabled(t)) ? a.run(t) : null;
    },
  };
}
