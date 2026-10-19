use crate::storage::FsStore;
use std::path::PathBuf;
use std::sync::OnceLock;

use super::events::EventBus;

/// Everything the request handlers need: the bucket store and the event bus
/// that drives hot reload. Initialized once at startup, before serving.
pub struct AppServices {
    pub store: FsStore,
    pub events: EventBus,
}

static SERVICES: OnceLock<AppServices> = OnceLock::new();

impl AppServices {
    pub fn init(buckets_root: impl Into<PathBuf>) -> &'static AppServices {
        SERVICES.get_or_init(|| Self {
            store: FsStore::new(buckets_root),
            events: EventBus::new(256),
        })
    }
}

pub fn services() -> &'static AppServices {
    SERVICES.get().expect("AppServices::init() must run before serving requests")
}
