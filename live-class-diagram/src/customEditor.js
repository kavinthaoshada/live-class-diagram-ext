class DiagramEditorProvider {
  constructor(panelManager) {
    this.panelManager = panelManager;
  }

  async openCustomDocument(uri) {
    return { uri, dispose() {} };
  }

  async resolveCustomEditor(_document, webviewPanel) {
    this.panelManager.attach(webviewPanel);
  }
}

module.exports = { DiagramEditorProvider };
