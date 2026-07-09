# Live Class Diagram — Project Plan

A VS Code extension that generates live, auto-updating UML class diagrams for a
project. A Rust engine parses source files and emits a class/relationship
model as JSON; the extension renders it as an interactive diagram in a
VS Code webview, and keeps it in sync as files are edited or added.

## Architecture

```
rust-engine/            headless analysis engine (no GUI)
  src/model.rs           diagram data model (classes, fields, methods, relationships)
  src/scanner.rs          workspace file discovery, exclusion rules
  src/languages/          per-language parsers (tree-sitter based)
    ecma.rs               TypeScript / JavaScript / JSX / TSX (covers React, Next.js)
    python.rs             Python
    java.rs               Java
    csharp.rs             C#
    php.rs                PHP (covers Laravel-style classes, interfaces, traits)
  src/relationships.rs    cross-file relationship inference
  src/watcher.rs          debounced file-system watcher -> rescans -> NDJSON on stdout
  src/main.rs             CLI: `analyze <root>` (one-shot) / `watch <root>` (continuous)
  (each src/languages/*.rs and relationships.rs/scanner.rs has its own
   #[cfg(test)] mod tests — 41 unit tests total, run with `cargo test`)

live-class-diagram/      VS Code extension (Node/JS host + webview UI)
  extension.js            activation, command + custom editor registration
  src/engineProcess.js    spawns rust-engine, parses NDJSON, emits `diagram` events
  src/diagramPanel.js     webview panel manager (HTML, messaging, caching latest model)
  src/customEditor.js     custom editor provider for the *.liveclass file
  src/placeholder.js      creates ClassDiagram.liveclass in the workspace root
  webview-src/            layout (dagre) + SVG UML renderer + pan/zoom (esbuild-bundled to media/)

.github/workflows/
  ci.yml                  cargo test + cargo build on 3 OSes, npm lint + webview build, on every push/PR
  release.yml             cross-platform matrix build of rust-engine + per-target `vsce package`,
                           triggered on `v*` tags, attaches .vsix files to a GitHub Release
```

Data flow: file change -> `notify` (Rust) -> debounce -> rescan -> JSON on
stdout -> extension host reads NDJSON -> `postMessage` to webview -> dagre
layout -> SVG re-render.

## How to open the diagram

- Command palette: "Live Class Diagram: Open Diagram", or
- Click `ClassDiagram.liveclass` in the workspace root (auto-created on
  activation) — it opens through a registered custom editor.

Once open: double-click any class to isolate it and its direct relationships
(Escape or "Show All" to return); use the "Group" toggle to cluster classes
into folder-based UML package boxes; `+`/`-`/"Reset" control zoom (now down
to 0.05x and up to 24x).

## Testing

- `cd rust-engine && cargo test` — 41 unit tests across the five language
  parsers, relationship inference, and the scanner.
- `cd live-class-diagram && npm run lint` — ESLint over the extension host
  and webview source.
- No automated test drives the actual VS Code extension host or webview yet
  (would need `@vscode/test-electron`); everything above the Rust/JSON
  boundary has so far only been verified by hand (see the walkthroughs
  described throughout "Completed" below) or the eyeballed browser
  screenshots taken during development.

## Completed

- [x] Rust engine CLI (`analyze`, `watch`) with clap
- [x] Diagram data model (classes, interfaces, enums, abstract classes,
      fields, methods, params, visibility, static/abstract flags)
- [x] Workspace scanner with sensible exclusions (node_modules, target, .git,
      dist, build, out, coverage, vendor, hidden dirs)
- [x] TypeScript/JavaScript parser (tree-sitter): classes, interfaces, enums,
      inheritance (`extends`), interface implementation (`implements`),
      constructor parameter properties, static/abstract/visibility modifiers.
      Covers React (`.jsx`/`.tsx`) and Next.js class-based code out of the box
      — no separate framework parser is needed, and `.next` build output is
      already excluded from scanning
- [x] Python parser (tree-sitter-python): classes, inheritance, `ABC`-based
      abstract classes, `Enum`/`IntEnum`/`Flag` base classes rendered as enum
      diagrams, class-level typed attributes vs. bare type-hint annotations
      (only attributes with an assigned value count as "static"),
      `self.x = ...` assignments inside any method treated as instance
      fields, `@staticmethod`/`@classmethod`/`@abstractmethod` decorators
- [x] Java parser (tree-sitter-java): classes, interfaces, enums, `extends`/
      `implements`, interface-extending-interfaces, fields, methods,
      constructors, visibility/static/abstract modifiers
- [x] C# parser (tree-sitter-c-sharp): classes, interfaces, enums, `: Base,
      IInterface` base lists split into extends/implements via the `I`+
      capital naming convention (C#'s grammar doesn't otherwise distinguish
      a base class from implemented interfaces), auto-properties treated as
      fields, visibility/static/abstract modifiers
- [x] PHP parser (tree-sitter-php): classes, interfaces, traits (new `Trait`
      diagram kind, rendered with a `«trait»` stereotype), enums (PHP 8.1+),
      `extends`/`implements`, Laravel-style `use SomeTrait;` mixins rendered
      as a realization edge to the trait, visibility/static/abstract
      modifiers. The scanner already excludes `vendor/` (Composer) the same
      way it excludes `node_modules/` and `.next/`
- [x] Relationship inference: inheritance, implementation, composition
      (single-value field of a known type), aggregation (array/collection
      field of a known type), dependency (method params/return types not
      already held as a field). Language-agnostic — it only looks at type
      name strings, so it worked unchanged for all five languages
- [x] Debounced file watcher (`notify`), filtered to relevant source files
      only, emitting NDJSON on each change
- [x] VS Code extension scaffolding: commands, configuration
      (`liveClassDiagram.enginePath`, `liveClassDiagram.excludeGlobs`)
- [x] Engine process manager (spawn, NDJSON parsing, restart, error reporting)
- [x] Webview panel manager + custom editor provider sharing one renderer
- [x] SVG UML renderer: three-compartment class boxes, stereotypes
      (`«interface»`, `«enumeration»`, `«abstract»`), proper UML edge styles
      (hollow-triangle inheritance/implementation, filled/hollow diamond
      composition/aggregation, open-arrow dependency/association, dashed
      lines for implementation/dependency), dagre auto-layout, pan/zoom,
      hover-to-highlight connected classes, theme-aware colors (VS Code CSS
      variables)
- [x] Auto-created `ClassDiagram.liveclass` placeholder file in the
      workspace root
- [x] End-to-end smoke test: `analyze` output verified against a hand-written
      sample project; `watch` verified to emit an updated model after a live
      file edit; webview rendering verified in a real browser against the
      built bundle
- [x] One-command local setup (`npm run setup`) and an in-editor "Build
      Engine Now" recovery action if the bundled binary is missing, so local
      testing doesn't require manually remembering two build steps
- [x] Five-language smoke test: a combined workspace with one sample file per
      language (TS, Python, Java, C#, PHP) analyzed in a single `analyze` run,
      output checked field-by-field for each parser
- [x] Scanner now respects the workspace's actual `.gitignore` (plus global
      git excludes and `.git/info/exclude`) via the `ignore` crate, replacing
      the plain `walkdir` traversal, on top of the fixed exclusion list.
      Requires `require_git(false)` so it works even when the workspace
      isn't itself a git repository — verified with and without a `.git`
      folder present
- [x] Wider zoom range (0.05x–24x, was 0.2x–6x) so large diagrams can be
      zoomed out far enough to get an overview or in far enough to read a
      single box clearly
- [x] Class focus/isolate mode: double-clicking a class re-lays-out and
      shows only that class plus everything directly connected to it (one
      hop), with the focused box outlined; "Show All" button and Escape key
      exit; live file-change updates keep recomputing the focused subgraph
      instead of silently dropping out of focus (falls back to the full
      diagram only if the focused class itself was deleted)
- [x] Group-by-folder clustering: a toggle that lays out each folder's
      classes as an independent compact subgraph, packs the groups into
      rows as UML package boxes (folder-tab rectangles), and draws
      cross-group relationships as straight box-to-box edges since they
      were never part of the same dagre graph. Grouping is automatically
      suspended (not cleared) while in focus mode and resumes on exit
- [x] Visual verification of all three (grouping, focus mode, zoom) in a
      real browser against a 9-class, 3-folder, 12-relationship sample
      workspace, screenshotted at each state
- [x] Rust unit test suite: 41 tests across all five language parsers,
      relationship inference, and the scanner (`cargo test`). Writing these
      surfaced two real, previously-undetected bugs (not just theoretical
      coverage):
      - the TS/JS parser used the wrong tree-sitter field name for
        JavaScript `#private` fields (`name` vs. `property`), so private
        fields were silently extracted with an empty name and public
        visibility
      - `RelevanceFilter` (used by the file watcher to decide whether a
        changed path matters) only checked the changed path itself against
        `.gitignore`, but directory-only patterns like `ignored_stuff/`
        only match when tested as a directory — so a live edit inside an
        ignored folder was still triggering a rescan. Fixed by walking
        ancestor directories up to the workspace root, matching how
        `WalkBuilder` prunes whole subtrees during the initial scan
- [x] CI workflow (`.github/workflows/ci.yml`): runs `cargo test` and
      `cargo build --release` on Ubuntu/Windows/macOS runners, plus
      `npm run lint` and `npm run compile:webview` for the extension, on
      every push and pull request. **Verified for real**: the project is now
      pushed to `github.com/kavinthaoshada/live-class-diagram-ext`, and all
      4 jobs (rust on 3 OSes + the extension job) completed successfully on
      GitHub's actual runners — confirmed via the GitHub API, not just by
      reading the YAML
- [x] Release workflow (`.github/workflows/release.yml`) exists: a 5-way
      matrix (win32-x64, darwin-x64, darwin-arm64, linux-x64, linux-arm64)
      that builds the Rust engine natively per target (with an aarch64
      Linux cross-linker step on the x86_64 Ubuntu runner), bundles the
      webview, and runs `vsce package --target <target>` so each platform
      gets its own small `.vsix` with only its own native binary. Triggered
      on `v*` tags, attaches all `.vsix` files to a GitHub Release.
      **Still unverified**: no tag has been pushed yet, so this workflow
      itself has never actually run — only `ci.yml` has real evidence
      behind it so far. Push a `v0.0.1`-style tag to find out if the
      aarch64 cross-compilation step needs adjusting
- [x] Generic type parameters in relationship inference, double-checked:
      turns out this already worked from the very first implementation —
      `extract_referenced_types` tokenizes on every non-alphanumeric
      character, so `Repository<User>` was already being split into
      `Repository` and `User` and each checked independently against known
      class names. Added two tests (`generic_field_type_links_to_inner_type_
      not_just_wrapper`, `..._to_both_wrapper_and_inner_type_when_both_known`)
      to lock this in rather than re-implementing something that already
      worked
- [x] PHP constructor property promotion (`function __construct(private
      string $name)`) now becomes a class field, mirroring the existing
      TypeScript parameter-property handling — confirmed the tree-sitter-php
      node kind (`property_promotion_parameter`) by dumping the real parse
      tree first, same discipline as every other language parser
- [x] Publishing prep: added a root `LICENSE` (MIT) plus a copy inside
      `live-class-diagram/` (vsce looks for it there specifically), added
      `repository`/`bugs`/`homepage` fields to `package.json` pointing at
      the real GitHub repo, and wrote real `CHANGELOG.md` entries for 0.0.1
      and 0.0.2. Repackaging now produces zero `vsce package` warnings
      (previously warned about a missing `repository` field and missing
      `LICENSE`)

## Todo

### Near-term (engine correctness & coverage)
- [ ] Incremental re-parsing (only re-parse changed files instead of a full
      workspace rescan on every change) for large projects
- [ ] Laravel/Eloquent-aware relationships: `hasMany`/`belongsTo`/
      `belongsToMany` method calls in a model imply a real association to
      the related model, but today they only show up as a generic
      dependency if the related class is referenced in a type hint at all
- [ ] C#'s extends-vs-implements split is a naming-convention heuristic
      (`IFoo` => interface); a base class that doesn't follow the `I`-prefix
      convention when combined with interfaces that also don't follow it
      would be misclassified — fine for idiomatic C#, not bulletproof
- [ ] A generic plugin mechanism so community-contributed language parsers
      don't have to be added to this repo directly

### Packaging & distribution
- [ ] Push a `v0.0.1`-style tag to actually exercise `release.yml` for the
      first time and confirm the 5-way cross-platform matrix works,
      especially the aarch64 Linux cross-compilation step
- [ ] A version bump policy tied to tags, so the release workflow's
      tag-triggered packaging has a clear source of truth for what version
      number to use (currently manual: `package.json` version is bumped by
      hand)
- [ ] Publish to the VS Code Marketplace (manual steps documented for the
      user — see the publishing instructions given alongside this update;
      requires the user's own Azure DevOps publisher account and PAT, which
      isn't something that can be created on their behalf)
- [ ] Automate marketplace publishing from `release.yml` itself
      (`vsce publish` with a `VSCE_PAT` repo secret) once the first manual
      publish has been done at least once
- [ ] Open VSX publishing (same idea, for editors like Cursor/VSCodium that
      use the Open VSX registry instead of the Microsoft Marketplace)
- [ ] A real icon (128x128 PNG) — `package.json` has no `icon` field yet, so
      the Marketplace listing will show a generic default icon until one is
      added

### UX polish
- [ ] Diffed re-render (animate node position/opacity changes between
      updates instead of a full redraw) for a smoother "live" feel
- [ ] Manual layout overrides (drag a class box, remember its position)
- [ ] Filter/search classes, collapse fields or methods per box
- [ ] Export diagram as SVG/PNG
- [ ] Status bar item showing engine health / last scan time
- [ ] Cross-group edges in grouped view are drawn as straight lines between
      box edges with no obstacle avoidance, so on a busy diagram a line can
      visually cross through an unrelated box in between; fine for now, but
      real edge routing (or at least routing around group boundaries) would
      read more cleanly on large multi-folder projects
- [ ] Group-by-folder and focus mode both reset on webview reload (closing
      and reopening the panel, or reloading VS Code) — neither preference is
      persisted via `webview.getState`/`setState` yet
- [ ] Focus mode only goes one hop out; an option for two hops, or a
      breadcrumb to click through a chain of focused classes, would help for
      classes whose most useful context is a neighbor-of-a-neighbor
- [ ] Grouping is currently folder-only; grouping by namespace/package
      (C#/Java/PHP) or by relationship type could be added as alternate
      modes once folder-based grouping has been used enough to know if it's
      the right default

### Future feature: Presentation Mode
- [ ] Animated walkthrough of the class diagram: reveal classes and
      relationships incrementally to narrate how the codebase is structured
      and how classes interact
- [ ] Step-by-step scripted camera (pan/zoom) driven by either a generated
      or user-authored script
- [ ] Likely a separate webview mode reusing the same layout/render engine

## Key technologies & concepts

- **Rust**: `tree-sitter` with per-language grammars (`tree-sitter-typescript`,
  `tree-sitter-javascript`, `tree-sitter-python`, `tree-sitter-java`,
  `tree-sitter-c-sharp`, `tree-sitter-php`) for real parsing (not regex) of
  class/interface/enum/trait declarations; `notify` for cross-platform
  file-system watching; `clap` for the CLI; `serde`/`serde_json` for the JSON
  model emitted to stdout; the `ignore` crate (the same crate ripgrep uses)
  for workspace traversal that honors `.gitignore`. Each language
  module was written by first dumping the real tree-sitter parse tree for a
  small sample (field names and node kinds are not consistent across
  grammars) and only then writing the extraction code against the confirmed
  shape, rather than guessing from familiarity with the grammar.
- **Process model**: the engine is headless and stdout-driven (NDJSON), so
  the extension treats it as a long-running child process rather than a
  library — keeps the Rust/Node boundary simple (no native Node addon/FFI).
- **VS Code extension APIs**: `registerCustomEditorProvider` (to make a
  `*.liveclass` file open the diagram when clicked in the Explorer),
  `createWebviewPanel`, webview `postMessage`/`onDidReceiveMessage`,
  workspace configuration contribution.
- **Webview rendering**: `dagre` for automatic hierarchical graph layout,
  hand-rolled SVG generation for authentic UML notation (compartments,
  stereotypes, relationship arrow/diamond conventions), `svg-pan-zoom` for
  interaction, VS Code theme CSS variables for automatic light/dark theming.
- **Build tooling**: `esbuild` bundles the webview's ES modules into a single
  script VS Code's webview CSP can load; a small Node script builds the Rust
  engine in release mode and copies the binary into the extension's `bin/`
  for local development (to be replaced by CI-built binaries for
  distribution).
