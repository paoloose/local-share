//! paoloose share: a shared bucket for dropping files.
//!
//! Crate layout:
//! - `shared`: isomorphic types and naming rules (used by server and UI)
//! - `storage`: filesystem bucket store (server only)
//! - `server`: Leptos server functions, blob endpoints, hot-reload events
//! - `app`: the UI (components, modals, state)
//! - `shell`: SSR document shell

pub mod app;
pub mod error_template;
pub mod server;
pub mod shared;
pub mod shell;

#[cfg(feature = "ssr")]
pub mod storage;

pub use app::App;
pub use shell::shell;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
