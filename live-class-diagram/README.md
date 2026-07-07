# Live Class Diagram

Live, automatically updating UML class diagrams for your project, right
inside VS Code. Edit a file or add a new class and the diagram updates
without you doing anything.

A Rust engine (`../rust-engine`) parses your source files and watches them
for changes; this extension renders the result as an interactive diagram in
a VS Code webview.

## Usage

- Run **"Live Class Diagram: Open Diagram"** from the command palette, or
- Click **`ClassDiagram.liveclass`** in your workspace root (created
  automatically) — it opens the diagram through a custom editor.

Currently supported languages: TypeScript, JavaScript (including React/Next.js
`.tsx`/`.jsx`), Python, Java, C#, and PHP (including Laravel-style classes and
traits).

## Settings

- `liveClassDiagram.enginePath` — path to the `rust-engine` executable.
  Leave empty to use the binary bundled with the extension.
- `liveClassDiagram.excludeGlobs` — glob patterns excluded from analysis.

## Development

```sh
npm install
npm run setup   # builds ../rust-engine (release) into bin/, then bundles webview-src/ into media/
```

Then press F5 (or use the "Run Extension" launch config) to start an
Extension Development Host.

If the bundled engine binary is ever missing (e.g. you skipped `npm run
setup`), the extension detects this at runtime and offers a **"Build Engine
Now"** action in an error notification, which runs the same build for you
and restarts the analysis automatically.

See `PROJECT_PLAN.md` in the repository root for architecture notes and the
project roadmap.
