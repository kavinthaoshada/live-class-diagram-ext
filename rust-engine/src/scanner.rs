use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    "out",
    ".vscode-test",
    "coverage",
    ".next",
    "vendor",
    "storage",
    "__pycache__",
    "venv",
    ".venv",
    "bin",
    "obj",
];

pub const SUPPORTED_EXTENSIONS: &[&str] =
    &["ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "java", "cs", "php"];

fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

fn has_excluded_component(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        EXCLUDED_DIRS.contains(&name.as_ref())
    })
}

/// Walks the project honoring the user's `.gitignore` (plus global git
/// excludes and `.git/info/exclude`) via the `ignore` crate, on top of a
/// fixed list of directories that are excluded even when a project has no
/// `.gitignore` of its own (e.g. `node_modules`, `vendor`).
pub fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        // A workspace doesn't have to be an actual git repository (or the
        // workspace root doesn't have to be the repo root) for its
        // `.gitignore` to be worth honoring.
        .require_git(false)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|entry| entry.into_path())
        .filter(|path| is_supported_extension(path) && !has_excluded_component(path))
        .collect()
}

/// Reusable per-workspace filter for the file watcher, so every changed path
/// doesn't have to rebuild a `.gitignore` matcher from scratch.
pub struct RelevanceFilter {
    root: PathBuf,
    gitignore: Gitignore,
}

impl RelevanceFilter {
    pub fn for_root(root: &Path) -> Self {
        let mut builder = GitignoreBuilder::new(root);
        builder.add(root.join(".gitignore"));
        let gitignore = builder.build().unwrap_or_else(|_| Gitignore::empty());
        Self { root: root.to_path_buf(), gitignore }
    }

    pub fn is_relevant(&self, path: &Path) -> bool {
        if !is_supported_extension(path) || has_excluded_component(path) {
            return false;
        }
        !self.is_gitignored(path)
    }

    /// A directory-only pattern like `ignored_stuff/` only matches when
    /// asked about a directory, not a file inside it — `WalkBuilder` handles
    /// this by pruning whole subtrees as it walks, but a single file path
    /// checked in isolation has to walk its own ancestors up to the
    /// workspace root to get the same result.
    fn is_gitignored(&self, path: &Path) -> bool {
        if self.gitignore.matched(path, false).is_ignore() {
            return true;
        }
        let mut current = path.parent();
        while let Some(dir) = current {
            if !dir.starts_with(&self.root) || dir == self.root {
                break;
            }
            if self.gitignore.matched(dir, true).is_ignore() {
                return true;
            }
            current = dir.parent();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn collects_supported_extensions_only() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "keep.ts", "export class Kept {}");
        write(dir.path(), "skip.txt", "not source");
        write(dir.path(), "skip.md", "# readme");

        let files = collect_source_files(dir.path());
        let names: Vec<_> = files.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, vec!["keep.ts"]);
    }

    #[test]
    fn excludes_fixed_directory_list_even_without_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/keep.ts", "export class Kept {}");
        write(dir.path(), "node_modules/dep/skip.ts", "export class Skip {}");
        write(dir.path(), "vendor/pkg/skip.php", "<?php class Skip {}");

        let files = collect_source_files(dir.path());
        assert_eq!(files.len(), 1, "expected only src/keep.ts, got {files:?}");
    }

    #[test]
    fn respects_gitignore_without_a_real_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", "ignored_stuff/\n");
        write(dir.path(), "keep.ts", "export class Kept {}");
        write(dir.path(), "ignored_stuff/skip.ts", "export class Skip {}");

        let files = collect_source_files(dir.path());
        let names: Vec<_> = files.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names, vec!["keep.ts"]);
    }

    #[test]
    fn relevance_filter_matches_collect_source_files_behavior() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", "ignored_stuff/\n");
        write(dir.path(), "keep.ts", "export class Kept {}");
        write(dir.path(), "ignored_stuff/skip.ts", "export class Skip {}");
        write(dir.path(), "keep.txt", "not source");

        let filter = RelevanceFilter::for_root(dir.path());
        assert!(filter.is_relevant(&dir.path().join("keep.ts")));
        assert!(!filter.is_relevant(&dir.path().join("ignored_stuff/skip.ts")));
        assert!(!filter.is_relevant(&dir.path().join("keep.txt")));
    }
}
