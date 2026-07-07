const { execFileSync } = require('child_process');
const fs = require('fs');
const path = require('path');

const engineDir = path.join(__dirname, '..', '..', 'rust-engine');
const binDir = path.join(__dirname, '..', 'bin');
const exeName = process.platform === 'win32' ? 'rust-engine.exe' : 'rust-engine';

console.log(`Building rust-engine (release) from ${engineDir}...`);
execFileSync('cargo', ['build', '--release'], { cwd: engineDir, stdio: 'inherit' });

fs.mkdirSync(binDir, { recursive: true });
fs.copyFileSync(path.join(engineDir, 'target', 'release', exeName), path.join(binDir, exeName));

console.log(`Engine binary copied to ${path.join(binDir, exeName)}`);
