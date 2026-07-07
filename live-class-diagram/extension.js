const vscode = require('vscode');
const { EngineProcess } = require('./src/engineProcess');
const { DiagramPanelManager } = require('./src/diagramPanel');
const { DiagramEditorProvider } = require('./src/customEditor');
const { ensurePlaceholderFile } = require('./src/placeholder');

function activate(context) {
  const engine = new EngineProcess(context);
  const panelManager = new DiagramPanelManager(context, engine);
  const editorProvider = new DiagramEditorProvider(panelManager);

  ensurePlaceholderFile();

  context.subscriptions.push(
    engine,
    vscode.commands.registerCommand('liveClassDiagram.open', () => panelManager.reveal()),
    vscode.commands.registerCommand('liveClassDiagram.refresh', () => engine.restart()),
    vscode.window.registerCustomEditorProvider('liveClassDiagram.diagramView', editorProvider, {
      webviewOptions: { retainContextWhenHidden: true },
      supportsMultipleEditorsPerDocument: false,
    })
  );
}

function deactivate() {}

module.exports = {
  activate,
  deactivate,
};
