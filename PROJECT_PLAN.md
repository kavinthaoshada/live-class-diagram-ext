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
   #[cfg(test)] mod tests — 51 unit tests total, run with `cargo test`)

live-class-diagram/      VS Code extension (Node/JS host + webview UI)
  extension.js            activation, command + custom editor registration
  src/engineProcess.js    spawns rust-engine, parses NDJSON, emits `diagram` events
  src/diagramPanel.js     webview panel manager (HTML, messaging, caching latest model)
  src/customEditor.js     custom editor provider for the *.liveclass file
  src/placeholder.js      creates ClassDiagram.liveclass in the workspace root
  webview-src/            layout (dagre) + SVG UML renderer + pan/zoom + export (esbuild-bundled to media/)
    export.js             inlines computed styles + resets pan/zoom transform, serializes to SVG or rasterizes to PNG

.github/workflows/
  ci.yml                  cargo test + cargo build on 3 OSes, npm lint + webview build, on every push/PR
  release.yml             cross-platform matrix build of rust-engine + per-target `vsce package`,
                           triggered on `v*` tags or manual dispatch; attaches .vsix files to a
                           GitHub Release and (opt-in, manual-dispatch only) publishes to the Marketplace
```

Data flow: file change -> `notify` (Rust) -> debounce -> rescan -> JSON on
stdout -> extension host reads NDJSON -> `postMessage` to webview -> dagre
layout -> SVG re-render.

## How to open the diagram

- Command palette: "Live Class Diagram: Open Diagram", or
- Click `ClassDiagram.liveclass` in the workspace root (auto-created on
  activation) — it opens through a registered custom editor.
- Or install from the Marketplace: `kavinthaoshada.live-class-diagram`.

Once open: double-click any class to isolate it and its direct relationships
(Escape or "Show All" to return); Ctrl/Cmd+click a class, or an individual
field or method, to jump to that exact line in the editor; use the "Group"
toggle to cluster classes into folder-based UML package boxes; type in the
search box (top-left) to highlight classes by name/field/method and dim the
rest; "Export PNG"/"Export SVG" save whatever the current view is (full
diagram, grouped, or a focused single class) to a file; `+`/`-`/"Reset"
control zoom (down to 0.05x, up to 24x).

## Release process

```sh
cd live-class-diagram
# commit all feature/fix changes first — npm version needs a clean tree
npm run release:patch   # or release:minor / release:major
git push --follow-tags
```

`npm version patch` bumps `package.json`'s version, commits that change, and
creates a `vX.Y.Z` git tag in one step — this is the whole "policy": there's
no separate version-vs-tag bookkeeping to get wrong, because npm's own
tagging convention (`v` prefix) already matches what `release.yml` expects.
Pushing the tag triggers `release.yml`'s 5-platform build; from there,
either publish manually per platform (see the Marketplace publishing
instructions from an earlier session) or use the workflow's manual dispatch
with "publish" ticked.

Remember to update `CHANGELOG.md` as part of the commit(s) before running
`npm version` — it doesn't do that for you.

## Testing

- `cd rust-engine && cargo test` — 51 unit tests across the five language
  parsers, relationship inference, and the scanner.
- `cd live-class-diagram && npm run lint` — ESLint over the extension host
  and webview source.
- No automated test drives the actual VS Code extension host or webview yet
  (would need `@vscode/test-electron`) — nothing runs in CI for this layer.
  Webview behavior has instead been checked manually with a throwaway local
  HTML harness serving the real built `media/webview.js` in a real browser,
  sometimes with a `<meta>` CSP tag deliberately matching the real webview's
  CSP string exactly (this is what caught the PNG export bug — the first
  version of this harness had no CSP at all and missed it entirely). Every
  claim below of something being "verified" or "caught" refers to this kind
  of manual, scripted-in-the-moment check, not a regression suite that runs
  again automatically — see the walkthroughs described throughout
  "Completed" below, or the eyeballed browser
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
- [x] **Published to the VS Code Marketplace**: `kavinthaoshada.live-class-diagram`
      v0.0.2 is live, built by `release.yml`'s 5-way matrix and published via
      the opt-in `publish-marketplace` job. Getting there surfaced three real
      GitHub Actions issues, each fixed in the workflow rather than worked
      around by hand each time:
      - `macos-13` runners were fully retired by GitHub in December 2025;
        switched the darwin-x64 job to `macos-15-intel`
      - the default `GITHUB_TOKEN` is read-only on newer repos, so creating
        a GitHub Release needs an explicit `permissions: contents: write`
        on that job
      - the release-attachment job was gated to `refs/tags/v*` only, so a
        manual re-run from a branch (needed to pick up the two fixes above
        without re-tagging) silently skipped it; broadened the condition and
        added a fallback that derives the release tag from
        `package.json`'s version when there's no real tag in `github.ref`
- [x] Extension icon: `package.json` now has `"icon":
      "media/class-diagram-generator.png"` — the image was already sitting
      in the right place (`media/` is already packaged, same as
      `webview.js`/`webview.css`), so no file move was needed
- [x] Export current view as PNG or SVG: two new toolbar buttons export
      exactly whatever is currently on screen — the full diagram, a
      folder-grouped layout, or a single focused class with its direct
      relationships (reusing the existing focus-mode "select one class"
      feature rather than building a separate multi-select UI for it).
      `vscode.window.showSaveDialog` + `vscode.workspace.fs.writeFile` on
      the extension-host side, so it's a native save dialog, not a browser
      download. Caught and fixed a real bug while verifying: `svg-pan-zoom`
      rewrites the live SVG's `viewBox` (and wraps content in a
      `.svg-pan-zoom_viewport` `<g>` with a transform matrix) to reflect
      whatever the user last panned/zoomed to — exporting after zooming in
      was silently exporting the zoomed viewport's dimensions and a
      leftover pan/zoom transform instead of the full diagram. Fixed by
      stashing the diagram's true size in `data-content-*` attributes at
      render time and resetting the pan/zoom transform on the exported
      clone. Also inlines every element's resolved computed style (colors
      etc. come from `var(--vscode-*)`, which don't resolve outside the
      webview's own page) so the exported file looks right when opened
      completely outside VS Code, not just when viewed inside it
- [x] Fixed "Export PNG" doing nothing (reported after the export feature
      shipped, while "Export SVG" worked fine — that asymmetry was the key
      clue). Root cause: the webview's CSP is `img-src ${webview.cspSource}`,
      and PNG rasterization loads the SVG into an `<img>` via a `blob:` URL
      to draw it onto a canvas — `blob:` wasn't in the allowed list, so the
      browser silently blocked the image load. SVG export never touches
      `<img>` at all (pure text serialization), which is exactly why only
      PNG broke. Fixed by adding `blob:` to `img-src`. This is also what
      prompted rebuilding the verification method: the previous round's
      browser check used a test page with **no CSP at all**, so it never
      could have caught this. Re-verified with a harness whose `<meta>` CSP
      tag matches the shipped one exactly — first confirmed the bug
      reproduces without `blob:`, then confirmed the fix resolves it with
      `blob:` added, rather than trusting a fix that "look right" in code
- [x] Export failures now show a `vscode.window.showErrorMessage` instead of
      failing silently — a direct response to the PNG bug above having no
      visible symptom at all beyond "nothing happens"
- [x] Group-by-folder and focus mode now persist across the webview being
      disposed and recreated (closing/reopening the panel, reloading the
      window) via `vscode.getState()`/`setState()`. Verified with a
      localStorage-backed stand-in for VS Code's real state persistence
      (survives an actual page navigation, unlike an in-memory JS variable)
      — toggled Group on, reloaded, confirmed the grouped layout reappeared
      without re-clicking; separately did the same for a focused class
- [x] `README.md` rewritten to actually describe Focus mode, Group-by-folder,
      Export, and the wider zoom range, plus an `Install` section and the
      full 5-language list — this is what the Marketplace listing displays,
      and it previously only described the original TS/JS-only MVP
- [x] Filter/search classes: a search box highlights matching classes (blue
      outline, same visual language as focus mode) and dims the rest,
      matching against class name, field names, and method names — not
      layout-destructive, so it works uniformly across the full diagram,
      grouped view, or a focused class without needing special-case
      handling for each mode. Shows a live "N of M classes match" count in
      place of the usual hint text. Verified in-browser: text matches
      surface via field names too (e.g. searching "user" correctly matched
      `AuthService` through its `userService` field, not just classes
      literally named User*), and the highlight correctly re-applies after
      toggling Group (since that fully rebuilds the SVG DOM)
- [x] Version-bump/release policy: documented (see "Release process" above)
      rather than built as new tooling — `npm version patch/minor/major`
      already bumps `package.json`, commits, and creates the matching
      `vX.Y.Z` git tag in one step, which is exactly the tag shape
      `release.yml` expects. Added `release:patch`/`release:minor`/
      `release:major` npm scripts as a discoverable alias for the same
      command rather than writing a custom bump script
- [x] New, simplified extension icon (user-provided) — the "zzeylon"
      wordmark mismatch from the original is gone, replaced with bold "LCD"
      lettering; still square (752x752), no `package.json` change needed
      since the filename didn't change
- [x] Laravel/Eloquent-aware relationships: `hasMany`/`belongsToMany`/
      `morphMany`/`morphToMany` (collection-shaped) and `belongsTo`/
      `hasOne`/`morphOne`/`morphTo` (single-shaped) relationship accessors
      are now recognized on PHP models. These aren't type-hinted in
      idiomatic Eloquent code (`return $this->hasMany(Post::class);` has no
      declared return type), so they were previously invisible to the
      existing type-hint-based relationship inference entirely. Implemented
      by recursively scanning each method's body for a `$this->` call to a
      known relation name, extracting the related class from its
      `X::class` argument, and synthesizing a field named after the
      accessor method itself (`posts`, matching how Eloquent callers
      actually use it: `$user->posts`) with a `[]`-suffixed or bare type —
      which means the *existing* composition/aggregation inference in
      `relationships.rs` picks these up automatically, with zero changes
      needed there. Confirmed the real tree-sitter-php shape
      (`member_call_expression`, `class_constant_access_expression`) before
      writing the extraction, same discipline as every other language
      parser. 4 new tests, including one exercising the full pipeline
      through to an actual rendered relationship edge
- [x] Ctrl/Cmd+click a class to jump to its declaration in the editor.
      Deliberately scoped to class-level only for now (uses the `file`/
      `line` every `ClassNode` already carries — no Rust engine changes
      needed at all); field/method-level jumps would need each parser to
      also track a line number per member, which none of them do yet.
      Chose a modifier-click over plain click specifically so it can never
      collide with the existing double-click-for-focus-mode gesture — a
      double-click fires two plain `click` events first, so if navigation
      triggered on any unmodified click it would fire from the first click
      before focus mode ever got the second. Verified all three cases in
      isolation: plain click does nothing (as before), double-click still
      focuses, Ctrl+click navigates and does not also focus
- [x] Field/method-level "jump to source": Ctrl/Cmd+click now works on an
      individual field or method line, not just the class as a whole.
      `FieldNode`/`MethodNode` gained a `line`, captured the same way
      `ClassNode`'s already was — required touching all 20 construction
      sites across the five language parsers (found every one by just
      building and reading the compiler's "missing field" errors, rather
      than grepping by hand). On the webview side, `classCompartments` now
      carries `{text, line}` per member instead of a plain display string,
      threaded through `layout.js` and `render.js`; a click on a member
      line calls `stopPropagation()` so it doesn't also trigger the
      class-level handler sitting on the same DOM group. Added a real
      regression test that caught an actual mistake: my first line
      assertions were off by one because `parse_php`'s test helper
      prepends a `<?php` line, shifting every subsequent line down —
      fixed the test, not the (correct) implementation. Verified in-browser
      that a field click, a method click, and a plain background click on
      the same box each report a different, correct line

## Todo

### Near-term (engine correctness & coverage)
- [ ] Incremental re-parsing (only re-parse changed files instead of a full
      workspace rescan on every change) for large projects
- [ ] C#'s extends-vs-implements split is a naming-convention heuristic
      (`IFoo` => interface); a base class that doesn't follow the `I`-prefix
      convention when combined with interfaces that also don't follow it
      would be misclassified — fine for idiomatic C#, not bulletproof
- [ ] A generic plugin mechanism so community-contributed language parsers
      don't have to be added to this repo directly

### Packaging & distribution
- [ ] Open VSX publishing (same idea as the Marketplace job, for editors
      like Cursor/VSCodium that use the Open VSX registry instead of the
      Microsoft Marketplace)

### UX polish
- [ ] Diffed re-render (animate node position/opacity changes between
      updates instead of a full redraw) for a smoother "live" feel
- [ ] Manual layout overrides (drag a class box, remember its position)
- [ ] Filter/search classes, collapse fields or methods per box
- [ ] Status bar item showing engine health / last scan time
- [ ] Export currently covers PNG/SVG of the on-screen view; a "copy to
      clipboard" option (skip the save dialog for quick sharing) or
      exporting at a user-chosen resolution/scale would be natural
      follow-ups if PNG/SVG export turns out to get used a lot
- [ ] Cross-group edges in grouped view are drawn as straight lines between
      box edges with no obstacle avoidance, so on a busy diagram a line can
      visually cross through an unrelated box in between; fine for now, but
      real edge routing (or at least routing around group boundaries) would
      read more cleanly on large multi-folder projects
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
