use crate::shared::types::{FileMeta, SortMode};
use leptos::prelude::*;

pub const OWNER_KEY: &str = "paoloose-share.owner";
pub const BUCKET_KEY: &str = "paoloose-share.bucket";

/// Which modal, if any, sits above the app. The identity and bucket prompts
/// can be mandatory (first visit) or triggered from the topbar chips.
#[derive(Clone, Debug, PartialEq)]
pub enum Modal {
    None,
    Identity { cancellable: bool },
    Bucket { cancellable: bool },
    Upload,
    Rename(FileMeta),
    ConfirmDelete(FileMeta),
}

/// Cross-cutting app state, provided once in `<App/>`.
#[derive(Clone, Copy, Debug)]
pub struct AppState {
    pub owner: RwSignal<String>,
    pub bucket: RwSignal<String>,
    pub modal: RwSignal<Modal>,
    pub toast: RwSignal<String>,
}

impl AppState {
    pub fn new() -> Self {
        let state = Self {
            owner: RwSignal::new(String::new()),
            bucket: RwSignal::new(String::new()),
            modal: RwSignal::new(Modal::None),
            toast: RwSignal::new(String::new()),
        };
        provide_context(state);
        state
    }

    pub fn show_toast(self, message: impl Into<String>) {
        self.toast.set(message.into());
    }

    pub fn clear_toast(self) {
        self.toast.set(String::new());
    }
}

/// Toolbar state shared between the toolbar and the file list.
#[derive(Clone, Copy, Debug)]
pub struct PageState {
    pub query: RwSignal<String>,
    pub sort: RwSignal<SortMode>,
}

impl PageState {
    pub fn new() -> Self {
        let state = Self {
            query: RwSignal::new(String::new()),
            sort: RwSignal::new(SortMode::default()),
        };
        provide_context(state);
        state
    }
}

pub fn use_app_state() -> AppState {
    expect_context::<AppState>()
}

pub fn use_page_state() -> PageState {
    expect_context::<PageState>()
}

#[cfg(target_arch = "wasm32")]
pub fn storage_get(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(key)
        .ok()
        .flatten()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn storage_get(_key: &str) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn storage_set(key: &str, value: &str) {
    if let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok()).flatten()
    {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn storage_set(_key: &str, _value: &str) {}
