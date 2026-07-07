import dagre from 'dagre';
import {
  classCompartments,
  stereotypeFor,
  ROW_HEIGHT,
  HEADER_ROW_HEIGHT,
  PADDING_X,
  PADDING_Y,
  MIN_WIDTH,
} from './model.js';

const MEMBER_FONT = '12px Consolas, "SF Mono", Menlo, monospace';
const HEADER_FONT = 'bold 13px "Segoe UI", system-ui, sans-serif';
const STEREOTYPE_FONT = 'italic 11px "Segoe UI", system-ui, sans-serif';

const GROUP_GAP = 48;
const GROUP_TAB_HEIGHT = 26;
const GROUP_PADDING = 16;
const MAX_ROW_WIDTH = 1700;

const measureCanvas = document.createElement('canvas');
const measureCtx = measureCanvas.getContext('2d');

function textWidth(str, font) {
  measureCtx.font = font;
  return measureCtx.measureText(str).width;
}

function sizeOfClass(cls) {
  const stereotype = stereotypeFor(cls.kind);
  const { header, sections } = classCompartments(cls);

  let width = MIN_WIDTH;
  if (stereotype) {
    width = Math.max(width, textWidth(`«${stereotype}»`, STEREOTYPE_FONT) + PADDING_X * 2);
  }
  for (const line of header) {
    width = Math.max(width, textWidth(line, HEADER_FONT) + PADDING_X * 2);
  }
  for (const section of sections) {
    for (const line of section) {
      width = Math.max(width, textWidth(line, MEMBER_FONT) + PADDING_X * 2);
    }
  }

  const headerHeight = (stereotype ? HEADER_ROW_HEIGHT : 0) + HEADER_ROW_HEIGHT + PADDING_Y;
  const sectionsHeight = sections.reduce(
    (sum, section) => sum + Math.max(1, section.length) * ROW_HEIGHT + PADDING_Y,
    0
  );

  return { width, height: headerHeight + sectionsHeight };
}

function layoutSubgraph(classes, relationships) {
  const graph = new dagre.graphlib.Graph();
  graph.setGraph({ rankdir: 'TB', nodesep: 55, ranksep: 90, marginx: 24, marginy: 24 });
  graph.setDefaultEdgeLabel(() => ({}));

  const sizeById = new Map();
  for (const cls of classes) {
    const size = sizeOfClass(cls);
    sizeById.set(cls.id, size);
    graph.setNode(cls.id, { ...size, cls });
  }

  const validRelationships = relationships.filter(
    (rel) => sizeById.has(rel.from) && sizeById.has(rel.to)
  );
  for (const rel of validRelationships) {
    graph.setEdge(rel.from, rel.to, { rel });
  }

  dagre.layout(graph);

  const nodes = graph.nodes().map((id) => {
    const node = graph.node(id);
    return {
      id,
      x: node.x - node.width / 2,
      y: node.y - node.height / 2,
      width: node.width,
      height: node.height,
      cls: node.cls,
    };
  });

  const edges = graph.edges().map((edge) => {
    const data = graph.edge(edge);
    return { from: edge.v, to: edge.w, points: data.points, rel: data.rel };
  });

  const graphInfo = graph.graph();
  return {
    nodes,
    edges,
    width: graphInfo.width || MIN_WIDTH + GROUP_PADDING * 2,
    height: graphInfo.height || 120,
  };
}

export function computeLayout(diagram, options = {}) {
  return options.groupByFolder
    ? computeGroupedLayout(diagram)
    : layoutSubgraph(diagram.classes, diagram.relationships);
}

function folderGroupKey(filePath) {
  const normalized = filePath.replace(/\\/g, '/');
  const idx = normalized.lastIndexOf('/');
  return idx === -1 ? '.' : normalized.slice(0, idx);
}

function folderLabel(groupKey) {
  if (groupKey === '.') {
    return '(root)';
  }
  const idx = groupKey.lastIndexOf('/');
  return idx === -1 ? groupKey : groupKey.slice(idx + 1);
}

function groupClassesByFolder(classes) {
  const groups = new Map();
  for (const cls of classes) {
    const key = folderGroupKey(cls.file);
    if (!groups.has(key)) {
      groups.set(key, { key, label: folderLabel(key), classes: [] });
    }
    groups.get(key).classes.push(cls);
  }
  return [...groups.values()];
}

function boxCenter(node) {
  return { x: node.x + node.width / 2, y: node.y + node.height / 2 };
}

function clipToBox(node, externalPoint) {
  const center = boxCenter(node);
  const dx = externalPoint.x - center.x;
  const dy = externalPoint.y - center.y;
  if (dx === 0 && dy === 0) {
    return center;
  }
  const scaleX = dx !== 0 ? node.width / 2 / Math.abs(dx) : Infinity;
  const scaleY = dy !== 0 ? node.height / 2 / Math.abs(dy) : Infinity;
  const scale = Math.min(scaleX, scaleY);
  return { x: center.x + dx * scale, y: center.y + dy * scale };
}

function straightEdgeBetween(fromNode, toNode) {
  const fromCenter = boxCenter(fromNode);
  const toCenter = boxCenter(toNode);
  return [clipToBox(fromNode, toCenter), clipToBox(toNode, fromCenter)];
}

function computeGroupedLayout(diagram) {
  const groups = groupClassesByFolder(diagram.classes);

  const groupLayouts = groups.map((group) => {
    const classIds = new Set(group.classes.map((c) => c.id));
    const intraEdges = diagram.relationships.filter(
      (rel) => classIds.has(rel.from) && classIds.has(rel.to)
    );
    const subLayout = layoutSubgraph(group.classes, intraEdges);
    return { ...group, ...subLayout };
  });

  let cursorX = 0;
  let cursorY = 0;
  let rowHeight = 0;
  const positioned = [];
  for (const groupLayout of groupLayouts) {
    const boxWidth = groupLayout.width + GROUP_PADDING * 2;
    const boxHeight = groupLayout.height + GROUP_PADDING * 2 + GROUP_TAB_HEIGHT;
    if (cursorX > 0 && cursorX + boxWidth > MAX_ROW_WIDTH) {
      cursorX = 0;
      cursorY += rowHeight + GROUP_GAP;
      rowHeight = 0;
    }
    positioned.push({ ...groupLayout, offsetX: cursorX, offsetY: cursorY });
    cursorX += boxWidth + GROUP_GAP;
    rowHeight = Math.max(rowHeight, boxHeight);
  }

  const nodes = [];
  const groupBoxes = [];
  const nodeById = new Map();

  for (const group of positioned) {
    const innerOffsetX = group.offsetX + GROUP_PADDING;
    const innerOffsetY = group.offsetY + GROUP_PADDING + GROUP_TAB_HEIGHT;

    groupBoxes.push({
      label: group.label,
      x: group.offsetX,
      y: group.offsetY,
      width: group.width + GROUP_PADDING * 2,
      height: group.height + GROUP_PADDING * 2 + GROUP_TAB_HEIGHT,
    });

    for (const node of group.nodes) {
      const translated = { ...node, x: node.x + innerOffsetX, y: node.y + innerOffsetY };
      nodes.push(translated);
      nodeById.set(translated.id, translated);
    }
  }

  const edges = [];
  const drawnEdgeKeys = new Set();
  for (const group of positioned) {
    const innerOffsetX = group.offsetX + GROUP_PADDING;
    const innerOffsetY = group.offsetY + GROUP_PADDING + GROUP_TAB_HEIGHT;
    for (const edge of group.edges) {
      edges.push({
        ...edge,
        points: edge.points.map((p) => ({ x: p.x + innerOffsetX, y: p.y + innerOffsetY })),
      });
      drawnEdgeKeys.add(`${edge.from}->${edge.to}->${edge.rel.kind}`);
    }
  }

  for (const rel of diagram.relationships) {
    const key = `${rel.from}->${rel.to}->${rel.kind}`;
    if (drawnEdgeKeys.has(key)) {
      continue;
    }
    const fromNode = nodeById.get(rel.from);
    const toNode = nodeById.get(rel.to);
    if (!fromNode || !toNode) {
      continue;
    }
    edges.push({ from: rel.from, to: rel.to, rel, points: straightEdgeBetween(fromNode, toNode) });
  }

  const totalWidth = Math.max(...groupBoxes.map((g) => g.x + g.width), MIN_WIDTH) + GROUP_PADDING;
  const totalHeight = cursorY + rowHeight + GROUP_PADDING;

  return { nodes, edges, groups: groupBoxes, width: totalWidth, height: totalHeight };
}
