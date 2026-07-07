const vscode = require('vscode');
const path = require('path');
const fs = require('fs');
const cp = require('child_process');
const readline = require('readline');
const { EventEmitter } = require('events');

function platformExeName() {
  return process.platform === 'win32' ? 'rust-engine.exe' : 'rust-engine';
}

class EngineProcess extends EventEmitter {
  constructor(context) {
    super();
    this.context = context;
    this.child = null;
    this.root = null;
    this.building = false;
  }

  resolveEnginePath() {
    const configured = vscode.workspace.getConfiguration('liveClassDiagram').get('enginePath');
    if (configured && configured.trim().length > 0) {
      return configured;
    }
    return this.context.asAbsolutePath(path.join('bin', platformExeName()));
  }

  start(root) {
    if (this.child && this.root === root) {
      return;
    }
    this.stop();
    this.root = root;

    const enginePath = this.resolveEnginePath();
    if (!fs.existsSync(enginePath)) {
      this.offerToBuildEngine(enginePath);
      return;
    }

    let child;
    try {
      child = cp.spawn(enginePath, ['watch', root]);
    } catch (err) {
      this.reportSpawnFailure(err);
      return;
    }

    child.on('error', (err) => this.reportSpawnFailure(err));

    const rl = readline.createInterface({ input: child.stdout });
    rl.on('line', (line) => {
      const trimmed = line.trim();
      if (!trimmed) {
        return;
      }
      try {
        this.emit('diagram', JSON.parse(trimmed));
      } catch (err) {
        console.error('Live Class Diagram: failed to parse engine output', err);
      }
    });

    child.stderr.on('data', (data) => {
      console.error(`rust-engine: ${data}`);
    });

    this.child = child;
  }

  offerToBuildEngine(enginePath) {
    const buildAction = 'Build Engine Now';
    vscode.window
      .showErrorMessage(
        `Live Class Diagram: analysis engine not found at ${enginePath}. It needs to be built once with Cargo (requires Rust to be installed).`,
        buildAction
      )
      .then((choice) => {
        if (choice === buildAction) {
          this.buildEngine();
        }
      });
  }

  buildEngine() {
    if (this.building) {
      return;
    }
    this.building = true;

    const task = new vscode.Task(
      { type: 'liveClassDiagramBuildEngine' },
      vscode.TaskScope.Workspace,
      'Build Live Class Diagram Engine',
      'live-class-diagram',
      new vscode.ShellExecution('npm', ['run', 'compile:engine'], {
        cwd: this.context.extensionPath,
      })
    );

    vscode.tasks.executeTask(task).then((execution) => {
      const disposable = vscode.tasks.onDidEndTaskProcess((event) => {
        if (event.execution !== execution) {
          return;
        }
        disposable.dispose();
        this.building = false;

        if (event.exitCode === 0) {
          vscode.window.showInformationMessage('Live Class Diagram: engine built successfully.');
          if (this.root) {
            this.start(this.root);
          }
        } else {
          vscode.window.showErrorMessage(
            'Live Class Diagram: engine build failed. Check the "Build Live Class Diagram Engine" terminal for details.'
          );
        }
      });
    });
  }

  reportSpawnFailure(err) {
    vscode.window.showErrorMessage(
      `Live Class Diagram: failed to start the analysis engine (${err.message}). ` +
        'Set "liveClassDiagram.enginePath" in settings if the bundled binary is missing or in the wrong place.'
    );
  }

  restart() {
    const root = this.root;
    this.stop();
    if (root) {
      this.start(root);
    }
  }

  stop() {
    if (this.child) {
      this.child.kill();
      this.child = null;
    }
    this.root = null;
  }

  dispose() {
    this.stop();
  }
}

module.exports = { EngineProcess };
