import svgPanZoom from 'svg-pan-zoom';
import { computeLayout } from './layout.js';
import { renderDiagram } from './render.js';
import { exportSvgString, exportPngDataUrl } from './export.js';

const vscode = acquireVsCodeApi();
const container = document.getElementById('diagram-container');
const emptyState = document.getElementById('empty-state');
const groupToggleButton = document.getElementById('group-toggle');
const exitFocusButton = document.getElementById('exit-focus');
const exportPngButton = document.getElementById('export-png');
const exportSvgButton = document.getElementById('export-svg');
const searchInput = document.getElementById('search-input');
const hint = document.getElementById('hint');
const DEFAULT_HINT_TEXT = hint.textContent;

const savedState = vscode.getState() || {};

let panZoom = null;
let currentDiagram = null;
let groupByFolder = Boolean(savedState.groupByFolder);
let focusedClassId = savedState.focusedClassId || null;
let lastViewKey = null;

groupToggleButton.classList.toggle('active', groupByFolder);

function persistState() {
  vscode.setState({ groupByFolder, focusedClassId });
}

function buildFocusDiagram(diagram, focusedId) {
  const focused = diagram.classes.find((c) => c.id === focusedId);
  if (!focused) {
    return null;
  }
  const relationships = diagram.relationships.filter(
    (rel) => rel.from === focusedId || rel.to === focusedId
  );
  const neighborIds = new Set([focusedId]);
  for (const rel of relationships) {
    neighborIds.add(rel.from);
    neighborIds.add(rel.to);
  }
  const classes = diagram.classes.filter((c) => neighborIds.has(c.id));
  return { classes, relationships };
}

function setFocusedClass(id) {
  focusedClassId = id;
  persistState();
  renderCurrentView();
}

function navigateToClass(id, line) {
  const cls = currentDiagram && currentDiagram.classes.find((c) => c.id === id);
  if (!cls) {
    return;
  }
  vscode.postMessage({ type: 'navigate', file: cls.file, line: line || cls.line });
}

function classMatchesQuery(cls, query) {
  if (cls.name.toLowerCase().includes(query)) {
    return true;
  }
  if (cls.fields.some((f) => f.name.toLowerCase().includes(query))) {
    return true;
  }
  if (cls.methods.some((m) => m.name.toLowerCase().includes(query))) {
    return true;
  }
  return false;
}

function applySearchHighlight() {
  const svg = document.getElementById('diagram-svg');
  if (!svg || !currentDiagram) {
    return;
  }
  const query = searchInput.value.trim().toLowerCase();
  const active = query.length > 0;
  const nodes = svg.querySelectorAll('.class-node');

  let matchCount = 0;
  for (const node of nodes) {
    const cls = currentDiagram.classes.find((c) => c.id === node.dataset.id);
    const matches = active && cls && classMatchesQuery(cls, query);
    if (matches) {
      matchCount += 1;
    }
    node.classList.toggle('search-match', matches);
    node.classList.toggle('search-dimmed', active && !matches);
  }

  hint.textContent = active
    ? `${matchCount} of ${nodes.length} classes match "${searchInput.value.trim()}"`
    : DEFAULT_HINT_TEXT;
}

function renderCurrentView() {
  if (!currentDiagram) {
    return;
  }

  const hasClasses = currentDiagram.classes.length > 0;
  emptyState.classList.toggle('visible', !hasClasses);
  container.classList.toggle('visible', hasClasses);

  exportPngButton.disabled = !hasClasses;
  exportSvgButton.disabled = !hasClasses;

  if (!hasClasses) {
    container.innerHTML = '';
    if (panZoom) {
      panZoom.destroy();
      panZoom = null;
    }
    return;
  }

  let viewDiagram = currentDiagram;
  if (focusedClassId) {
    const focusDiagram = buildFocusDiagram(currentDiagram, focusedClassId);
    if (!focusDiagram) {
      // The focused class was removed by a live edit; fall back to the full diagram.
      focusedClassId = null;
      persistState();
    } else {
      viewDiagram = focusDiagram;
    }
  }

  const viewKey = focusedClassId ? `focus:${focusedClassId}` : groupByFolder ? 'grouped' : 'full';
  const modeChanged = viewKey !== lastViewKey;
  lastViewKey = viewKey;

  const previousPan = !modeChanged && panZoom ? panZoom.getPan() : null;
  const previousZoom = !modeChanged && panZoom ? panZoom.getZoom() : null;
  if (panZoom) {
    panZoom.destroy();
    panZoom = null;
  }

  const layout = computeLayout(viewDiagram, { groupByFolder: groupByFolder && !focusedClassId });
  const svg = renderDiagram(container, layout, {
    focusedId: focusedClassId,
    onFocusRequest: setFocusedClass,
    onNavigateRequest: navigateToClass,
  });

  panZoom = svgPanZoom(svg, {
    zoomEnabled: true,
    controlIconsEnabled: false,
    fit: modeChanged,
    center: modeChanged,
    minZoom: 0.05,
    maxZoom: 24,
    zoomScaleSensitivity: 0.3,
  });

  if (!modeChanged && previousPan && previousZoom) {
    panZoom.zoom(previousZoom);
    panZoom.pan(previousPan);
  }

  exitFocusButton.classList.toggle('hidden', !focusedClassId);
  groupToggleButton.disabled = Boolean(focusedClassId);

  applySearchHighlight();
}

window.addEventListener('message', (event) => {
  const message = event.data;
  if (message.type === 'update') {
    currentDiagram = message.diagram;
    renderCurrentView();
  }
});

window.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && focusedClassId) {
    setFocusedClass(null);
  }
});

groupToggleButton.addEventListener('click', () => {
  groupByFolder = !groupByFolder;
  groupToggleButton.classList.toggle('active', groupByFolder);
  persistState();
  renderCurrentView();
});

exitFocusButton.addEventListener('click', () => setFocusedClass(null));

searchInput.addEventListener('input', () => applySearchHighlight());

exportSvgButton.addEventListener('click', () => {
  const svg = document.getElementById('diagram-svg');
  if (!svg) {
    return;
  }
  vscode.postMessage({ type: 'export', format: 'svg', svgText: exportSvgString(svg) });
});

exportPngButton.addEventListener('click', async () => {
  const svg = document.getElementById('diagram-svg');
  if (!svg) {
    return;
  }
  try {
    const dataUrl = await exportPngDataUrl(svg);
    vscode.postMessage({ type: 'export', format: 'png', dataUrl });
  } catch (err) {
    vscode.postMessage({ type: 'export-error', message: String(err && err.message ? err.message : err) });
  }
});

document.getElementById('zoom-in').addEventListener('click', () => panZoom && panZoom.zoomIn());
document.getElementById('zoom-out').addEventListener('click', () => panZoom && panZoom.zoomOut());
document.getElementById('zoom-reset').addEventListener('click', () => {
  if (panZoom) {
    panZoom.resetZoom();
    panZoom.center();
  }
});

vscode.postMessage({ type: 'ready' });
