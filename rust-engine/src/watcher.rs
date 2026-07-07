use crate::model::ClassDiagram;
use crate::scanner::RelevanceFilter;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

const DEBOUNCE: Duration = Duration::from_millis(300);

pub fn run<F: Fn(ClassDiagram)>(root: &Path, on_update: F) {
    on_update(crate::analyze_project(root));

    let filter = RelevanceFilter::for_root(root);

    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher: RecommendedWatcher = match notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    }) {
        Ok(watcher) => watcher,
        Err(err) => {
            eprintln!("failed to start file watcher: {err}");
            return;
        }
    };

    if let Err(err) = watcher.watch(root, RecursiveMode::Recursive) {
        eprintln!("failed to watch {}: {err}", root.display());
        return;
    }

    loop {
        let Ok(first) = rx.recv() else {
            break;
        };
        let mut relevant = event_is_relevant(&first, &filter);

        while let Ok(next) = rx.recv_timeout(DEBOUNCE) {
            relevant = relevant || event_is_relevant(&next, &filter);
        }

        if relevant {
            on_update(crate::analyze_project(root));
        }
    }
}

fn event_is_relevant(event: &notify::Result<Event>, filter: &RelevanceFilter) -> bool {
    match event {
        Ok(event) => event.paths.iter().any(|path| filter.is_relevant(path)),
        Err(_) => false,
    }
}
