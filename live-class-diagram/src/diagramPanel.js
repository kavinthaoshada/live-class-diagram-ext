const vscode = require('vscode');
const crypto = require('crypto');
const path = require('path');
const os = require('os');

class DiagramPanelManager {
  constructor(context, engine) {
    this.context = context;
    this.engine = engine;
    this.panels = new Set();
    this.latestDiagram = null;
    this.commandPanel = null;

    engine.on('diagram', (diagram) => {
      this.latestDiagram = diagram;
      for (const panel of this.panels) {
        panel.webview.postMessage({ type: 'update', diagram });
      }
    });
  }

  currentWorkspaceRoot() {
    const folders = vscode.workspace.workspaceFolders;
    return folders && folders.length > 0 ? folders[0].uri.fsPath : undefined;
  }

  ensureEngineRunning() {
    const root = this.currentWorkspaceRoot();
    if (root) {
      this.engine.start(root);
    }
  }

  reveal() {
    if (this.commandPanel) {
      this.commandPanel.reveal(vscode.ViewColumn.Beside);
      return;
    }
    const panel = vscode.window.createWebviewPanel(
      'liveClassDiagram.diagramView',
      'Live Class Diagram',
      vscode.ViewColumn.Beside,
      { enableScripts: true, retainContextWhenHidden: true }
    );
    this.commandPanel = panel;
    panel.onDidDispose(() => {
      this.commandPanel = null;
    });
    this.attach(panel);
  }

  attach(panel) {
    panel.webview.options = { enableScripts: true };
    panel.webview.html = this.getHtml(panel.webview);
    this.panels.add(panel);

    panel.webview.onDidReceiveMessage((message) => {
      if (message.type === 'ready' && this.latestDiagram) {
        panel.webview.postMessage({ type: 'update', diagram: this.latestDiagram });
      }
      if (message.type === 'export') {
        this.handleExport(message);
      }
      if (message.type === 'export-error') {
        vscode.window.showErrorMessage(`Live Class Diagram: export failed (${message.message}).`);
      }
      if (message.type === 'navigate') {
        this.handleNavigate(message);
      }
    });

    panel.onDidDispose(() => {
      this.panels.delete(panel);
    });

    this.ensureEngineRunning();
  }

  async handleExport(message) {
    const isPng = message.format === 'png';
    const extension = isPng ? 'png' : 'svg';
    const defaultDir = this.currentWorkspaceRoot() || os.homedir();

    const uri = await vscode.window.showSaveDialog({
      defaultUri: vscode.Uri.file(path.join(defaultDir, `class-diagram.${extension}`)),
      filters: isPng ? { 'PNG Image': ['png'] } : { 'SVG Image': ['svg'] },
    });
    if (!uri) {
      return;
    }

    const buffer = isPng
      ? Buffer.from(message.dataUrl.split(',')[1], 'base64')
      : Buffer.from(message.svgText, 'utf8');

    await vscode.workspace.fs.writeFile(uri, buffer);
    vscode.window.showInformationMessage(`Live Class Diagram: exported to ${uri.fsPath}`);
  }

  async handleNavigate(message) {
    try {
      const uri = vscode.Uri.file(message.file);
      const document = await vscode.workspace.openTextDocument(uri);
      const line = Math.max(0, (message.line || 1) - 1);
      const range = new vscode.Range(line, 0, line, 0);
      const editor = await vscode.window.showTextDocument(document, { preview: true, selection: range });
      editor.revealRange(range, vscode.TextEditorRevealType.InCenter);
    } catch (err) {
      vscode.window.showErrorMessage(`Live Class Diagram: could not open ${message.file} (${err.message}).`);
    }
  }

  getHtml(webview) {
    const mediaUri = (file) =>
      webview.asWebviewUri(vscode.Uri.joinPath(this.context.extensionUri, 'media', file));
    const scriptUri = mediaUri('webview.js');
    const styleUri = mediaUri('webview.css');
    const nonce = crypto.randomBytes(16).toString('hex');

    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource} blob:; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';" />
<link href="${styleUri}" rel="stylesheet" />
<title>Live Class Diagram</title>
</head>
<body>
<div id="search-box">
  <input id="search-input" type="text" placeholder="Search classes..." title="Highlight classes by name, field, or method" />
</div>
<div id="toolbar">
  <button id="group-toggle" title="Group classes by their containing folder">Group</button>
  <button id="exit-focus" title="Return to the full diagram" class="hidden">Show All</button>
  <button id="export-png" title="Export the current view as a PNG image">Export PNG</button>
  <button id="export-svg" title="Export the current view as an SVG image">Export SVG</button>
  <button id="zoom-in" title="Zoom in">+</button>
  <button id="zoom-out" title="Zoom out">-</button>
  <button id="zoom-reset" title="Reset view">Reset</button>
</div>
<div id="hint">Double-click a class to focus on it. Ctrl/Cmd+click a class or a field/method to open it in the editor. Press Esc or "Show All" to return from focus.</div>
<div id="empty-state">No classes detected yet. Edit or add a source file to see the live diagram.</div>
<div id="diagram-container"></div>
<script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
  }
}

module.exports = { DiagramPanelManager };
