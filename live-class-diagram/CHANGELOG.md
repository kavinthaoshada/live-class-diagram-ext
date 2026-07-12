# Change Log

All notable changes to the "live-class-diagram" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [0.0.7]

- Ctrl/Cmd+click now works on individual fields and methods, not just the
  class itself — jumps to that exact line in the editor

## [0.0.6]

- Laravel/Eloquent relationship accessors (`hasMany`, `belongsTo`,
  `belongsToMany`, `hasOne`, `morphMany`, `morphOne`, `morphTo`) are now
  recognized and drawn as real relationships on PHP models
- Ctrl/Cmd+click a class to jump straight to its declaration in the editor
- New, simpler extension icon

## [0.0.5]

- Search box to highlight classes by name, field, or method name, dimming
  the rest — helps navigate diagrams with a lot of classes

## [0.0.4]

- Fixed: "Export PNG" silently did nothing (the webview's Content-Security-
  Policy didn't allow the `blob:` URL used to rasterize the SVG). "Export
  SVG" was unaffected since it doesn't need one.
- The Group toggle and Focus mode are now remembered when you close and
  reopen the diagram panel, or reload the window
- Extension icon added
- Export failures now show a visible error notification instead of failing
  silently

## [0.0.3]

- Export the current view (full diagram, grouped, or a focused class) as
  PNG or SVG via a native save dialog

## [0.0.2]

- Class focus/isolate mode: double-click a class to see it and its direct
  relationships in isolation
- Group classes by their containing folder into UML package boxes
- Wider zoom range (0.05x-24x)
- `.gitignore` is now respected when scanning a workspace
- Python, Java, C#, and PHP (including Laravel-style traits) parsers, in
  addition to TypeScript/JavaScript (including React and Next.js)

## [0.0.1]

- Initial release: live UML class diagram for TypeScript/JavaScript projects,
  rendered in a VS Code webview and kept in sync with a file watcher