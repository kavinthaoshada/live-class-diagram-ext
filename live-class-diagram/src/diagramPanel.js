const vscode = require('vscode');
const crypto = require('crypto');

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
    });

    panel.onDidDispose(() => {
      this.panels.delete(panel);
    });

    this.ensureEngineRunning();
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
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource}; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';" />
<link href="${styleUri}" rel="stylesheet" />
<title>Live Class Diagram</title>
</head>
<body>
<div id="toolbar">
  <button id="group-toggle" title="Group classes by their containing folder">Group</button>
  <button id="exit-focus" title="Return to the full diagram" class="hidden">Show All</button>
  <button id="zoom-in" title="Zoom in">+</button>
  <button id="zoom-out" title="Zoom out">-</button>
  <button id="zoom-reset" title="Reset view">Reset</button>
</div>
<div id="hint">Double-click a class to focus on it and its direct relationships. Press Esc or "Show All" to return.</div>
<div id="empty-state">No classes detected yet. Edit or add a source file to see the live diagram.</div>
<div id="diagram-container"></div>
<script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
  }
}

module.exports = { DiagramPanelManager };
