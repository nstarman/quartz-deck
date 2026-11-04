// quartz-deck editor shell (US2): live preview + slide navigation.
// All edits go through the qd CLI / API; this page only displays.
const frame = document.getElementById('preview');
const statusEl = document.getElementById('status');
const banner = document.getElementById('banner');
const list = document.getElementById('slides');
let html = null;

function currentHash() {
  try { return frame.contentWindow.location.hash || ''; } catch { return ''; }
}

function load(path) {
  const hash = currentHash() || location.hash;
  html = path;
  frame.src = path + '?v=' + Date.now() + hash;
}

function go(anchor) {
  const hash = '#/' + anchor;
  history.replaceState(null, '', hash);
  try { frame.contentWindow.location.hash = hash; } catch { /* not loaded yet */ }
}

async function refreshList() {
  const res = await fetch('/api/deck');
  if (!res.ok) return;
  const deck = await res.json();
  const here = currentHash().replace(/^#\//, '');
  list.replaceChildren(...deck.slides.map((s) => {
    const li = document.createElement('li');
    if (s.anchor === here) li.className = 'current';
    const b = document.createElement('button');
    b.type = 'button';
    b.dataset.id = s.id;
    const idx = document.createElement('span');
    idx.className = 'idx';
    idx.textContent = s.index;
    b.append(idx, s.title || s.summary || '(untitled)');
    b.addEventListener('click', () => go(s.anchor));
    li.append(b);
    return li;
  }));
}

frame.addEventListener('load', () => {
  try {
    frame.contentWindow.addEventListener('hashchange', refreshList);
  } catch { /* cross-origin never expected */ }
  refreshList();
});

const es = new EventSource('/events');
es.addEventListener('open', () => { statusEl.textContent = 'connected'; });
es.addEventListener('error', () => { statusEl.textContent = 'disconnected — is qd serve running?'; });
es.addEventListener('changed', () => { statusEl.textContent = 'changed…'; refreshList(); });
es.addEventListener('rendering', () => { statusEl.textContent = 'rendering…'; });
es.addEventListener('rendered', (e) => {
  const d = JSON.parse(e.data);
  statusEl.textContent = 'rendered ' + d.version.slice(0, 7);
  banner.hidden = true;
  load(d.html);
});
es.addEventListener('render_error', (e) => {
  const d = JSON.parse(e.data);
  statusEl.textContent = 'render failed (showing last good render)';
  banner.textContent = d.message;
  banner.hidden = false;
});
