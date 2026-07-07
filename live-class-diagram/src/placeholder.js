const vscode = require('vscode');
const fs = require('fs');
const path = require('path');

const PLACEHOLDER_NAME = 'ClassDiagram.liveclass';
const PLACEHOLDER_CONTENT = JSON.stringify(
  {
    _comment: 'This file opens the Live Class Diagram view in VS Code. Do not edit or delete.',
    generatedBy: 'live-class-diagram',
  },
  null,
  2
);

function ensurePlaceholderFile() {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    return;
  }
  const target = path.join(folders[0].uri.fsPath, PLACEHOLDER_NAME);
  if (!fs.existsSync(target)) {
    fs.writeFileSync(target, PLACEHOLDER_CONTENT, 'utf8');
  }
}

module.exports = { ensurePlaceholderFile, PLACEHOLDER_NAME };
