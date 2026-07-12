# Live Class Diagram

Live, automatically updating UML class diagrams for your project, right
inside VS Code. Edit a file or add a new class and the diagram updates
without you doing anything.

A Rust engine (`../rust-engine`) parses your source files and watches them
for changes; this extension renders the result as an interactive diagram in
a VS Code webview.

## Install

Search for **"Live Class Diagram"** in the Extensions view, or from the
command line:

```sh
code --install-extension kavinthaoshada.live-class-diagram
```

## Usage

- Run **"Live Class Diagram: Open Diagram"** from the command palette, or
- Click **`ClassDiagram.liveclass`** in your workspace root (created
  automatically) — it opens the diagram through a custom editor.

Supported languages: TypeScript, JavaScript (including React/Next.js
`.tsx`/`.jsx`), Python, Java, C#, and PHP (including Laravel-style classes
and traits).

## Features

- **Live updates** — the diagram redraws automatically as you edit, add, or
  remove source files. No manual refresh needed.
- **Full UML notation** — inheritance, interface implementation,
  composition, aggregation, and dependency are rendered with their proper
  UML arrow/diamond conventions, not generic lines.
- **Focus mode** — double-click any class to isolate it and everything
  directly connected to it, so a busy diagram doesn't get in the way when
  you only care about one part of it. Press **Esc** or click **"Show All"**
  to return to the full diagram.
- **Group by folder** — the **Group** toggle clusters classes into
  UML package boxes based on which folder they live in, which helps once a
  project has too many classes to make sense of as one flat graph.
- **Export** — **Export PNG** / **Export SVG** save exactly what's currently
  on screen (the full diagram, a grouped view, or a focused single class) to
  a file via a native save dialog.
- **Search** — type in the search box to highlight classes by name, field,
  or method name and dim the rest.
- **Jump to source** — **Ctrl/Cmd+click** a class, or an individual field or
  method, to open that exact declaration in the editor.
- **Zoom** — from 0.05x (zoom out far enough to see a very large diagram at
  once) up to 24x (zoom in far enough to read one box clearly).
- Your Group/Focus view is remembered if you close and reopen the diagram
  panel or reload the window.
- **Laravel/Eloquent aware** — `hasMany`/`belongsTo`/`belongsToMany`/
  `hasOne`/`morphMany`/`morphOne`/`morphTo` relationship accessors on
  Eloquent models are recognized and drawn as real relationships, even
  though they aren't type-hinted in the PHP itself.

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
