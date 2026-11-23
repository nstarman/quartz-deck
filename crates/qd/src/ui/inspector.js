// The properties column (specs 019, 023): beside the slide whenever the person
// keeps it (View ▸ Properties), with its views (Format, Animate) above its tabs,
// the tabs following the selection. Each control is the one it was: only where
// it sits changes.

import { get as setting } from '/ui/settings.js';
import { el } from '/ui/run.js';

const NAMES = { slide: 'Slide', title: 'Title', element: 'Look', text: 'Text', position: 'Arrange' };
const VIEWS = { format: 'Format', animate: 'Animate' };

const main = document.querySelector('main');
const panel = document.querySelector('#panel');
const views = panel.querySelector('.views');
const tabs = panel.querySelector('.tabs');
const fmtbar = document.querySelector('#fmtbar');
let what = () => ({}); // the editor's selection: { kind, typeable, title }
let tab = null;
let elementTab = 'element'; // the one last chosen for an element, kept for the next

export const shown = () => main.classList.contains('inspector');

/** Show it when the person keeps it. */
export function fit() {
  const on = !!setting('ui.inspector');
  if (on === shown()) return;
  main.classList.toggle('inspector', on);
  update();
}

/** The tabs for what is selected now, and the one to show (data-model "Which tabs show"). */
function update() {
  const { kind, typeable, title } = what();
  const typing = !fmtbar.hidden;
  // (typing on the slide, nothing is selected: the text's formatting is what's there)
  const have = kind ? ['element', typeable && 'text', 'position'].filter(Boolean) : typing ? ['text'] : [title ? 'title' : 'slide'];
  tab = typing && have.includes('text') ? 'text' : kind ? (have.includes(elementTab) ? elementTab : 'element') : have[0];
  tabs.replaceChildren(...have.map((name) => tabButton(name, NAMES[name], () => {
    if (kind) elementTab = name;
    tab = name;
    show();
  })));
  show();
}

const tabButton = (name, label, click) => {
  const b = el('button', { type: 'button', textContent: label, id: `tab-${name}`, role: 'tab' });
  b.addEventListener('click', click);
  return b;
};

/** A view of the column (Format, Animate): the panel's data-view, which the CSS shows. */
function view(name) {
  panel.dataset.view = name;
  for (const b of views.children) {
    const on = b.dataset.view === name;
    b.ariaSelected = String(on);
    b.tabIndex = on ? 0 : -1;
  }
}

function show() {
  for (const b of tabs.children) {
    const on = b.id === `tab-${tab}`;
    b.ariaSelected = String(on);
    b.tabIndex = on ? 0 : -1; // Tab reaches the tab shown; the arrows the others
  }
  for (const x of panel.querySelectorAll(':is(.bars, .body) > [data-tab]')) x.classList.toggle('off', x.dataset.tab !== tab);
}

let toSlide = () => {};

/** F6: from the slide to the tab shown, and from the column back to the slide. */
export function toggleFocus() {
  if (panel.contains(document.activeElement)) toSlide();
  else (panel.dataset.view === 'format' && tabs.querySelector('[aria-selected=true]') || views.querySelector('[aria-selected=true]'))?.focus();
}

export function init(get, slide) {
  what = get;
  toSlide = slide;
  views.replaceChildren(...Object.entries(VIEWS).map(([name, label]) => {
    const b = tabButton(`view-${name}`, label, () => view(name));
    b.dataset.view = name;
    return b;
  }));
  view('format');
  // each tablist's own keys: ← → move along its tabs (WAI-ARIA tabs)
  for (const list of [views, tabs]) list.addEventListener('keydown', (e) => {
    const d = { ArrowRight: 1, ArrowLeft: -1 }[e.key];
    if (!d) return;
    e.preventDefault();
    const all = [...list.children];
    const next = all[(all.indexOf(document.activeElement) + d + all.length) % all.length];
    next.click();
    next.focus();
  });
  // Esc in a field: back to the slide, the selection kept
  panel.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape' || !e.target.matches('input, textarea, select')) return;
    e.preventDefault();
    e.stopPropagation();
    toSlide();
  });
  // the panel redrawn (a new selection), typing begun or ended
  new MutationObserver(update).observe(panel.querySelector('.body'), { childList: true });
  new MutationObserver(update).observe(fmtbar, { attributes: true, attributeFilter: ['hidden'] });
  fit();
}
