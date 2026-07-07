const esbuild = require('esbuild');
const fs = require('fs');
const path = require('path');

const watch = process.argv.includes('--watch');

fs.mkdirSync(path.join(__dirname, 'media'), { recursive: true });
fs.copyFileSync(
  path.join(__dirname, 'webview-src', 'style.css'),
  path.join(__dirname, 'media', 'webview.css')
);

const buildOptions = {
  entryPoints: [path.join(__dirname, 'webview-src', 'main.js')],
  bundle: true,
  outfile: path.join(__dirname, 'media', 'webview.js'),
  format: 'iife',
  target: 'es2020',
  logLevel: 'info',
};

async function run() {
  if (watch) {
    const ctx = await esbuild.context(buildOptions);
    await ctx.watch();
    console.log('watching for webview changes...');
  } else {
    await esbuild.build(buildOptions);
  }
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});
