// The editor's own sheets (spec 008 US5, research R8): a small dialog beside what
// opened it, in place of the browser's prompt, confirm and alert. Escape and
// Cancel give null; Enter and OK give the values, once every field is valid.

const el = (tag, props = {}) => Object.assign(document.createElement(tag), props);
/** A slide's background colours: light first, two dark. */
const BACKGROUNDS = ['#ffffff', '#fdf6e3', '#f4f6fb', '#eaf4ec', '#fff4e5', '#fdecea', '#222225', '#000000'];
const previewUrl = (path) => '/preview/' + path.split('/').map(encodeURIComponent).join('/');

/**
 * Ask: `fields` are `{name, label, kind: 'text'|'swatches'|'files'|'gallery', value,
 * validate?(value) → message, placeholder?}`; `anchor` is the element (or rect) it
 * opens beside. Resolves to `{name: value}` on OK, `null` otherwise.
 */
export function ask({ title, message = '', fields = [], ok = 'OK', cancel = 'Cancel', anchor = null }) {
  const d = el('dialog', { className: 'sheet', ariaLabel: title });
  const form = el('form', { method: 'dialog' });
  form.append(el('h2', { textContent: title }));
  if (message) form.append(el('p', { textContent: message }));
  const values = {};
  const checks = [];
  for (const f of fields) form.append(field(f, values, checks, () => form.requestSubmit(okB)));
  const okB = el('button', { type: 'submit', value: 'ok', textContent: ok, className: 'default' });
  const bar = el('div', { className: 'buttons' });
  if (cancel) {
    const c = el('button', { type: 'button', value: 'cancel', textContent: cancel });
    c.addEventListener('click', () => d.close());
    bar.append(c);
  }
  bar.append(okB);
  form.append(bar);
  d.append(form);
  document.body.append(d);
  const validate = () => { okB.disabled = checks.map((c) => c()).some(Boolean); };
  form.addEventListener('input', validate);
  validate();
  d.showModal();
  const r = anchor?.getBoundingClientRect?.() ?? anchor;
  if (r) {
    Object.assign(d.style, { margin: '0', inset: 'auto' });
    d.style.left = Math.max(8, Math.min(r.left, innerWidth - d.offsetWidth - 8)) + 'px';
    d.style.top = Math.max(8, r.bottom + 4 + d.offsetHeight > innerHeight ? r.top - d.offsetHeight - 4 : r.bottom + 4) + 'px';
  }
  (form.querySelector('input[type=radio]:checked') || form.querySelector('input:not([type=color])') || okB).focus();
  // a task later: the dialog puts focus back where it was first, so a caller can then move it
  return new Promise((done) => d.addEventListener('close', () => setTimeout(() => {
    done(d.returnValue === 'ok' ? Object.fromEntries(Object.entries(values).map(([k, get]) => [k, get()])) : null);
    d.remove();
  })));
}

/** Yes or no: true on OK. */
export const confirmSheet = async (title, message, ok, anchor) => !!(await ask({ title, message, ok, anchor }));

function field(f, values, checks, submit) {
  const wrap = el('div', { className: `field is-${f.kind}` });
  const err = el('span', { className: 'error', role: 'alert' });
  if (f.kind === 'gallery') return gallery(f, wrap, values, submit);
  if (f.kind === 'swatches') {
    let value = f.value ?? '';
    const group = el('div', { className: 'swatches', role: 'radiogroup', ariaLabel: f.label });
    const pick = (v) => {
      value = v;
      for (const x of group.children) x.setAttribute('aria-checked', String(x.dataset.value === v));
    };
    const custom = el('input', { type: 'color', value: /^#[0-9a-f]{6}$/i.test(value) ? value : '#ffffff', title: 'Custom…', ariaLabel: 'Custom colour' });
    for (const c of ['', ...BACKGROUNDS]) {
      const b = el('button', { type: 'button', title: c || 'None', textContent: c ? '' : 'None', role: 'radio' });
      b.dataset.value = c;
      if (c) b.style.background = c;
      b.addEventListener('click', () => pick(c));
      group.append(b);
    }
    custom.addEventListener('input', () => pick(custom.value));
    pick(value);
    wrap.append(el('span', { className: 'label', textContent: f.label }), group, custom);
    values[f.name] = () => value;
    return wrap;
  }
  // text, and files (a list of the deck's images that fills the text field)
  const input = el('input', { type: 'text', name: f.name, value: f.value ?? '', placeholder: f.placeholder ?? '' });
  const label = el('label', { textContent: f.label });
  label.append(input);
  if (f.kind === 'files') {
    const list = el('ul', { className: 'files', ariaLabel: 'Images beside the deck' });
    fetch('/api/images').then((r) => r.json()).then(({ images }) => {
      if (!images.length) list.append(el('li', { className: 'empty', textContent: 'No images beside the deck yet — drop one on the slide.' }));
      for (const { path } of images) {
        const b = el('button', { type: 'button', title: path });
        b.dataset.path = path;
        b.append(el('img', { src: previewUrl(path), alt: '' }), el('span', { textContent: path }));
        b.setAttribute('aria-pressed', String(path === input.value));
        b.addEventListener('click', () => {
          for (const x of list.querySelectorAll('button')) x.setAttribute('aria-pressed', String(x === b));
          input.value = path;
          input.dispatchEvent(new Event('input', { bubbles: true }));
        });
        b.addEventListener('dblclick', submit);
        const li = el('li');
        li.append(b);
        list.append(li);
      }
    }, () => {});
    wrap.append(list);
  }
  wrap.append(label, err);
  values[f.name] = () => input.value.trim();
  // OK waits for a valid value; the message waits until the person has typed
  let touched = false;
  input.addEventListener('input', () => { touched = true; });
  checks.push(() => {
    const msg = f.validate?.(input.value.trim()) || '';
    err.textContent = touched ? msg : '';
    input.setAttribute('aria-invalid', String(touched && !!msg));
    return !!msg;
  });
  return wrap;
}

const FROM = { deck: 'This deck’s', mine: 'Yours', 'built-in': 'Built in' };

/**
 * Layouts to pick from (spec 009): `f.options` as `qd layouts` lists them, `f.size`
 * the deck's [width, height]. Each preview is drawn from the layout's own boxes;
 * arrows move between them (radio buttons), Enter or a double click picks.
 */
function gallery(f, wrap, values, submit) {
  const [w, h] = f.size;
  const headed = f.options.some((x) => x.from !== 'built-in'); // headings only when there's more than the built-in ones
  let from = null;
  let grid = null;
  for (const o of f.options) {
    if (o.from !== from) {
      from = o.from;
      const name = FROM[from] || from;
      grid = el('div', { className: 'gallery', role: 'radiogroup', ariaLabel: name });
      if (headed) wrap.append(el('h3', { textContent: name }));
      wrap.append(grid);
    }
    const label = el('label', { title: o.description });
    label.dataset.layout = o.name;
    const radio = el('input', { type: 'radio', name: f.name, value: o.name, checked: o.name === f.value });
    const preview = el('span', { className: 'preview' });
    preview.append(el('span', { className: 'bar' }));
    for (const b of o.boxes) {
      const box = el('span', { className: `box ${b.kind}` });
      Object.assign(box.style, { left: `${(100 * b.left) / w}%`, top: `${(100 * b.top) / h}%`, width: `${(100 * b.width) / w}%`, height: `${(100 * b.height) / h}%` });
      preview.append(box);
    }
    label.append(radio, preview, el('span', { className: 'name', textContent: o.name.replace(/-/g, ' ') }));
    label.addEventListener('dblclick', submit);
    grid.append(label);
  }
  if (!wrap.querySelector('input:checked')) wrap.querySelector('input').checked = true;
  values[f.name] = () => wrap.querySelector('input:checked')?.value;
  return wrap;
}
