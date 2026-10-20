pub mod components;
pub mod hooks;
pub mod routes;
pub mod state;

use components::modals;
use leptos::ev;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::{ParamSegment, StaticSegment};
use state::{AppState, Modal, PageState};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let app = AppState::new();
    let _page = PageState::new();

    // Restoring the saved identity and bucket, and the first-visit prompts,
    // run after hydration so the server HTML and first client render match.
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if app.owner.get_untracked().is_empty() {
                if let Some(owner) = state::storage_get(state::OWNER_KEY) {
                    if !owner.trim().is_empty() {
                        app.owner.set(owner.trim().to_string());
                    }
                }
            }
            if app.bucket.get_untracked().is_empty() {
                if let Some(bucket) = state::storage_get(state::BUCKET_KEY) {
                    if !bucket.trim().is_empty() {
                        app.bucket.set(bucket.trim().to_string());
                    }
                }
            }
            if app.owner.get_untracked().is_empty() {
                app.modal.set(Modal::Identity { cancellable: false });
            } else if app.bucket.get_untracked().is_empty() {
                app.modal.set(Modal::Bucket { cancellable: false });
            }
        });
    }

    window_event_listener(ev::keydown, move |ev: web_sys::KeyboardEvent| {
        if ev.key() != "Escape" {
            return;
        }
        match app.modal.get() {
            Modal::Upload | Modal::Rename(_) | Modal::ConfirmDelete(_) => {
                app.modal.set(Modal::None)
            }
            _ => {}
        }
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/paoloose-share.css"/>
        <Title text="paoloose share"/>
        <components::topbar::Topbar/>
        <Router>
            <Routes fallback=|| "Page not found.".into_view()>
                <Route path=StaticSegment("") view=routes::SharePage/>
                <Route path=ParamSegment("bucket") view=routes::SharePage/>
            </Routes>
        </Router>
        <modals::identity::IdentityModal/>
        <modals::bucket::BucketModal/>
        <modals::upload::UploadModal/>
        <modals::rename::RenameModal/>
        <modals::confirm::ConfirmModal/>
        <components::toast::Toast/>
    }
}
