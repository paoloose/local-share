//! Hot reload plumbing: an in-process event bus plus a filesystem watcher, and
//! the SSE endpoint that streams file changes to the browser.

use crate::server::services::services;
use crate::shared::naming::BucketName;
use crate::shared::types::{FileEvent, FileEventKind};
use crate::storage::BucketStore;
use axum::extract::Query;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use notify::Watcher;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use tokio::sync::broadcast;

/// Fan-out bus for `FileEvent`s. Every mutation (and every out-of-band disk
/// change seen by the watcher) is published here.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<FileEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: FileEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<FileEvent> {
        self.tx.subscribe()
    }
}

/// Watches the buckets directory so files added, changed or removed directly
/// on disk (e.g. `cp` into a bucket) also hot reload open clients.
pub fn spawn_fs_watcher(
    root: PathBuf,
    bus: EventBus,
) -> notify::Result<notify::RecommendedWatcher> {
    // notify reports absolute paths, so anchor everything on the canonical
    // root or `strip_prefix` would drop every event.
    let root = root.canonicalize()?;
    let watch_root = root.clone();
    let mut watcher = notify::recommended_watcher(
        move |result: Result<notify::Event, notify::Error>| {
            let Ok(event) = result else { return };
            for path in &event.paths {
                if let Some(file_event) = translate(&watch_root, path, &event.kind) {
                    bus.publish(file_event);
                }
            }
        },
    )?;
    watcher.watch(&root, notify::RecursiveMode::Recursive)?;
    Ok(watcher)
}

fn translate(
    root: &std::path::Path,
    path: &std::path::Path,
    kind: &notify::EventKind,
) -> Option<FileEvent> {
    // Reads are not mutations: inotify reports them as Access events, and
    // mapping those to updates would make every refetch trigger the next one
    // (a self-sustaining loop through the watcher).
    if matches!(kind, notify::EventKind::Access(_)) {
        return None;
    }
    let relative = path.strip_prefix(root).ok()?;
    let mut components = relative.components();
    let bucket = BucketName::parse(&components.next()?.as_os_str().to_string_lossy()).ok()?;
    let rest: Vec<String> = components
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect();
    if rest.is_empty() {
        return None;
    }

    // Manifest writes ride along with every mutation and also cover hand
    // edits and other app instances touching the same bucket.
    if rest.iter().any(|part| part.ends_with(".tmp")) {
        return None;
    }
    if rest.len() == 1 && rest[0] == "meta.json" {
        return Some(FileEvent {
            bucket: bucket.to_string(),
            id: String::new(),
            name: "meta.json".to_string(),
            kind: FileEventKind::Changed,
        });
    }

    // Blob files: buckets/<bucket>/files/<name>. UUID names are app-managed
    // (the app publishes precise events itself); anything else was copied in
    // by hand and gets registered so it shows up like any dropped file.
    if rest.len() == 2 && rest[0] == "files" {
        let name = rest[1].clone();
        if uuid::Uuid::parse_str(&name).is_ok() {
            return None;
        }
        match kind {
            notify::EventKind::Create(_) => {
                let _ = services().store.register_external(&bucket, &name);
            }
            notify::EventKind::Remove(_) => {
                let _ = services().store.delete(&bucket, &name);
            }
            _ => {}
        }
        let kind = match kind {
            notify::EventKind::Create(_) => FileEventKind::Added,
            notify::EventKind::Remove(_) => FileEventKind::Removed,
            notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
                FileEventKind::Renamed
            }
            _ => FileEventKind::Changed,
        };
        return Some(FileEvent {
            bucket: bucket.to_string(),
            id: name.clone(),
            name,
            kind,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, AccessMode, DataChange, ModifyKind};
    use std::path::Path;

    #[test]
    fn reads_never_emit_events() {
        let root = Path::new("/srv/buckets");
        let manifest = root.join("andes/meta.json");
        let blob = root.join("andes/files/report.pdf");

        for path in [&manifest, &blob] {
            assert!(translate(root, path, &notify::EventKind::Access(AccessKind::Read)).is_none());
            assert!(
                translate(root, path, &notify::EventKind::Access(AccessKind::Close(AccessMode::Read)))
                    .is_none()
            );
        }
    }

    #[test]
    fn manifest_writes_emit_changed_events() {
        let root = Path::new("/srv/buckets");
        let manifest = root.join("andes/meta.json");

        let event = translate(
            root,
            &manifest,
            &notify::EventKind::Modify(ModifyKind::Data(DataChange::Content)),
        )
        .expect("manifest writes must be reported");
        assert_eq!(event.bucket, "andes");
        assert_eq!(event.kind, FileEventKind::Changed);

        assert!(
            translate(root, &manifest, &notify::EventKind::Modify(ModifyKind::Any)).is_some(),
            "hand edits should still be reported"
        );
    }
}

/// `GET /api/events?bucket=<name>`: server-sent events for one bucket.
pub async fn sse(
    Query(params): Query<HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let bucket = params.get("bucket").cloned().unwrap_or_default();
    let rx = services().events.subscribe();

    let stream = futures::stream::unfold(rx, move |mut rx| {
        let bucket = bucket.clone();
        async move {
            loop {
                match rx.recv().await {
                    Ok(event) if event.bucket == bucket => {
                        let payload = serde_json::to_string(&event).unwrap_or_default();
                        return Some((Ok(Event::default().data(payload)), rx));
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
