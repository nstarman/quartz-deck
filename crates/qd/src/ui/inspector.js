// The inspector (spec 019): on a window wide enough, the Properties panel
// stays beside the slide, its tabs following the selection, and the slide,
// element and formatting bars move into it instead of floating (research
// R2). Each control is the one it was: only where it sits changes.

import { get as setting } from '/ui/settings.js';
import { el } from '/ui/run.js';

const WIDE = 1200; // px of window it needs (research R3: tune on real screens)
const SLIDE_MIN = 640; // px the slide keeps beside it
const WIDTH = 320; // its own, as the panel's
const NAMES = { slide: 'Slide', title: 'Title', element: 'Element', text: 'Text', position: 'Position' };

const main = document.querySelector('main');
const panel = document.querySelector('#panel');
const tabs = panel.querySelector('.tabs');
const bars = panel.querySelector('.bars');
const stage = document.querySelector('#stage');
const fmtbar = document.querySelector('#fmtbar');
let what = () => ({}); // the editor's selection: { kind, typeable, title }
let tab = null;
let elementTab = 'element'; // the one last chosen for an element, kept for the next

export const shown = () => main.classList.contains('inspector');

/** It would fit: the window is wide, and the slide keeps its room beside it. */
export function fits() {
  const besides = shown() || main.classList.contains('panel-open') ? 0 : WIDTH;
  return innerWidth >= WIDE && stage.getBoundingClientRect().width - besides >= SLIDE_MIN;
}

/** Show it when the person wants it and it fits; else everything as before. */
export function fit() {
  const on = !!setting('ui.inspector') && fits();
  if (on === shown()) return;
  main.classList.toggle('inspector', on);
  for (const [bar, name] of [[document.querySelector('#slidebar'), 'slide'], [document.querySelector('#elbar'), 'element'], [fmtbar, 'text']]) {
    bar.dataset.tab = name;
    // back home: the slide bar heads the stage, the floating bars live in the body
    if (on) bars.append(bar);
    else if (name === 'slide') stage.prepend(bar);
    else document.body.append(bar);
  }
  update();
  dispatchEvent(new Event('qd-inspector'));
}

/** The tabs for what is selected now, and the one to show (data-model "Which tabs show"). */
function update() {
  const { kind, typeable, title } = what();
  const typing = !fmtbar.hidden;
  // (typing on the slide, nothing is selected: the text's formatting is what's there)
  const have = kind ? ['element', typeable && 'text', 'position'].filter(Boolean) : typing ? ['text'] : [title ? 'title' : 'slide'];
  tab = typing && have.includes('text') ? 'text' : kind ? (have.includes(elementTab) ? elementTab : 'element') : have[0];
  tabs.replaceChildren(...have.map((name) => {
    const b = el('button', { type: 'button', textContent: NAMES[name], id: `tab-${name}`, role: 'tab' });
    b.addEventListener('click', () => {
      if (kind) elementTab = name;
      tab = name;
      show();
    });
    return b;
  }));
  show();
}

function show() {
  for (const b of tabs.children) b.ariaSelected = String(b.id === `tab-${tab}`);
  for (const x of panel.querySelectorAll(':is(.bars, .body) > [data-tab]')) x.classList.toggle('off', x.dataset.tab !== tab);
}

export function init(get) {
  what = get;
  // the panel redrawn (a new selection), typing begun or ended
  new MutationObserver(update).observe(panel.querySelector('.body'), { childList: true });
  new MutationObserver(update).observe(fmtbar, { attributes: true, attributeFilter: ['hidden'] });
  // the window, the source or History pane, the panel: room for it or not
  new ResizeObserver(fit).observe(stage);
  addEventListener('resize', fit);
  fit();
}
