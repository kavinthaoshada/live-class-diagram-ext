import { classCompartments, stereotypeFor, ROW_HEIGHT, HEADER_ROW_HEIGHT, PADDING_X, PADDING_Y } from './model.js';

const SVG_NS = 'http://www.w3.org/2000/svg';

function svgEl(tag, attrs = {}) {
  const el = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) {
    el.setAttribute(key, value);
  }
  return el;
}

function buildMarkerDefs() {
  const defs = svgEl('defs');

  const hollowTriangle = svgEl('marker', {
    id: 'arrow-hollow',
    viewBox: '0 0 20 20',
    refX: '18',
    refY: '10',
    markerWidth: '16',
    markerHeight: '16',
    orient: 'auto',
  });
  hollowTriangle.appendChild(
    svgEl('path', { d: 'M 2 2 L 18 10 L 2 18 Z', class: 'marker-hollow' })
  );
  defs.appendChild(hollowTriangle);

  const openArrow = svgEl('marker', {
    id: 'arrow-open',
    viewBox: '0 0 20 20',
    refX: '16',
    refY: '10',
    markerWidth: '14',
    markerHeight: '14',
    orient: 'auto',
  });
  openArrow.appendChild(
    svgEl('path', { d: 'M 2 2 L 16 10 L 2 18', class: 'marker-open' })
  );
  defs.appendChild(openArrow);

  const diamondFilled = svgEl('marker', {
    id: 'diamond-filled',
    viewBox: '0 0 24 14',
    refX: '2',
    refY: '7',
    markerWidth: '22',
    markerHeight: '13',
    orient: 'auto',
  });
  diamondFilled.appendChild(
    svgEl('path', { d: 'M 2 7 L 12 1 L 22 7 L 12 13 Z', class: 'marker-diamond-filled' })
  );
  defs.appendChild(diamondFilled);

  const diamondHollow = svgEl('marker', {
    id: 'diamond-hollow',
    viewBox: '0 0 24 14',
    refX: '2',
    refY: '7',
    markerWidth: '22',
    markerHeight: '13',
    orient: 'auto',
  });
  diamondHollow.appendChild(
    svgEl('path', { d: 'M 2 7 L 12 1 L 22 7 L 12 13 Z', class: 'marker-diamond-hollow' })
  );
  defs.appendChild(diamondHollow);

  return defs;
}

const DASHED_KINDS = new Set(['implementation', 'dependency']);
const START_MARKER = { composition: 'diamond-filled', aggregation: 'diamond-hollow' };
const END_MARKER = {
  inheritance: 'arrow-hollow',
  implementation: 'arrow-hollow',
  association: 'arrow-open',
  dependency: 'arrow-open',
};

function pathData(points) {
  return points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');
}

function buildEdge(edge) {
  const group = svgEl('g', {
    class: `edge edge-${edge.rel.kind}`,
    'data-from': edge.from,
    'data-to': edge.to,
  });
  const path = svgEl('path', {
    d: pathData(edge.points),
    class: `edge-line${DASHED_KINDS.has(edge.rel.kind) ? ' dashed' : ''}`,
    fill: 'none',
  });
  const startMarker = START_MARKER[edge.rel.kind];
  const endMarker = END_MARKER[edge.rel.kind];
  if (startMarker) path.setAttribute('marker-start', `url(#${startMarker})`);
  if (endMarker) path.setAttribute('marker-end', `url(#${endMarker})`);
  group.appendChild(path);
  return group;
}

function buildGroupNode(group) {
  const tabWidth = Math.min(Math.max(group.label.length * 7 + 24, 64), group.width - 8);
  const el = svgEl('g', { class: 'group-node' });

  el.appendChild(
    svgEl('path', {
      class: 'group-tab',
      d: `M ${group.x} ${group.y + 26} V ${group.y + 6} Q ${group.x} ${group.y} ${group.x + 6} ${group.y} H ${group.x + tabWidth - 6} Q ${group.x + tabWidth} ${group.y} ${group.x + tabWidth} ${group.y + 6} V ${group.y + 26} Z`,
    })
  );
  el.appendChild(
    svgEl('rect', {
      class: 'group-box',
      x: group.x,
      y: group.y + 22,
      width: group.width,
      height: group.height - 22,
      rx: 4,
    })
  );
  const label = svgEl('text', { class: 'group-label', x: group.x + 10, y: group.y + 17 });
  label.textContent = group.label;
  el.appendChild(label);

  return el;
}

function buildClassNode(node, focusedId) {
  const { cls, width, height } = node;
  const stereotype = stereotypeFor(cls.kind);
  const { sections } = classCompartments(cls);

  const group = svgEl('g', {
    class: `class-node kind-${cls.kind}${cls.id === focusedId ? ' focused' : ''}`,
    'data-id': cls.id,
    transform: `translate(${node.x}, ${node.y})`,
  });

  group.appendChild(svgEl('rect', { class: 'box', x: 0, y: 0, width, height, rx: 5 }));

  let cursorY = PADDING_Y;
  if (stereotype) {
    const stereotypeText = svgEl('text', { class: 'stereotype', x: width / 2, y: cursorY + 11, 'text-anchor': 'middle' });
    stereotypeText.textContent = `«${stereotype}»`;
    group.appendChild(stereotypeText);
    cursorY += HEADER_ROW_HEIGHT;
  }
  const nameText = svgEl('text', { class: 'class-name', x: width / 2, y: cursorY + 12, 'text-anchor': 'middle' });
  nameText.textContent = cls.name;
  group.appendChild(nameText);
  cursorY += HEADER_ROW_HEIGHT + PADDING_Y;

  group.appendChild(svgEl('line', { class: 'divider', x1: 0, y1: cursorY, x2: width, y2: cursorY }));

  for (const section of sections) {
    const lines = section.length > 0 ? section : [{ text: '', line: null }];
    for (const line of lines) {
      const attrs = { class: 'member', x: PADDING_X, y: cursorY + 14 };
      if (line.line) {
        attrs['data-line'] = line.line;
      }
      const lineText = svgEl('text', attrs);
      lineText.textContent = line.text;
      group.appendChild(lineText);
      cursorY += ROW_HEIGHT;
    }
    cursorY += PADDING_Y;
    group.appendChild(svgEl('line', { class: 'divider', x1: 0, y1: cursorY, x2: width, y2: cursorY }));
  }

  return group;
}

export function renderDiagram(container, layout, options = {}) {
  const { focusedId = null, onFocusRequest = null, onNavigateRequest = null } = options;

  container.innerHTML = '';
  const svg = svgEl('svg', {
    id: 'diagram-svg',
    width: layout.width,
    height: layout.height,
    viewBox: `0 0 ${layout.width} ${layout.height}`,
    // svg-pan-zoom rewrites viewBox live as the user pans/zooms, so exporting
    // later can't rely on it (or on clientWidth/Height) to recover the
    // diagram's actual logical size — stash the original values instead.
    'data-content-width': layout.width,
    'data-content-height': layout.height,
  });
  svg.appendChild(buildMarkerDefs());

  if (layout.groups && layout.groups.length > 0) {
    const groupLayer = svgEl('g', { class: 'group-layer' });
    for (const group of layout.groups) {
      groupLayer.appendChild(buildGroupNode(group));
    }
    svg.appendChild(groupLayer);
  }

  const edgeLayer = svgEl('g', { class: 'edge-layer' });
  for (const edge of layout.edges) {
    edgeLayer.appendChild(buildEdge(edge));
  }
  svg.appendChild(edgeLayer);

  const nodeLayer = svgEl('g', { class: 'node-layer' });
  for (const node of layout.nodes) {
    nodeLayer.appendChild(buildClassNode(node, focusedId));
  }
  svg.appendChild(nodeLayer);

  attachInteractions(svg, onFocusRequest, onNavigateRequest);
  container.appendChild(svg);
  return svg;
}

function attachInteractions(svg, onFocusRequest, onNavigateRequest) {
  const nodes = svg.querySelectorAll('.class-node');
  for (const node of nodes) {
    node.addEventListener('mouseenter', () => setHighlight(svg, node.dataset.id));
    node.addEventListener('mouseleave', () => setHighlight(svg, null));
    if (onFocusRequest) {
      node.addEventListener('dblclick', () => onFocusRequest(node.dataset.id));
    }
    if (onNavigateRequest) {
      // Ctrl/Cmd+click to jump to source, so it never fights with the
      // plain double-click used for focus mode.
      node.addEventListener('click', (event) => {
        if (event.ctrlKey || event.metaKey) {
          onNavigateRequest(node.dataset.id);
        }
      });

      for (const memberEl of node.querySelectorAll('.member[data-line]')) {
        memberEl.addEventListener('click', (event) => {
          if (event.ctrlKey || event.metaKey) {
            // Stop the class-level handler above from also firing and
            // navigating to the class's own line instead of this member's.
            event.stopPropagation();
            onNavigateRequest(node.dataset.id, Number(memberEl.dataset.line));
          }
        });
      }
    }
  }
}

function setHighlight(svg, id) {
  const active = Boolean(id);
  svg.classList.toggle('has-highlight', active);
  for (const node of svg.querySelectorAll('.class-node')) {
    node.classList.toggle('dimmed', active && node.dataset.id !== id);
  }
  for (const edge of svg.querySelectorAll('.edge')) {
    const connected = edge.dataset.from === id || edge.dataset.to === id;
    edge.classList.toggle('dimmed', active && !connected);
    edge.classList.toggle('highlighted', active && connected);
  }
}
