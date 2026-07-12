const SVG_NS = 'http://www.w3.org/2000/svg';

const EXPORT_STYLE_PROPS = [
  'fill',
  'stroke',
  'stroke-width',
  'stroke-dasharray',
  'stroke-linecap',
  'stroke-linejoin',
  'opacity',
  'font-family',
  'font-size',
  'font-weight',
  'font-style',
  'text-anchor',
];

function backgroundColor() {
  const value = getComputedStyle(document.body).getPropertyValue('--vscode-editor-background');
  return value.trim() || '#1e1e1e';
}

// A cloned SVG serialized on its own has no access to the page's external
// stylesheet (webview.css) or its `var(--vscode-*)` custom properties, so
// every shape would render unstyled once it leaves this document (as a
// standalone .svg file, or rasterized to PNG via <img>/<canvas>). Baking in
// each element's already-resolved computed style makes the export look
// right regardless of where it's opened.
function buildExportableSvg(liveSvg) {
  const clone = liveSvg.cloneNode(true);
  const sourceEls = liveSvg.querySelectorAll('*');
  const cloneEls = clone.querySelectorAll('*');

  // svg-pan-zoom drops the top-level viewBox and instead wraps the whole
  // diagram in a `.svg-pan-zoom_viewport` <g> with a transform matrix
  // reflecting the current pan/zoom — reset it so the export always shows
  // the full diagram at its natural layout coordinates.
  const viewport = clone.querySelector('.svg-pan-zoom_viewport');
  if (viewport) {
    viewport.removeAttribute('transform');
  }

  sourceEls.forEach((sourceEl, i) => {
    const computed = getComputedStyle(sourceEl);
    let styleText = '';
    for (const prop of EXPORT_STYLE_PROPS) {
      const value = computed.getPropertyValue(prop);
      if (value) {
        styleText += `${prop}:${value};`;
      }
    }
    cloneEls[i].setAttribute('style', styleText);
  });

  const width = Number(liveSvg.dataset.contentWidth) || liveSvg.viewBox.baseVal.width || liveSvg.clientWidth;
  const height = Number(liveSvg.dataset.contentHeight) || liveSvg.viewBox.baseVal.height || liveSvg.clientHeight;

  // The live SVG's viewBox/width/height may currently reflect whatever
  // svg-pan-zoom last panned/zoomed to, not the full diagram — pin the
  // clone back to the full logical size regardless of on-screen zoom state.
  clone.setAttribute('viewBox', `0 0 ${width} ${height}`);
  clone.setAttribute('width', String(width));
  clone.setAttribute('height', String(height));
  clone.removeAttribute('style');

  const backgroundRect = document.createElementNS(SVG_NS, 'rect');
  backgroundRect.setAttribute('x', '0');
  backgroundRect.setAttribute('y', '0');
  backgroundRect.setAttribute('width', String(width));
  backgroundRect.setAttribute('height', String(height));
  backgroundRect.setAttribute('fill', backgroundColor());
  clone.insertBefore(backgroundRect, clone.firstChild);

  return { svg: clone, width, height };
}

export function exportSvgString(liveSvg) {
  const { svg } = buildExportableSvg(liveSvg);
  const serializer = new XMLSerializer();
  return `<?xml version="1.0" encoding="UTF-8"?>\n${serializer.serializeToString(svg)}`;
}

export function exportPngDataUrl(liveSvg, scale = 2) {
  return new Promise((resolve, reject) => {
    const { svg, width, height } = buildExportableSvg(liveSvg);
    const serializer = new XMLSerializer();
    const svgString = serializer.serializeToString(svg);

    const blob = new Blob([svgString], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const img = new Image();

    img.onload = () => {
      const canvas = document.createElement('canvas');
      canvas.width = width * scale;
      canvas.height = height * scale;
      const ctx = canvas.getContext('2d');
      ctx.scale(scale, scale);
      ctx.drawImage(img, 0, 0, width, height);
      URL.revokeObjectURL(url);
      resolve(canvas.toDataURL('image/png'));
    };
    img.onerror = (err) => {
      URL.revokeObjectURL(url);
      reject(err);
    };
    img.src = url;
  });
}
