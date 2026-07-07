import svgPanZoom from 'svg-pan-zoom';
import { computeLayout } from './layout.js';
import { renderDiagram } from './render.js';

const vscode = acquireVsCodeApi();
const container = document.getElementById('diagram-container');
const emptyState = document.getElementById('empty-state');
const groupToggleButton = document.getElementById('group-toggle');
const exitFocusButton = document.getElementById('exit-focus');

let panZoom = null;
let currentDiagram = null;
let groupByFolder = false;
let focusedClassId = null;
let lastViewKey = null;

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
  renderCurrentView();
}

function renderCurrentView() {
  if (!currentDiagram) {
    return;
  }

  const hasClasses = currentDiagram.classes.length > 0;
  emptyState.classList.toggle('visible', !hasClasses);
  container.classList.toggle('visible', hasClasses);

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
  renderCurrentView();
});

exitFocusButton.addEventListener('click', () => setFocusedClass(null));

document.getElementById('zoom-in').addEventListener('click', () => panZoom && panZoom.zoomIn());
document.getElementById('zoom-out').addEventListener('click', () => panZoom && panZoom.zoomOut());
document.getElementById('zoom-reset').addEventListener('click', () => {
  if (panZoom) {
    panZoom.resetZoom();
    panZoom.center();
  }
});

vscode.postMessage({ type: 'ready' });
